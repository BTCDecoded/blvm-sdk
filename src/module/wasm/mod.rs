//! WASM module runtime.
//!
//! Loads `.wasm` modules and runs them with host imports (storage, config, log).
//! Uses trait-based storage so the embedder (e.g. blvm-node) provides the implementation.

mod host;
mod instance;
#[cfg(all(feature = "node", feature = "wasm-modules"))]
mod loader;

pub use host::{create_host_imports, WasmHostContext, WasmStorage, WasmTree};
pub use instance::WasmModuleInstance;

#[cfg(all(feature = "node", feature = "wasm-modules"))]
pub use loader::BlvmSdkWasmLoader;
