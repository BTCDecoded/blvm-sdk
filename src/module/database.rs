//! Module database utilities
//!
//! Each module has its own database at `{data_dir}/db/`. Use this helper to open it.
//!
//! `MODULE_CONFIG_DATABASE_BACKEND` is normally set by the node when spawning a module, from
//! `[storage] database_backend` and optional `[modules] module_database_backend` (see
//! `blvm_node::storage::database::module_subprocess_database_backend_preference`). That value
//! matches the chain store name; if it is not suitable for dynamic module trees (RocksDB / Redb),
//! this crate falls back by trying **Sled** then **TidesDB** via
//! [`create_database`](blvm_node::storage::database::create_database) (same order as
//! `try_create_module_kv_database` in newer `blvm-node`).

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
        // "auto" and anything unrecognised: let the caller fall through to the default selection
        _ => default_backend(),
    }
}

/// Returns true for backends that support fully dynamic `open_tree()` without any pre-declaration.
///
/// - **Sled**: any tree name is created on demand.
/// - **TidesDB**: uses `get_or_create_cf`, fully dynamic.
/// - **Redb**: only works for a hard-coded set of node/module tables (`schema`, `items`, etc.).
///   Arbitrary tree names return an error — use Sled or TidesDB for general module storage.
/// - **RocksDB**: column families must be declared at database open time.
fn supports_dynamic_trees(backend: DatabaseBackend) -> bool {
    matches!(backend, DatabaseBackend::Sled | DatabaseBackend::TidesDB)
}

/// Best available backend for module databases (prefers fully-dynamic backends).
///
/// Returns Sled as the preferred target; `create_database` will surface an error if the
/// feature is not compiled into blvm-node, and the caller falls through to TidesDB.
fn best_module_backend() -> DatabaseBackend {
    // Prefer Sled: lightweight, embedded, fully dynamic open_tree().
    // If Sled is not compiled in, create_database will return an error and the caller
    // falls back to TidesDB. The node default (rocksdb) is intentionally NOT used here.
    DatabaseBackend::Sled
}

/// Try Sled then TidesDB for a module-process KV store (dynamic `open_tree` names).
///
/// Uses only [`create_database`] so this builds against `blvm-node` releases that predate the
/// `try_create_module_kv_database` helper.
fn try_open_dynamic_module_kv(db_path: &Path) -> Result<Arc<dyn Database>> {
    let sled_err = match create_database(db_path, DatabaseBackend::Sled, None) {
        Ok(db) => return Ok(Arc::from(db)),
        Err(e) => e,
    };
    match create_database(db_path, DatabaseBackend::TidesDB, None) {
        Ok(db) => Ok(Arc::from(db)),
        Err(e) => Err(anyhow::anyhow!(
            "Failed to open module KV store at {:?}: sled: {}; tidesdb: {}",
            db_path,
            sled_err,
            e
        )),
    }
}

/// Open the module's database at `{data_dir}/db/`.
///
/// Module databases need to create named trees on demand (`open_tree()`). Only **Sled** and
/// **TidesDB** support fully dynamic tree creation. Redb only supports a fixed set of
/// pre-declared tables; RocksDB requires all column families to be declared at open time.
///
/// Backend selection (first match wins):
/// 1. `MODULE_CONFIG_DATABASE_BACKEND` — set by the node from effective module DB policy (see
///    `blvm_node::storage::database::module_subprocess_database_backend_preference`)
/// 2. `MODULE_DATABASE_BACKEND` env var (same values; manual override)
/// 3. Auto-select Sled as first choice, then fall back through Sled→TidesDB
///    [`create_database`](blvm_node::storage::database::create_database) attempts
///
/// If the resolved backend does not support dynamic trees the function warns and falls back
/// as above. If no dynamic backend is compiled into `blvm-node`, it returns an error.
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

    let explicit = std::env::var("MODULE_CONFIG_DATABASE_BACKEND")
        .or_else(|_| std::env::var("MODULE_DATABASE_BACKEND"))
        .ok();

    let backend = explicit
        .as_deref()
        .map(parse_backend)
        .unwrap_or_else(best_module_backend);

    if supports_dynamic_trees(backend) {
        // Explicitly requested or auto-selected a good module backend.
        return create_database(&db_path, backend, None).map(Arc::from);
    }

    // The resolved backend does not support fully-dynamic open_tree().
    if explicit.is_some() {
        // User explicitly asked for this backend — warn but still try to fall back so
        // existing module processes don't hard-fail on start-up.
        warn!(
            "MODULE_DATABASE_BACKEND={:?} does not support dynamic open_tree(). \
             Module databases require Sled or TidesDB. \
             Set MODULE_CONFIG_DATABASE_BACKEND=sled (or tidesdb) to remove this warning. \
             Note: Redb only supports pre-declared tables (schema, items); \
             RocksDB requires all column families at open time.",
            backend
        );
    } else {
        // Auto-detected a static backend (e.g. rocksdb is the node default).
        warn!(
            "Default backend {:?} does not support dynamic open_tree(); \
             auto-selecting best available module backend (sled or tidesdb). \
             Set MODULE_CONFIG_DATABASE_BACKEND=sled to suppress this warning.",
            backend
        );
    }

    // Use whichever dynamic KV backends are compiled into blvm-node (Sled / TidesDB).
    try_open_dynamic_module_kv(&db_path).map_err(|e| {
        anyhow::anyhow!(
            "{e} (hint: {:?} is not usable for arbitrary module `open_tree` names; \
             `blvm-sdk` feature `node` enables `blvm-node/sled`, or build `blvm-node` with `tidesdb`)",
            backend
        )
    })
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
