//! Host import implementations for WASM modules.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use wasmtime::{Engine, Extern, Linker, StoreLimits};

/// Key-value pair for WASM host storage iteration.
type WasmKvPair = (Vec<u8>, Vec<u8>);

/// Minimal tree interface for WASM host storage. Embedder implements this.
pub trait WasmTree: Send + Sync {
    fn insert(&self, key: &[u8], value: &[u8]) -> Result<(), String>;
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, String>;
    fn remove(&self, key: &[u8]) -> Result<(), String>;

    /// Scan all key-value pairs for iteration. Used by host_storage_iter_*.
    fn scan(&self) -> Result<Vec<WasmKvPair>, String>;
}

/// Minimal storage interface. Embedder implements this (e.g. wraps blvm-node Database).
pub trait WasmStorage: Send + Sync {
    fn open_tree(&self, name: &str) -> Result<Arc<dyn WasmTree>, String>;
}

/// Active iterator: (pairs, current_index)
type IterState = (Vec<(Vec<u8>, Vec<u8>)>, usize);

/// Host context stored in the wasmtime Store.
pub struct WasmHostContext {
    storage: Arc<dyn WasmStorage>,
    config: HashMap<String, String>,
    trees: RefCell<HashMap<i32, Arc<dyn WasmTree>>>,
    next_tree_id: RefCell<i32>,
    iters: RefCell<HashMap<i32, IterState>>,
    next_iter_id: RefCell<i32>,
    /// [`StoreLimits`] wired via [`wasmtime::Store::limiter`] for memory/instance caps.
    pub(crate) wasm_limits: StoreLimits,
    /// Fuel reset before each guest entry when [`wasmtime::Config::consume_fuel`] is enabled.
    pub(crate) fuel_per_invocation: u64,
}

impl WasmHostContext {
    pub fn new(storage: Arc<dyn WasmStorage>, config: HashMap<String, String>) -> Self {
        let wasm_limits = crate::module::wasm::limits::store_limits(&config);
        let fuel_per_invocation = crate::module::wasm::limits::fuel_per_invocation(&config);
        Self {
            storage,
            config,
            trees: RefCell::new(HashMap::new()),
            next_tree_id: RefCell::new(1),
            iters: RefCell::new(HashMap::new()),
            next_iter_id: RefCell::new(1),
            wasm_limits,
            fuel_per_invocation,
        }
    }

    fn open_tree(&self, name: &str) -> Result<i32, String> {
        let tree = self.storage.open_tree(name)?;
        let id = {
            let mut next = self.next_tree_id.borrow_mut();
            let id = *next;
            *next += 1;
            id
        };
        self.trees.borrow_mut().insert(id, tree);
        Ok(id)
    }

    fn get_tree(&self, tree_id: i32) -> Result<Arc<dyn WasmTree>, String> {
        self.trees
            .borrow()
            .get(&tree_id)
            .cloned()
            .ok_or_else(|| format!("Unknown tree_id {tree_id}"))
    }

    fn iter_open(&self, tree_id: i32) -> Result<i32, String> {
        let tree = self.get_tree(tree_id)?;
        let pairs = tree.scan()?;
        let id = {
            let mut next = self.next_iter_id.borrow_mut();
            let id = *next;
            *next += 1;
            id
        };
        self.iters.borrow_mut().insert(id, (pairs, 0));
        Ok(id)
    }

    fn iter_next(&self, iter_handle: i32) -> Result<Option<WasmKvPair>, String> {
        let mut iters = self.iters.borrow_mut();
        let (pairs, idx) = iters
            .get_mut(&iter_handle)
            .ok_or_else(|| format!("Unknown iter_handle {iter_handle}"))?;
        if *idx >= pairs.len() {
            return Ok(None);
        }
        let (k, v) = pairs[*idx].clone();
        *idx += 1;
        Ok(Some((k, v)))
    }

    fn iter_close(&self, iter_handle: i32) {
        self.iters.borrow_mut().remove(&iter_handle);
    }
}

pub fn create_host_imports(engine: &Engine) -> Result<Linker<WasmHostContext>, String> {
    let mut linker = Linker::new(engine);

    linker
        .func_wrap(
            "env",
            "host_log",
            |mut caller: wasmtime::Caller<'_, WasmHostContext>, level: i32, ptr: i32, len: i32| {
                if len <= 0 {
                    return;
                }
                if let Some(mem) = caller.get_export("memory").and_then(Extern::into_memory) {
                    let ptr_u = ptr as usize;
                    let len_u = len as usize;
                    let mut buf = vec![0u8; len_u];
                    if mem.read(&caller, ptr_u, &mut buf).is_ok() {
                        let msg = String::from_utf8_lossy(&buf);
                        match level {
                            0 => tracing::trace!("[wasm] {}", msg),
                            1 => tracing::debug!("[wasm] {}", msg),
                            2 => tracing::info!("[wasm] {}", msg),
                            3 => tracing::warn!("[wasm] {}", msg),
                            _ => tracing::error!("[wasm] {}", msg),
                        }
                    }
                }
            },
        )
        .map_err(|e| e.to_string())?;

    linker
        .func_wrap(
            "env",
            "host_storage_open_tree",
            |mut caller: wasmtime::Caller<'_, WasmHostContext>,
             name_ptr: i32,
             name_len: i32|
             -> i32 {
                read_str_and_then(&mut caller, name_ptr, name_len, |ctx, name| {
                    ctx.open_tree(name).map_err(|e| e.to_string())
                })
                .unwrap_or(-1)
            },
        )
        .map_err(|e| e.to_string())?;

    linker
        .func_wrap(
            "env",
            "host_storage_put",
            |mut caller: wasmtime::Caller<'_, WasmHostContext>,
             tree_id: i32,
             k_ptr: i32,
             k_len: i32,
             v_ptr: i32,
             v_len: i32|
             -> i32 {
                let ctx = caller.data();
                let tree = match ctx.get_tree(tree_id) {
                    Ok(t) => t,
                    Err(_) => return -1,
                };
                let k = read_slice(&mut caller, k_ptr, k_len).unwrap_or_default();
                let v = read_slice(&mut caller, v_ptr, v_len).unwrap_or_default();
                tree.insert(&k, &v).map(|_| 0).unwrap_or(-1)
            },
        )
        .map_err(|e| e.to_string())?;

    linker
        .func_wrap(
            "env",
            "host_storage_get",
            |mut caller: wasmtime::Caller<'_, WasmHostContext>,
             tree_id: i32,
             k_ptr: i32,
             k_len: i32,
             out_ptr: i32,
             out_len: i32|
             -> i32 {
                let ctx = caller.data();
                let tree = match ctx.get_tree(tree_id) {
                    Ok(t) => t,
                    Err(_) => return -1,
                };
                let k = read_slice(&mut caller, k_ptr, k_len).unwrap_or_default();
                let value = match tree.get(&k) {
                    Ok(Some(v)) => v,
                    Ok(None) | Err(_) => return -1,
                };
                if (out_len as usize) < value.len() {
                    return -1;
                }
                write_slice(&mut caller, out_ptr, out_len, &value)
                    .map(|_| value.len() as i32)
                    .unwrap_or(-1)
            },
        )
        .map_err(|e| e.to_string())?;

    linker
        .func_wrap(
            "env",
            "host_storage_remove",
            |mut caller: wasmtime::Caller<'_, WasmHostContext>,
             tree_id: i32,
             k_ptr: i32,
             k_len: i32|
             -> i32 {
                let ctx = caller.data();
                let tree = match ctx.get_tree(tree_id) {
                    Ok(t) => t,
                    Err(_) => return -1,
                };
                let k = read_slice(&mut caller, k_ptr, k_len).unwrap_or_default();
                tree.remove(&k).map(|_| 0).unwrap_or(-1)
            },
        )
        .map_err(|e| e.to_string())?;

    linker
        .func_wrap(
            "env",
            "host_storage_iter_open",
            |caller: wasmtime::Caller<'_, WasmHostContext>, tree_id: i32| -> i32 {
                caller.data().iter_open(tree_id).unwrap_or(-1)
            },
        )
        .map_err(|e| e.to_string())?;

    linker
        .func_wrap(
            "env",
            "host_storage_iter_next",
            |mut caller: wasmtime::Caller<'_, WasmHostContext>,
             iter_handle: i32,
             k_out_ptr: i32,
             k_out_len: i32,
             v_out_ptr: i32,
             v_out_len: i32|
             -> i32 {
                let ctx = caller.data();
                let (k, v) = match ctx.iter_next(iter_handle) {
                    Ok(Some(pair)) => pair,
                    Ok(None) => return 0,
                    Err(_) => return -1,
                };
                if (k_out_len as usize) < k.len() || (v_out_len as usize) < v.len() {
                    return -1;
                }
                if write_slice(&mut caller, k_out_ptr, k_out_len, &k).is_err() {
                    return -1;
                }
                if write_slice(&mut caller, v_out_ptr, v_out_len, &v).is_err() {
                    return -1;
                }
                1
            },
        )
        .map_err(|e| e.to_string())?;

    linker
        .func_wrap(
            "env",
            "host_storage_iter_close",
            |caller: wasmtime::Caller<'_, WasmHostContext>, iter_handle: i32| {
                caller.data().iter_close(iter_handle);
            },
        )
        .map_err(|e| e.to_string())?;

    linker
        .func_wrap(
            "env",
            "host_config_get",
            |mut caller: wasmtime::Caller<'_, WasmHostContext>,
             key_ptr: i32,
             key_len: i32,
             out_ptr: i32,
             out_len: i32|
             -> i32 {
                let key = read_str(&mut caller, key_ptr, key_len).unwrap_or_default();
                let ctx = caller.data();
                let value = ctx.config.get(&key).cloned().unwrap_or_default();
                let bytes = value.as_bytes();
                if (out_len as usize) < bytes.len() {
                    return -1;
                }
                write_slice(&mut caller, out_ptr, out_len, bytes)
                    .map(|_| bytes.len() as i32)
                    .unwrap_or(-1)
            },
        )
        .map_err(|e| e.to_string())?;

    linker
        .func_wrap(
            "env",
            "host_ctx_storage_open_tree",
            |mut caller: wasmtime::Caller<'_, WasmHostContext>,
             _ctx_handle: i32,
             name_ptr: i32,
             name_len: i32|
             -> i32 {
                read_str_and_then(&mut caller, name_ptr, name_len, |ctx, name| {
                    ctx.open_tree(name).map_err(|e| e.to_string())
                })
                .unwrap_or(-1)
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(linker)
}

fn read_str_and_then<F, R>(
    caller: &mut wasmtime::Caller<'_, WasmHostContext>,
    ptr: i32,
    len: i32,
    f: F,
) -> Result<R, String>
where
    F: FnOnce(&WasmHostContext, &str) -> Result<R, String>,
{
    let s = read_str(caller, ptr, len).ok_or_else(|| "read_str failed".to_string())?;
    f(caller.data(), &s)
}

fn read_str(
    caller: &mut wasmtime::Caller<'_, WasmHostContext>,
    ptr: i32,
    len: i32,
) -> Option<String> {
    let bytes = read_slice(caller, ptr, len)?;
    String::from_utf8(bytes).ok()
}

fn read_slice(
    caller: &mut wasmtime::Caller<'_, WasmHostContext>,
    ptr: i32,
    len: i32,
) -> Option<Vec<u8>> {
    if len <= 0 {
        return Some(vec![]);
    }
    let ptr_u = ptr as usize;
    let len_u = len as usize;
    let mem = caller.get_export("memory")?.into_memory()?;
    let mut buf = vec![0u8; len_u];
    mem.read(caller, ptr_u, &mut buf).ok()?;
    Some(buf)
}

fn write_slice(
    caller: &mut wasmtime::Caller<'_, WasmHostContext>,
    ptr: i32,
    max_len: i32,
    data: &[u8],
) -> Result<(), String> {
    let ptr_u = ptr as usize;
    let max = max_len as usize;
    let to_write = data.len().min(max);
    if to_write == 0 && !data.is_empty() {
        return Err("output buffer too small".to_string());
    }
    if to_write == 0 {
        return Ok(());
    }
    let mem = caller
        .get_export("memory")
        .and_then(Extern::into_memory)
        .ok_or_else(|| "no memory export".to_string())?;
    mem.write(caller, ptr_u, &data[..to_write])
        .map_err(|e| e.to_string())?;
    Ok(())
}
