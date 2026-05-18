//! WASM module instance management.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use wasmtime::{Engine, Instance, Module, Store};

use blvm_node::module::ipc::protocol::{CliSpec, InvocationResultPayload};
use blvm_node::module::traits::ModuleError;

use super::host::{create_host_imports, WasmHostContext, WasmStorage};
use super::limits;

/// Fixed offsets in linear memory for host→module string passing.
/// Uses first page; modules typically leave high addresses free.
const BUF_A_OFFSET: usize = 0x1000;
const BUF_B_OFFSET: usize = 0x2000;
const BUF_SIZE: usize = 0x1000;

/// A loaded WASM module instance. Uses Mutex for thread-safe mutable access when behind Arc.
pub struct WasmModuleInstance {
    _engine: Engine,
    store: Mutex<Store<WasmHostContext>>,
    instance: Instance,
}

fn reset_guest_fuel(store: &mut Store<WasmHostContext>) -> wasmtime::Result<()> {
    store.set_fuel(store.data().fuel_per_invocation)
}

impl WasmModuleInstance {
    pub fn load_from_path_with_context(
        path: &Path,
        storage: Arc<dyn WasmStorage>,
        config: HashMap<String, String>,
    ) -> Result<Self, wasmtime::Error> {
        let wasm_bytes = std::fs::read(path)
            .map_err(|e| wasmtime::Error::msg(format!("Failed to read WASM file: {e}")))?;
        Self::load_from_bytes_with_context(&wasm_bytes, storage, config)
    }

    pub fn load_from_bytes_with_context(
        wasm_bytes: &[u8],
        storage: Arc<dyn WasmStorage>,
        config: HashMap<String, String>,
    ) -> Result<Self, wasmtime::Error> {
        let engine = limits::wasm_engine_with_fuel()?;
        let host_ctx = WasmHostContext::new(storage, config);
        let mut store = Store::new(&engine, host_ctx);
        store.limiter(|host| &mut host.wasm_limits);
        let linker =
            create_host_imports(&engine).map_err(|e| wasmtime::Error::msg(e.to_string()))?;
        let module = Module::new(&engine, wasm_bytes)?;
        let instance = linker.instantiate(&mut store, &module)?;

        Ok(Self {
            _engine: engine,
            store: Mutex::new(store),
            instance,
        })
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    fn call_string_export(&self, name: &str) -> Result<String, wasmtime::Error> {
        let mut store = self
            .store
            .lock()
            .map_err(|e| wasmtime::Error::msg(e.to_string()))?;
        let func = self
            .instance
            .get_func(&mut *store, name)
            .ok_or_else(|| wasmtime::Error::msg(format!("Export '{name}' not found")))?;

        reset_guest_fuel(&mut store)?;
        let mut results = [wasmtime::Val::I32(0), wasmtime::Val::I32(0)];
        func.call(&mut *store, &[], &mut results)?;

        let ptr = results[0].i32().ok_or_else(|| {
            wasmtime::Error::msg(format!("Export '{name}' must return (i32, i32); bad ptr"))
        })? as u32;
        let len = results[1].i32().ok_or_else(|| {
            wasmtime::Error::msg(format!("Export '{name}' must return (i32, i32); bad len"))
        })? as u32;

        if len == 0 {
            return Ok(String::new());
        }

        let memory = self
            .instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| wasmtime::Error::msg("Module has no 'memory' export"))?;

        let data = memory.data(&mut *store);
        let end = (ptr + len) as usize;
        if end > data.len() {
            return Err(wasmtime::Error::msg(format!(
                "Export '{name}' returned invalid ptr/len"
            )));
        }
        let bytes = &data[ptr as usize..end];
        String::from_utf8(bytes.to_vec()).map_err(|e| wasmtime::Error::msg(e.to_string()))
    }

    pub fn module_name(&self) -> Result<String, wasmtime::Error> {
        self.call_string_export("module_name")
    }

    pub fn module_version(&self) -> Result<String, wasmtime::Error> {
        self.call_string_export("module_version")
    }

    fn cli_spec_json(&self) -> Result<String, wasmtime::Error> {
        self.call_string_export("cli_spec")
    }

    /// Call an export that takes (ptr, len, ptr, len, ctx) and returns (ptr, len).
    fn call_dispatch(
        &self,
        export_name: &str,
        arg1: &str,
        arg2: &[u8],
    ) -> Result<String, wasmtime::Error> {
        let mut store = self
            .store
            .lock()
            .map_err(|e| wasmtime::Error::msg(e.to_string()))?;
        let func = self
            .instance
            .get_func(&mut *store, export_name)
            .ok_or_else(|| wasmtime::Error::msg(format!("Export '{export_name}' not found")))?;

        let memory = self
            .instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| wasmtime::Error::msg("Module has no 'memory' export"))?;

        let arg1_bytes = arg1.as_bytes();
        let arg1_len = arg1_bytes.len().min(BUF_SIZE);
        let arg2_len = arg2.len().min(BUF_SIZE);

        memory.write(&mut *store, BUF_A_OFFSET, &arg1_bytes[..arg1_len])?;
        memory.write(&mut *store, BUF_B_OFFSET, &arg2[..arg2_len])?;

        reset_guest_fuel(&mut store)?;
        let mut results = [wasmtime::Val::I32(0), wasmtime::Val::I32(0)];
        func.call(
            &mut *store,
            &[
                wasmtime::Val::I32(BUF_A_OFFSET as i32),
                wasmtime::Val::I32(arg1_len as i32),
                wasmtime::Val::I32(BUF_B_OFFSET as i32),
                wasmtime::Val::I32(arg2_len as i32),
                wasmtime::Val::I32(0), // ctx_handle
            ],
            &mut results,
        )?;

        let ptr = results[0].i32().ok_or_else(|| {
            wasmtime::Error::msg(format!(
                "Export '{export_name}' must return (i32, i32); bad ptr"
            ))
        })? as u32;
        let len = results[1].i32().ok_or_else(|| {
            wasmtime::Error::msg(format!(
                "Export '{export_name}' must return (i32, i32); bad len"
            ))
        })? as u32;

        if len == 0 {
            return Ok(String::new());
        }

        let data = memory.data(&mut *store);
        let end = (ptr + len) as usize;
        if end > data.len() {
            return Err(wasmtime::Error::msg(format!(
                "Export '{export_name}' returned invalid ptr/len"
            )));
        }
        let bytes = &data[ptr as usize..end];
        String::from_utf8(bytes.to_vec()).map_err(|e| wasmtime::Error::msg(e.to_string()))
    }
}

impl blvm_node::module::wasm::WasmModuleInstance for WasmModuleInstance {
    fn invoke_cli(
        &self,
        subcommand: &str,
        args: Vec<String>,
    ) -> Result<InvocationResultPayload, ModuleError> {
        let args_json = serde_json::to_vec(&args).map_err(|e| {
            ModuleError::SerializationError(format!("Failed to serialize args: {e}"))
        })?;

        let result = self
            .call_dispatch("dispatch_cli", subcommand, &args_json)
            .map_err(|e| ModuleError::OperationError(format!("WASM dispatch_cli failed: {e}")))?;

        // Result is JSON: { "stdout": "...", "stderr": "...", "exit_code": 0 }
        let parsed: serde_json::Value = serde_json::from_str(&result).map_err(|e| {
            ModuleError::OperationError(format!("WASM returned invalid CLI result JSON: {e}"))
        })?;

        let stdout = parsed
            .get("stdout")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let stderr = parsed
            .get("stderr")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let exit_code = parsed
            .get("exit_code")
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as i32;

        Ok(InvocationResultPayload::Cli {
            stdout,
            stderr,
            exit_code,
        })
    }

    fn invoke_rpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ModuleError> {
        let params_bytes = serde_json::to_vec(&params).map_err(|e| {
            ModuleError::SerializationError(format!("Failed to serialize params: {e}"))
        })?;

        let result = self
            .call_dispatch("dispatch_rpc", method, &params_bytes)
            .map_err(|e| ModuleError::OperationError(format!("WASM dispatch_rpc failed: {e}")))?;

        serde_json::from_str(&result).map_err(|e| {
            ModuleError::OperationError(format!("WASM returned invalid RPC result JSON: {e}"))
        })
    }

    fn cli_spec(&self) -> Option<CliSpec> {
        let json = self.cli_spec_json().ok()?;
        let parsed: serde_json::Value = serde_json::from_str(&json).ok()?;
        let obj = parsed.as_object()?;
        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| self.module_name().ok())?;
        let version = obj.get("version").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
        let about = obj.get("about").and_then(|v| v.as_str()).map(String::from);
        let subcommands = obj
            .get("subcommands")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let o = v.as_object()?;
                        let name = o.get("name")?.as_str()?.to_string();
                        let about = o.get("about").and_then(|v| v.as_str()).map(String::from);
                        let args = o
                            .get("args")
                            .and_then(|v| v.as_array())
                            .map(|a| {
                                a.iter()
                                    .filter_map(|arg| {
                                        let o = arg.as_object()?;
                                        Some(blvm_node::module::ipc::protocol::CliArgSpec {
                                            name: o.get("name")?.as_str()?.to_string(),
                                            long_name: o
                                                .get("long_name")
                                                .and_then(|v| v.as_str())
                                                .map(String::from),
                                            short_name: o
                                                .get("short_name")
                                                .and_then(|v| v.as_str())
                                                .map(String::from),
                                            required: o.get("required").and_then(|v| v.as_bool()),
                                            takes_value: o
                                                .get("takes_value")
                                                .and_then(|v| v.as_bool()),
                                            default: o
                                                .get("default")
                                                .and_then(|v| v.as_str())
                                                .map(String::from),
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        Some(blvm_node::module::ipc::protocol::CliSubcommandSpec {
                            name,
                            about,
                            args,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(CliSpec {
            version,
            name,
            about,
            subcommands,
        })
    }
}
