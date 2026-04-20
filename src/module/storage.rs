//! Module storage abstraction.
//!
//! Provides `ModuleStorage` and `ModuleTree` traits so modules can use storage
//! without depending on a specific backend. Native modules use `DatabaseStorageAdapter`;
//! WASM modules will use host-provided implementations.

use anyhow::Result;
use std::any::Any;
use std::sync::Arc;

/// Key-value tree interface for module storage.
///
/// Minimal interface that both native (Database) and WASM (host calls) can implement.
pub trait ModuleTree: Send + Sync {
    /// Insert a key-value pair.
    fn insert(&self, key: &[u8], value: &[u8]) -> Result<()>;

    /// Get a value by key.
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Remove a key.
    fn remove(&self, key: &[u8]) -> Result<()>;

    /// Iterate over all key-value pairs.
    fn iter(&self) -> Box<dyn Iterator<Item = Result<(Vec<u8>, Vec<u8>)>> + Send + '_>;
}

/// Storage interface for modules. Host implements this.
///
/// - **Native:** `DatabaseStorageAdapter` wraps local redb.
/// - **WASM:** Host provides implementation via host calls.
pub trait ModuleStorage: Send + Sync {
    /// Open a named tree/table.
    fn open_tree(&self, name: &str) -> Result<Arc<dyn ModuleTree>>;
}

/// Adapter: blvm-node `Tree` → `ModuleTree`.
struct TreeAdapter(Arc<dyn blvm_node::storage::database::Tree>);

impl ModuleTree for TreeAdapter {
    fn insert(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.0.insert(key, value)
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.0.get(key)
    }

    fn remove(&self, key: &[u8]) -> Result<()> {
        self.0.remove(key)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Result<(Vec<u8>, Vec<u8>)>> + Send + '_> {
        let items: Vec<_> = self.0.iter().collect();
        Box::new(items.into_iter())
    }
}

/// Adapter: blvm-node `Database` → `ModuleStorage`.
///
/// Wraps the existing database so it can be used as `ModuleStorage`.
/// Used by native modules.
pub struct DatabaseStorageAdapter {
    db: Arc<dyn blvm_node::storage::database::Database>,
}

impl DatabaseStorageAdapter {
    /// Create adapter from a database.
    pub fn new(db: Arc<dyn blvm_node::storage::database::Database>) -> Self {
        Self { db }
    }
}

impl ModuleStorage for DatabaseStorageAdapter {
    fn open_tree(&self, name: &str) -> Result<Arc<dyn ModuleTree>> {
        let tree = self.db.open_tree(name)?;
        let arc_tree = Arc::from(tree);
        Ok(Arc::new(TreeAdapter(arc_tree)))
    }
}

/// Adapter: `ModuleTree` → blvm-node `Tree`.
///
/// Allows `ModuleStorage` to be used where `Database` is expected (e.g. `ctx.db()`).
struct ModuleTreeAdapter(Arc<dyn ModuleTree>);

impl blvm_node::storage::database::Tree for ModuleTreeAdapter {
    fn insert(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.0.insert(key, value)
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.0.get(key)
    }

    fn remove(&self, key: &[u8]) -> Result<()> {
        self.0.remove(key)
    }

    fn contains_key(&self, key: &[u8]) -> Result<bool> {
        Ok(self.0.get(key)?.is_some())
    }

    fn clear(&self) -> Result<()> {
        for item in self.0.iter() {
            let (k, _) = item?;
            self.0.remove(&k)?;
        }
        Ok(())
    }

    fn len(&self) -> Result<usize> {
        let mut n = 0;
        for item in self.0.iter() {
            item?;
            n += 1;
        }
        Ok(n)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Result<(Vec<u8>, Vec<u8>)>> + '_> {
        self.0.iter()
    }

    fn batch(&self) -> anyhow::Result<Box<dyn blvm_node::storage::database::BatchWriter + '_>> {
        Ok(Box::new(SimpleBatchWriter {
            tree: self.0.clone(),
            puts: Vec::new(),
            deletes: Vec::new(),
        }))
    }
}

struct SimpleBatchWriter {
    tree: Arc<dyn ModuleTree>,
    puts: Vec<(Vec<u8>, Vec<u8>)>,
    deletes: Vec<Vec<u8>>,
}

impl blvm_node::storage::database::BatchWriter for SimpleBatchWriter {
    fn put(&mut self, key: &[u8], value: &[u8]) {
        self.deletes.retain(|k| k.as_slice() != key);
        self.puts.push((key.to_vec(), value.to_vec()));
    }

    fn delete(&mut self, key: &[u8]) {
        self.puts.retain(|(k, _)| k.as_slice() != key);
        self.deletes.push(key.to_vec());
    }

    fn commit(self: Box<Self>) -> Result<()> {
        for key in &self.deletes {
            self.tree.remove(key)?;
        }
        for (key, value) in &self.puts {
            self.tree.insert(key, value)?;
        }
        Ok(())
    }

    fn len(&self) -> usize {
        self.puts.len() + self.deletes.len()
    }
}

/// Bridge: `ModuleStorage` → blvm-node `Database`.
///
/// Allows `InvocationContext` to hold `ModuleStorage` while `ctx.db()` still
/// returns `Arc<dyn Database>` for compatibility with existing module code.
pub struct ModuleStorageDatabaseBridge {
    storage: Arc<dyn ModuleStorage>,
}

impl ModuleStorageDatabaseBridge {
    /// Create bridge from module storage.
    pub fn new(storage: Arc<dyn ModuleStorage>) -> Self {
        Self { storage }
    }
}

impl blvm_node::storage::database::Database for ModuleStorageDatabaseBridge {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn open_tree(&self, name: &str) -> Result<Box<dyn blvm_node::storage::database::Tree>> {
        let tree = self.storage.open_tree(name)?;
        Ok(Box::new(ModuleTreeAdapter(tree)))
    }

    fn flush(&self) -> Result<()> {
        Ok(())
    }
}
