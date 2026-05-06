//! Module database utilities
//!
//! Each module has its own database at `{data_dir}/db/`. Use this helper to open it.
//! Modules use the same format as the node by default (via MODULE_CONFIG_DATABASE_BACKEND).

use anyhow::Result;
use blvm_node::storage::database::{create_database, default_backend, Database, DatabaseBackend};
use std::path::Path;
use std::sync::Arc;
use tracing::warn;

fn parse_backend(s: &str) -> DatabaseBackend {
    match s.to_lowercase().as_str() {
        "redb" => DatabaseBackend::Redb,
        "rocksdb" => DatabaseBackend::RocksDB,
        "sled" => DatabaseBackend::Sled,
        "tidesdb" => DatabaseBackend::TidesDB,
        "auto" => default_backend(),
        _ => DatabaseBackend::Redb, // fallback for unknown/standalone
    }
}

/// Open the module's database at `{data_dir}/db/`.
///
/// Uses the same backend as the node when `MODULE_CONFIG_DATABASE_BACKEND` is set
/// (redb, rocksdb, sled, tidesdb, auto). Falls back to the node's compiled-in
/// `default_backend()` (rocksdb when built with default features).
///
/// # Example
/// ```ignore
/// let db = open_module_db(module_data_dir)?;
/// let tree = db.open_tree("my_tree")?;
/// tree.insert(b"key", b"value")?;
/// ```
pub fn open_module_db<P: AsRef<Path>>(data_dir: P) -> Result<Arc<dyn Database>> {
    let db_path = data_dir.as_ref().join("db");
    std::fs::create_dir_all(&db_path)?;
    let backend = std::env::var("MODULE_CONFIG_DATABASE_BACKEND")
        .or_else(|_| std::env::var("MODULE_DATABASE_BACKEND"))
        .map(|s| parse_backend(&s))
        .unwrap_or_else(|_| default_backend());

    // Redb and Sled support dynamic open_tree(); RocksDB and TidesDB require all column
    // families to be pre-declared at open time, which is incompatible with the module
    // system's on-demand tree creation. Auto-fallback to Redb (or Sled) when needed.
    match backend {
        DatabaseBackend::Redb | DatabaseBackend::Sled => {
            create_database(&db_path, backend, None).map(Arc::from)
        }
        other => {
            warn!(
                "Module DB backend {:?} does not support dynamic open_tree(); \
                 falling back to Redb or Sled. Set MODULE_DATABASE_BACKEND=redb \
                 (or sled) to suppress this warning.",
                other
            );
            create_database(&db_path, DatabaseBackend::Redb, None)
                .or_else(|_| create_database(&db_path, DatabaseBackend::Sled, None))
                .map(Arc::from)
                .map_err(|_| {
                    anyhow::anyhow!(
                        "Module DB backend {:?} requires pre-declared column families. \
                         Enable the 'redb' or 'sled' feature on blvm-node, or set \
                         MODULE_DATABASE_BACKEND to 'redb' or 'sled'.",
                        other
                    )
                })
        }
    }
}

/// Schema version key (stored in the schema tree).
const SCHEMA_VERSION_KEY: &[u8] = b"schema_version";

/// Context passed to migration functions. Provides put/get/delete against the module's schema tree
/// and access to the database for opening other trees.
///
/// Migrations run locally in the module process. Use `put`/`get`/`delete` for schema metadata.
/// For application data migrations, use `open_tree` to open and migrate other trees.
#[derive(Clone)]
pub struct MigrationContext {
    tree: Arc<dyn blvm_node::storage::database::Tree>,
    db: Arc<dyn Database>,
}

impl MigrationContext {
    /// Create a new MigrationContext wrapping the schema tree and database.
    pub fn new(tree: Arc<dyn blvm_node::storage::database::Tree>, db: Arc<dyn Database>) -> Self {
        Self { tree, db }
    }

    /// Insert a key-value pair into the schema tree.
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.tree.insert(key, value)
    }

    /// Get a value by key from the schema tree.
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.tree.get(key)
    }

    /// Remove a key from the schema tree.
    pub fn delete(&self, key: &[u8]) -> Result<()> {
        self.tree.remove(key)
    }

    /// Open a named tree for application data migrations.
    pub fn open_tree(&self, name: &str) -> Result<Box<dyn blvm_node::storage::database::Tree>> {
        self.db.open_tree(name)
    }
}

/// A single up migration step.
pub type MigrationUp = fn(&MigrationContext) -> Result<()>;

/// A single down migration step (for rollback).
pub type MigrationDown = fn(&MigrationContext) -> Result<()>;

/// Migration pair: (version, up, optional down for rollback).
pub type Migration = (u32, MigrationUp, Option<MigrationDown>);

/// Run pending up migrations. Opens the "schema" tree, reads current version, runs each migration
/// with version > current in order, then updates schema_version.
///
/// # Example
/// ```ignore
/// let db = open_module_db(data_dir)?;
/// run_migrations(&db, &[(1, up_initial, Some(down_initial)), (2, up_add_cache, None)])?;
/// ```
pub fn run_migrations(db: &Arc<dyn Database>, migrations: &[(u32, MigrationUp)]) -> Result<()> {
    run_migrations_with_down(
        db,
        &migrations
            .iter()
            .map(|(v, u)| (*v, *u, None))
            .collect::<Vec<_>>(),
    )
}

/// Run pending up migrations. Supports optional down migrations for rollback.
pub fn run_migrations_with_down(db: &Arc<dyn Database>, migrations: &[Migration]) -> Result<()> {
    let tree = db.open_tree("schema")?;
    let tree = Arc::from(tree);
    let ctx = MigrationContext::new(tree, Arc::clone(db));

    let current: u32 = ctx
        .get(SCHEMA_VERSION_KEY)?
        .and_then(|v| String::from_utf8(v).ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let mut pending: Vec<_> = migrations
        .iter()
        .filter(|(v, _, _)| *v > current)
        .copied()
        .collect();
    pending.sort_by_key(|(v, _, _)| *v);

    for (version, up, _down) in pending {
        up(&ctx)?;
        ctx.put(SCHEMA_VERSION_KEY, version.to_string().as_bytes())?;
    }

    Ok(())
}

/// Rollback migrations down to `target_version` (exclusive). Runs down migrations in reverse
/// order for each applied version > target_version. Requires down functions to be provided.
///
/// # Example
/// ```ignore
/// run_migrations_down(&db, &[(1, up_initial, Some(down_initial)), (2, up_add_cache, Some(down_cache))], 1)?;
/// // Rolls back from 2 to 1 (runs down_cache only).
/// ```
pub fn run_migrations_down(
    db: &Arc<dyn Database>,
    migrations: &[Migration],
    target_version: u32,
) -> Result<()> {
    let tree = db.open_tree("schema")?;
    let tree = Arc::from(tree);
    let ctx = MigrationContext::new(tree, Arc::clone(db));

    let current: u32 = ctx
        .get(SCHEMA_VERSION_KEY)?
        .and_then(|v| String::from_utf8(v).ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if current <= target_version {
        return Ok(());
    }

    let mut to_rollback: Vec<_> = migrations
        .iter()
        .filter(|(v, _, d)| *v > target_version && *v <= current && d.is_some())
        .copied()
        .collect();
    to_rollback.sort_by_key(|(v, _, _)| std::cmp::Reverse(*v));

    for (version, _up, down) in to_rollback {
        if let Some(down_fn) = down {
            down_fn(&ctx)?;
        } else {
            anyhow::bail!("Migration version {} has no down function", version);
        }
    }

    ctx.put(SCHEMA_VERSION_KEY, target_version.to_string().as_bytes())?;
    Ok(())
}
