//! WASM module loader implementation for blvm-node.
//!
//! Implements blvm_node::module::wasm::WasmModuleLoader. Requires both
//! `node` and `wasm-modules` features.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use blvm_node::module::wasm::WasmModuleLoader;
use blvm_node::storage::database::{create_database, Database, DatabaseBackend, Tree};

use super::host::{WasmStorage, WasmTree};
use super::instance::WasmModuleInstance;

/// Adapter: blvm-node Tree → WasmTree
struct TreeAdapter(Arc<dyn Tree>);

impl WasmTree for TreeAdapter {
    fn insert(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
        self.0.insert(key, value).map_err(|e| e.to_string())
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, String> {
        self.0.get(key).map_err(|e| e.to_string())
    }

    fn remove(&self, key: &[u8]) -> Result<(), String> {
        self.0.remove(key).map_err(|e| e.to_string())
    }

    fn scan(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>, String> {
        let mut out = Vec::new();
        for r in self.0.iter() {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }
}

/// Adapter: blvm-node Database → WasmStorage
struct StorageAdapter {
    db: Arc<dyn Database>,
}

impl WasmStorage for StorageAdapter {
    fn open_tree(&self, name: &str) -> Result<Arc<dyn WasmTree>, String> {
        let tree = self.db.open_tree(name).map_err(|e| e.to_string())?;
        Ok(Arc::new(TreeAdapter(Arc::from(tree))))
    }
}

/// Loader that bridges blvm-sdk's WASM runtime to blvm-node storage.
pub struct BlvmSdkWasmLoader;

impl WasmModuleLoader for BlvmSdkWasmLoader {
    fn load(
        &self,
        path: &Path,
        data_dir: &Path,
        config: HashMap<String, String>,
    ) -> Result<Arc<dyn blvm_node::module::wasm::WasmModuleInstance>, String> {
        let db_path = data_dir.join("db");
        std::fs::create_dir_all(&db_path).map_err(|e| e.to_string())?;
        let db =
            create_database(&db_path, DatabaseBackend::Redb, None).map_err(|e| e.to_string())?;
        let storage = Arc::new(StorageAdapter { db: Arc::from(db) });
        let instance = WasmModuleInstance::load_from_path_with_context(path, storage, config)
            .map_err(|e| e.to_string())?;
        Ok(Arc::new(instance))
    }
}
