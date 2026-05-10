//! Hello-module: minimal example demonstrating all extension points
//!
//! Uses run_module_main! — no manual handle_invocation, no manual event loop.
//! Layout: `modules/hello/` with `module.toml`, `config.toml`, binary.
//! Usage: `blvm hello greet [--name NAME]` → node forwards to running module.
//!
//! Run with env: `MODULE_ID=hello SOCKET_PATH=/path/data.sock DATA_DIR=/path/data`
//! Or with clap fallback: `cargo run --example hello-module -- --module-id hello --socket-path /path --data-dir /path`

use blvm_sdk::module::prelude::*;
use blvm_sdk::module::{MigrationContext, ModuleBootstrap, ModuleDb};
use blvm_sdk::run_module;
use clap::Parser;
use serde_json::Value;
use std::path::PathBuf;
use tracing::info;

#[migration(version = 1)]
fn up_initial(ctx: &MigrationContext) -> anyhow::Result<()> {
    ctx.put(b"schema_version", b"1")?;
    Ok(())
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[config(name = "hello")]
pub struct HelloConfig {
    #[config_env]
    pub greeting: String,
}

#[derive(Clone)]
#[module(migrations = ((1, up_initial)))]
pub struct HelloModule {
    #[allow(dead_code)]
    config: HelloConfig,
}

#[module]
impl HelloModule {
    /// Greet someone by name.
    #[command]
    fn greet(&self, _ctx: &InvocationContext, name: Option<String>) -> Result<String, ModuleError> {
        Ok(format!(
            "Hello, {}!\n",
            name.unwrap_or_else(|| "world".into())
        ))
    }

    #[rpc_method(name = "hello_greet")]
    fn hello_greet(
        &self,
        params: &Value,
        _db: &std::sync::Arc<dyn blvm_node::storage::database::Database>,
    ) -> Result<Value, ModuleError> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("world");
        Ok(serde_json::json!({ "message": format!("Hello, {}!", name) }))
    }
}

#[derive(Parser)]
struct Args {
    #[arg(long)]
    module_id: Option<String>,

    #[arg(long)]
    socket_path: Option<PathBuf>,

    #[arg(long)]
    data_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    blvm_node::utils::init_module_logging("hello_module", None);

    let bootstrap = match ModuleBootstrap::from_env() {
        Ok(b) => b,
        Err(_) => {
            let args = Args::parse();
            ModuleBootstrap {
                module_id: args
                    .module_id
                    .unwrap_or_else(|| "hello_standalone".to_string()),
                socket_path: args
                    .socket_path
                    .unwrap_or_else(|| PathBuf::from("data/modules/hello.sock")),
                data_dir: args
                    .data_dir
                    .unwrap_or_else(|| PathBuf::from("data/modules/hello")),
            }
        }
    };

    info!("Hello module starting");

    let db = ModuleDb::open(&bootstrap.data_dir)?;
    db.run_migrations(&[(1, up_initial)])?;

    let config = HelloConfig::load(bootstrap.data_dir.join("config.toml")).unwrap_or_default();
    let module = HelloModule { config };

    run_module! {
        bootstrap: &bootstrap,
        module_name: "hello",
        module: module,
        module_type: HelloModule,
        db: db.as_db(),
    }?;

    Ok(())
}
