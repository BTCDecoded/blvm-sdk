//! Wasmtime embedder limits parsed from per-module `config` (`HashMap` passed into `WasmModuleInstance`).

use std::collections::HashMap;
use wasmtime::{Engine, StoreLimits, StoreLimitsBuilder};

/// Default maximum linear memory (single memory) for guest modules — 32 MiB.
pub(crate) const DEFAULT_MAX_LINEAR_MEMORY_BYTES: usize = 32 * 1024 * 1024;
/// Default fuel granted before each guest entry (`dispatch_*`, exported getters).
pub(crate) const DEFAULT_FUEL_PER_INVOCATION: u64 = 50_000_000;

pub(crate) fn wasm_engine_with_fuel() -> Result<Engine, wasmtime::Error> {
    let mut cfg = wasmtime::Config::new();
    cfg.consume_fuel(true);
    Engine::new(&cfg)
}

pub(crate) fn store_limits(config: &HashMap<String, String>) -> StoreLimits {
    let max_mem = parse_usize(config.get("wasm_max_linear_memory_bytes"))
        .unwrap_or(DEFAULT_MAX_LINEAR_MEMORY_BYTES);
    let instances = parse_usize(config.get("wasm_max_instances")).unwrap_or(8);
    let tables = parse_usize(config.get("wasm_max_tables")).unwrap_or(64);
    let memories = parse_usize(config.get("wasm_max_memories")).unwrap_or(4);
    StoreLimitsBuilder::new()
        .memory_size(max_mem)
        .instances(instances)
        .tables(tables)
        .memories(memories)
        .build()
}

pub(crate) fn fuel_per_invocation(config: &HashMap<String, String>) -> u64 {
    parse_u64(config.get("wasm_fuel_per_invocation")).unwrap_or(DEFAULT_FUEL_PER_INVOCATION)
}

fn parse_usize(v: Option<&String>) -> Option<usize> {
    v.and_then(|s| s.trim().parse().ok())
}

fn parse_u64(v: Option<&String>) -> Option<u64> {
    v.and_then(|s| s.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::{Engine, Linker, Module, Store};

    #[test]
    fn fuel_exhausts_on_infinite_loop() {
        let engine = wasm_engine_with_fuel().expect("engine with fuel");
        let wat = r#"(module (memory 1) (func (export "spin") (loop (br 0))))"#;
        let module = Module::new(&engine, wat).expect("module");
        let linker = Linker::new(&engine);
        let mut store = Store::new(&engine, ());
        store.set_fuel(50_000).expect("set fuel");
        let instance = linker
            .instantiate(&mut store, &module)
            .expect("instantiate");
        let spin = instance
            .get_typed_func::<(), ()>(&mut store, "spin")
            .expect("spin export");
        assert!(
            spin.call(&mut store, ()).is_err(),
            "expected trap when fuel exhausted"
        );
    }

    #[test]
    fn memory_limit_blocks_default_sized_memory() {
        let engine = Engine::default();
        let wat = r#"(module (memory 1))"#;
        let module = Module::new(&engine, wat).expect("module");
        let linker = Linker::new(&engine);
        struct LimiterState {
            limits: StoreLimits,
        }
        let limits = StoreLimitsBuilder::new().memory_size(4096).build();
        let mut store = Store::new(&engine, LimiterState { limits });
        store.limiter(|state| &mut state.limits);
        assert!(
            linker.instantiate(&mut store, &module).is_err(),
            "instantiation should fail when linear memory exceeds cap"
        );
    }
}
