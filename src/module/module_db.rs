//! Standard module storage API.
//!
//! All modules use this single system: `ModuleDb` for persistent storage.
//! No ad-hoc storage—open your DB, run migrations, use named trees.
//!
//! ## Usage
//!
//! ```ignore
//! let db = ModuleDb::open(&bootstrap.data_dir)?;
//! db.run_migrations(migrations!(1 => up_initial, 2 => up_add_proposals))?;
//! let tree = db.tree("proposals")?;
//! tree.insert(b"key", b"value")?;
//! ```
//!
//! ## Tree naming
//!
//! Define tree names as constants in your module (e.g. `const PROPOSALS_TREE: &str = "proposals"`).
//! Avoid generic names like `"items"`—use descriptive names for each logical store.

use anyhow::Result;
use blvm_node::storage::database::{Database, Tree};
use std::path::Path;
use std::sync::Arc;

use crate::module::database::{open_module_db, run_migrations, MigrationUp};

/// Standard module database.
///
/// Single entry point for all module storage. Opens DB at `{data_dir}/db/`,
/// runs migrations, provides named trees.
#[derive(Clone)]
pub struct ModuleDb {
    db: Arc<dyn Database>,
}

impl ModuleDb {
    /// Open the module database at `{data_dir}/db/`.
    ///
    /// Uses the same backend as the node when `MODULE_CONFIG_DATABASE_BACKEND` is set.
    pub fn open<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let db = open_module_db(data_dir)?;
        Ok(Self { db })
    }

    /// Open and run migrations. Convention for modules that use migrations.
    pub fn open_with_migrations<P: AsRef<Path>>(
        data_dir: P,
        migrations: &[(u32, MigrationUp)],
    ) -> Result<Self> {
        let db = Self::open(data_dir)?;
        db.run_migrations(migrations)?;
        Ok(db)
    }

    /// Open at data_dir, or fallback to temp dir when data_dir fails (e.g. standalone without data dir).
    pub fn open_or_temp<P: AsRef<Path>>(data_dir: P, module_name: &str) -> Result<Self> {
        Self::open(&data_dir).or_else(|_| {
            let temp = std::env::temp_dir().join(module_name);
            Self::open(&temp).or_else(|_| {
                let dir = temp.join("db");
                std::fs::create_dir_all(&dir).ok();
                let db = blvm_node::storage::database::create_database(
                    &dir,
                    blvm_node::storage::database::DatabaseBackend::Redb,
                    None,
                )?;
                Ok(Self { db: Arc::from(db) })
            })
        })
    }

    /// Like `open_or_temp` but runs migrations when the primary data_dir succeeds.
    pub fn open_or_temp_with_migrations<P: AsRef<Path>>(
        data_dir: P,
        module_name: &str,
        migrations: &[(u32, MigrationUp)],
    ) -> Result<Self> {
        let db = Self::open_or_temp(data_dir, module_name)?;
        let _ = db.run_migrations(migrations);
        Ok(db)
    }

    /// Run pending migrations.
    pub fn run_migrations(&self, migrations: &[(u32, MigrationUp)]) -> Result<()> {
        run_migrations(&self.db, migrations)
    }

    /// Open a named tree. Use descriptive constants defined in your module.
    pub fn tree(&self, name: &str) -> Result<Arc<dyn Tree>> {
        let t = self.db.open_tree(name)?;
        Ok(Arc::from(t))
    }

    /// Access the underlying database (for compatibility with code expecting `Arc<dyn Database>`).
    pub fn as_db(&self) -> Arc<dyn Database> {
        Arc::clone(&self.db)
    }
}
