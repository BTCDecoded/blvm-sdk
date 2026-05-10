//! Demo-module: full example with config, CLI, RPC, storage, and migrations
//!
//! Uses #[module(migrations = ...)] on struct (name/config inferred), #[module] on impl.
//! CLI from #[command] methods (ctx: &InvocationContext), RPC from #[rpc_method], events from #[on_event].
//!
//! Bootstrap from env (MODULE_ID, SOCKET_PATH, DATA_DIR).
//! Run: MODULE_ID=x SOCKET_PATH=/tmp/d.sock DATA_DIR=/tmp/d cargo run --example demo-module

use blvm_node::Hash;
use blvm_sdk::module::prelude::*;
use blvm_sdk::module::MigrationContext;
use blvm_sdk::run_module_main;
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

// --- Config ---

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[config(name = "demo")]
pub struct DemoConfig {
    #[serde(default)]
    pub prefix: String,

    #[serde(default)]
    pub max_items: u32,
}

// --- Migrations ---

#[migration(version = 1)]
fn up_initial(ctx: &MigrationContext) -> anyhow::Result<()> {
    ctx.put(b"schema_version", b"1")?;
    Ok(())
}

#[migration(version = 2)]
fn up_add_items_tree(_ctx: &MigrationContext) -> anyhow::Result<()> {
    Ok(())
}

// --- Module: one struct, one impl. Name/config inferred; only migrations explicit. ---

const DATA_TREE: &str = "data";

#[derive(Clone)]
#[module(migrations = ((1, up_initial), (2, up_add_items_tree)))]
pub struct DemoModule {
    #[allow(dead_code)]
    config: DemoConfig,
}

#[module]
impl DemoModule {
    /// Set key=value in the items store.
    #[command]
    fn set(
        &self,
        ctx: &InvocationContext,
        key: String,
        value: String,
    ) -> Result<String, ModuleError> {
        let tree = ctx
            .db()
            .open_tree(DATA_TREE)
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        tree.insert(key.as_bytes(), value.as_bytes())
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        Ok(format!("Set {}={}\n", key, value))
    }

    /// Get value for key from the items store.
    #[command]
    fn get(&self, ctx: &InvocationContext, key: String) -> Result<String, ModuleError> {
        let tree = ctx
            .db()
            .open_tree(DATA_TREE)
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        let value = tree
            .get(key.as_bytes())
            .map_err(|e| ModuleError::Other(e.to_string()))?
            .map(|v| String::from_utf8_lossy(&v).into_owned())
            .unwrap_or_else(|| "<not found>".into());
        Ok(format!("{}={}\n", key, value))
    }

    fn list(&self, ctx: &InvocationContext) -> Result<String, ModuleError> {
        let tree = ctx
            .db()
            .open_tree(DATA_TREE)
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        let items: Vec<String> = tree
            .iter()
            .filter_map(|r| r.ok())
            .map(|(k, v)| {
                format!(
                    "{}={}",
                    String::from_utf8_lossy(&k),
                    String::from_utf8_lossy(&v)
                )
            })
            .collect();
        Ok(if items.is_empty() {
            "(empty)\n".into()
        } else {
            items.join("\n") + "\n"
        })
    }

    /// Delete key from the items store.
    #[command]
    fn delete(&self, ctx: &InvocationContext, key: String) -> Result<String, ModuleError> {
        let tree = ctx
            .db()
            .open_tree(DATA_TREE)
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        tree.remove(key.as_bytes())
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        Ok(format!("Deleted {}\n", key))
    }

    /// Demo: limit command with i32 and bool args.
    #[command]
    fn limit(
        &self,
        _ctx: &InvocationContext,
        count: i32,
        verbose: bool,
    ) -> Result<String, ModuleError> {
        Ok(format!(
            "count={} (i32), verbose={} (bool)\n",
            count, verbose
        ))
    }

    // RPC — name defaults to function name (demo_set, demo_get, demo_list)
    #[rpc_method]
    fn demo_set(
        &self,
        params: &Value,
        db: &Arc<dyn blvm_node::storage::database::Database>,
    ) -> Result<Value, ModuleError> {
        let key = params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModuleError::Other("missing key".into()))?;
        let value = params.get("value").and_then(|v| v.as_str()).unwrap_or("");
        let tree = db
            .open_tree(DATA_TREE)
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        tree.insert(key.as_bytes(), value.as_bytes())
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        Ok(serde_json::json!({ "ok": true, "key": key }))
    }

    #[rpc_method]
    fn demo_get(
        &self,
        params: &Value,
        db: &Arc<dyn blvm_node::storage::database::Database>,
    ) -> Result<Value, ModuleError> {
        let key = params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModuleError::Other("missing key".into()))?;
        let tree = db
            .open_tree(DATA_TREE)
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        let value = tree
            .get(key.as_bytes())
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        Ok(serde_json::json!({
            "key": key,
            "value": value.map(|v| String::from_utf8_lossy(&v).into_owned())
        }))
    }

    #[rpc_method]
    fn demo_list(
        &self,
        _params: &Value,
        db: &Arc<dyn blvm_node::storage::database::Database>,
    ) -> Result<Value, ModuleError> {
        let tree = db
            .open_tree(DATA_TREE)
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        let items: Vec<_> = tree
            .iter()
            .filter_map(|r| r.ok())
            .map(|(k, v)| {
                (
                    String::from_utf8_lossy(&k).into_owned(),
                    String::from_utf8_lossy(&v).into_owned(),
                )
            })
            .collect();
        Ok(serde_json::json!({ "items": items }))
    }

    // Events
    #[on_event(ModuleLoaded)]
    async fn on_module_loaded(&self, module_name: &str, version: &str) -> Result<(), ModuleError> {
        info!("Demo module: {} v{} loaded", module_name, version);
        Ok(())
    }

    #[on_event(NewBlock)]
    async fn on_new_block(&self, block_hash: &Hash, height: u64) -> Result<(), ModuleError> {
        info!(
            "Demo module: new block {:?} at height {}",
            block_hash, height
        );
        Ok(())
    }
}

// --- Entry ---

run_module_main!(DemoModule);
