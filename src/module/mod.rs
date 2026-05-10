//! Module Development APIs
//!
//! Provides APIs for developing modules that extend blvm-node.
//!
//! This module re-exports the necessary types and traits from `blvm-node` to provide
//! a clean, developer-friendly interface for module development.
//!
//! The `wasm` submodule (when `wasm-modules` is enabled) is node-independent:
//! embedders implement `WasmStorage`/`WasmTree` to bridge to their storage.

#[cfg(feature = "node")]
pub mod bootstrap;
#[cfg(feature = "node")]
pub mod cli_args;
#[cfg(feature = "node")]
pub mod database;
#[cfg(feature = "node")]
pub mod module_db;
#[cfg(feature = "node")]
pub mod storage;

#[cfg(feature = "wasm-modules")]
pub mod wasm;

/// Minimal module entry point. Expands to full main with bootstrap, migrations, config load, run_module.
///
/// **Single-arg form** (preferred when `#[module]` has `config` and `migrations`):
/// ```ignore
/// run_module_main!(DemoModule);
/// ```
///
/// **Explicit form** (when ModuleMeta is not implemented):
/// ```ignore
/// run_module_main!("demo", DemoModule, DemoConfig, migrations!(1 => up_initial, 2 => up_add_items_tree));
/// ```
#[cfg(feature = "node")]
#[macro_export]
macro_rules! run_module_main {
    ($module_type:ty) => {
        #[tokio::main]
        async fn main() -> Result<(), Box<dyn std::error::Error>> {
            let bootstrap = $crate::module::ModuleBootstrap::from_env()?;
            blvm_node::utils::init_module_logging(
                <$module_type as $crate::module::ModuleMeta>::MODULE_NAME
                    .replace('-', "_")
                    .as_str(),
                None,
            );
            let db = $crate::module::ModuleDb::open(&bootstrap.data_dir)?;
            db.run_migrations(<$module_type as $crate::module::ModuleMeta>::migrations())?;
            let config = <<$module_type as $crate::module::ModuleMeta>::Config>::load(
                bootstrap.data_dir.join("config.toml"),
            )
            .unwrap_or_default();
            let module = <$module_type as $crate::module::ModuleMeta>::__module_new(config);
            $crate::run_module! {
                bootstrap: &bootstrap,
                module_name: <$module_type as $crate::module::ModuleMeta>::MODULE_NAME,
                module: module,
                module_type: $module_type,
                db: db.as_db(),
            }?;
            Ok(())
        }
    };
    (
        $module_name:expr,
        $module_type:ty,
        $config_type:ty,
        $migrations:expr,
    ) => {
        #[tokio::main]
        async fn main() -> Result<(), Box<dyn std::error::Error>> {
            blvm_node::utils::init_module_logging($module_name.replace('-', "_").as_str(), None);
            let bootstrap = $crate::module::ModuleBootstrap::from_env()?;
            let db = $crate::module::ModuleDb::open(&bootstrap.data_dir)?;
            db.run_migrations($migrations)?;
            let config =
                <$config_type>::load(bootstrap.data_dir.join("config.toml")).unwrap_or_default();
            let module = <$module_type>::__module_new(config);
            $crate::run_module! {
                bootstrap: &bootstrap,
                module_name: $module_name,
                module: module,
                module_type: $module_type,
                db: db.as_db(),
            }?;
            Ok(())
        }
    };
}

/// Collect migrations for `run_migrations`. Sugar for `&[(1, up_initial), (2, up_add_cache), ...]`.
///
/// # Example
/// ```ignore
/// run_migrations(&db, migrations!(1 => up_initial, 2 => up_add_cache))?;
/// ```
#[cfg(feature = "node")]
#[macro_export]
macro_rules! migrations {
    ($($v:literal => $up:ident),* $(,)?) => {
        &[$(($v, $up as $crate::module::MigrationUp)),*]
    };
}

/// Run a module with automatic connect, CLI/RPC/event registration, and dispatch.
///
/// Replaces manual main-loop boilerplate. CLI spec, RPC methods, and event types are
/// auto-discovered from #[command], #[rpc_methods], and #[event_handlers]. On unload
/// (invocation channel closed), the module exits cleanly.
///
/// # Example (unified module — preferred)
/// ```ignore
/// let bootstrap = ModuleBootstrap::from_env()?;
/// run_module! {
///     bootstrap: &bootstrap,
///     module_name: "demo",
///     module: DemoModule { config },
///     module_type: DemoModule,
///     db,
/// }
/// ```
///
/// # Example (explicit args — legacy)
/// ```ignore
/// run_module! {
///     socket_path: args.socket_path.clone(),
///     module_id: &args.module_id,
///     module_name: "demo-module",
///     version: env!("CARGO_PKG_VERSION"),
///     cli: DemoCli,
///     cli_type: DemoCli,
///     module_type: DemoModule,
///     module: DemoModule { config: config.clone() },
///     db,
/// }
/// ```
#[cfg(feature = "node")]
#[macro_export]
macro_rules! run_module {
    (
        bootstrap: $bootstrap:expr,
        module_name: $module_name:expr,
        module: $module:expr,
        module_type: $module_type:ty,
        db: $db:expr,
    ) => {{
        let __bootstrap = $bootstrap;
        let __module = $module;
        $crate::run_module! {
            socket_path: __bootstrap.socket_path.clone(),
            module_id: &__bootstrap.module_id,
            module_name: $module_name,
            version: env!("CARGO_PKG_VERSION"),
            cli: __module.clone(),
            cli_type: $module_type,
            module_type: $module_type,
            module: __module,
            db: $db,
        }
    }};
    (
        bootstrap: $bootstrap:expr,
        module_name: $module_name:expr,
        module_type: $module_type:ty,
        cli_type: $cli_type:ty,
        db: $db:expr,
        setup: $setup:expr,
        event_types: $event_types:expr,
    ) => {{
        let __bootstrap = $bootstrap;
        let __db = Arc::clone(&$db);
        $crate::run_module! {
            socket_path: __bootstrap.socket_path.clone(),
            module_id: &__bootstrap.module_id,
            module_name: $module_name,
            version: env!("CARGO_PKG_VERSION"),
            module_type: $module_type,
            cli_type: $cli_type,
            db: __db,
            setup: $setup,
            event_types: $event_types,
            on_event: |e, m: &$module_type, ctx| {
                let m = m.clone();
                let ctx = ctx.clone();
                async move { m.dispatch_event(e, &ctx).await }
            },
            data_dir: __bootstrap.data_dir.as_path(),
        }
    }};
    (
        bootstrap: $bootstrap:expr,
        module_name: $module_name:expr,
        module_type: $module_type:ty,
        cli_type: $cli_type:ty,
        db: $db:expr,
        setup: $setup:expr,
        event_types: $event_types:expr,
        on_event: $on_event:expr,
    ) => {{
        let __bootstrap = $bootstrap;
        let __db = Arc::clone(&$db);
        $crate::run_module! {
            socket_path: __bootstrap.socket_path.clone(),
            module_id: &__bootstrap.module_id,
            module_name: $module_name,
            version: env!("CARGO_PKG_VERSION"),
            module_type: $module_type,
            cli_type: $cli_type,
            db: __db,
            setup: $setup,
            event_types: $event_types,
            on_event: $on_event,
            data_dir: __bootstrap.data_dir.as_path(),
        }
    }};
    (
        socket_path: $socket_path:expr,
        module_id: $module_id:expr,
        module_name: $module_name:expr,
        version: $version:expr,
        module_type: $module_type:ty,
        cli_type: $cli_type:ty,
        db: $db:expr,
        setup: $setup:expr,
        event_types: $event_types:expr,
        on_event: $on_event:expr,
        data_dir: $data_dir:expr,
    ) => {{
        use blvm_node::module::ipc::protocol::{
            InvocationMessage, InvocationResultMessage, InvocationResultPayload, InvocationType,
        };
        use std::sync::Arc;
        use $crate::module::runner::{run_module_with_setup, InvocationContext};

        let db = Arc::clone(&$db);

        let dispatch = |invocation: InvocationMessage,
                        ctx: InvocationContext,
                        module: &$module_type,
                        cli: &$cli_type| {
            let (success, payload, error) = match &invocation.invocation_type {
                InvocationType::Cli { subcommand, args } => {
                    let args: Vec<String> = args.clone();
                    match cli.dispatch_cli(&ctx, subcommand, &args) {
                        Ok(stdout) => (
                            true,
                            Some(InvocationResultPayload::Cli {
                                stdout,
                                stderr: String::new(),
                                exit_code: 0,
                            }),
                            None,
                        ),
                        Err(e) => (false, None, Some(e.to_string())),
                    }
                }
                InvocationType::Rpc { method, params } => {
                    let db_ref = ctx.db();
                    match module.dispatch_rpc(method, params, db_ref) {
                        Ok(v) => (true, Some(InvocationResultPayload::Rpc(v)), None),
                        Err(e) => (false, None, Some(e.to_string())),
                    }
                }
            };
            InvocationResultMessage {
                correlation_id: invocation.correlation_id,
                success,
                payload,
                error,
            }
        };

        let rpc_names = <$module_type>::rpc_method_names();
        let cli_spec = <$cli_type>::cli_spec();

        run_module_with_setup(
            $socket_path,
            $module_id,
            $module_name,
            $version,
            cli_spec,
            rpc_names.as_slice(),
            $event_types,
            dispatch,
            $on_event,
            $setup,
            db,
            $data_dir,
        )
        .await
    }};
    (
        socket_path: $socket_path:expr,
        module_id: $module_id:expr,
        module_name: $module_name:expr,
        version: $version:expr,
        cli: $cli:expr,
        cli_type: $cli_type:ty,
        module_type: $module_type:ty,
        module: $module:expr,
        db: $db:expr,
    ) => {{
        use blvm_node::module::ipc::protocol::{
            InvocationMessage, InvocationResultMessage, InvocationResultPayload, InvocationType,
        };
        use std::sync::Arc;
        use $crate::module::runner::{run_module as run_module_fn, InvocationContext};

        let cli = $cli;
        let module = Arc::new($module);
        let db = Arc::clone(&$db);

        let dispatch = |invocation: InvocationMessage,
                        ctx: InvocationContext,
                        module: &Arc<$module_type>,
                        cli: &$cli_type| {
            let (success, payload, error) = match &invocation.invocation_type {
                InvocationType::Cli { subcommand, args } => {
                    let args: Vec<String> = args.clone();
                    match cli.dispatch_cli(&ctx, subcommand, &args) {
                        Ok(stdout) => (
                            true,
                            Some(InvocationResultPayload::Cli {
                                stdout,
                                stderr: String::new(),
                                exit_code: 0,
                            }),
                            None,
                        ),
                        Err(e) => (false, None, Some(e.to_string())),
                    }
                }
                InvocationType::Rpc { method, params } => {
                    let db_ref = ctx.db();
                    match module.dispatch_rpc(method, params, db_ref) {
                        Ok(v) => (true, Some(InvocationResultPayload::Rpc(v)), None),
                        Err(e) => (false, None, Some(e.to_string())),
                    }
                }
            };
            InvocationResultMessage {
                correlation_id: invocation.correlation_id,
                success,
                payload,
                error,
            }
        };

        let rpc_names = <$module_type>::rpc_method_names();
        let cli_spec = <$cli_type>::cli_spec();

        run_module_fn(
            $socket_path,
            $module_id,
            $module_name,
            $version,
            cli_spec,
            rpc_names.as_slice(),
            <$module_type>::event_types(),
            dispatch,
            |e, m: &Arc<$module_type>, ctx: &InvocationContext| {
                let m = std::sync::Arc::clone(m);
                let ctx = ctx.clone();
                async move { m.dispatch_event(e, &ctx).await }
            },
            module,
            cli,
            db,
        )
        .await
    }};
    (
        bootstrap: $bootstrap:expr,
        module_name: $module_name:expr,
        module: $module:expr,
        module_type: $module_type:ty,
        db: $db:expr,
        on_connect: $on_connect:expr,
        on_tick: $on_tick:expr,
    ) => {{
        let __bootstrap = $bootstrap;
        let __module = $module;
        $crate::run_module! {
            socket_path: __bootstrap.socket_path.clone(),
            module_id: &__bootstrap.module_id,
            module_name: $module_name,
            version: env!("CARGO_PKG_VERSION"),
            cli: __module.clone(),
            cli_type: $module_type,
            module_type: $module_type,
            module: __module,
            db: $db,
            on_connect: Some($on_connect),
            on_tick: Some($on_tick),
        }
    }};
    (
        socket_path: $socket_path:expr,
        module_id: $module_id:expr,
        module_name: $module_name:expr,
        version: $version:expr,
        cli: $cli:expr,
        cli_type: $cli_type:ty,
        module_type: $module_type:ty,
        module: $module:expr,
        db: $db:expr,
        on_connect: $on_connect:expr,
        on_tick: $on_tick:expr,
    ) => {{
        use blvm_node::module::ipc::protocol::{
            InvocationMessage, InvocationResultMessage, InvocationResultPayload, InvocationType,
        };
        use std::sync::Arc;
        use $crate::module::runner::{run_module_with_tick, InvocationContext};

        let cli = $cli;
        let module = Arc::new($module);
        let db = Arc::clone(&$db);

        let dispatch = |invocation: InvocationMessage,
                        ctx: InvocationContext,
                        module: &Arc<$module_type>,
                        cli: &$cli_type| {
            let (success, payload, error) = match &invocation.invocation_type {
                InvocationType::Cli { subcommand, args } => {
                    let args: Vec<String> = args.clone();
                    match cli.dispatch_cli(&ctx, subcommand, &args) {
                        Ok(stdout) => (
                            true,
                            Some(InvocationResultPayload::Cli {
                                stdout,
                                stderr: String::new(),
                                exit_code: 0,
                            }),
                            None,
                        ),
                        Err(e) => (false, None, Some(e.to_string())),
                    }
                }
                InvocationType::Rpc { method, params } => {
                    let db_ref = ctx.db();
                    match module.dispatch_rpc(method, params, db_ref) {
                        Ok(v) => (true, Some(InvocationResultPayload::Rpc(v)), None),
                        Err(e) => (false, None, Some(e.to_string())),
                    }
                }
            };
            InvocationResultMessage {
                correlation_id: invocation.correlation_id,
                success,
                payload,
                error,
            }
        };

        let rpc_names = <$module_type>::rpc_method_names();
        let cli_spec = <$cli_type>::cli_spec();

        run_module_with_tick(
            $socket_path,
            $module_id,
            $module_name,
            $version,
            cli_spec,
            rpc_names.as_slice(),
            <$module_type>::event_types(),
            dispatch,
            |e, m: &Arc<$module_type>, ctx: &InvocationContext| {
                let m = std::sync::Arc::clone(m);
                let ctx = ctx.clone();
                async move { m.dispatch_event(e, &ctx).await }
            },
            $on_connect,
            $on_tick,
            module,
            cli,
            db,
        )
        .await
    }};
}

/// Register RPC methods with the node on connect.
///
/// Call this after connecting via `ModuleIntegration::connect`. Pass the node API and
/// the method names (as string literals) that were registered with `#[rpc_method(name = "...")]`.
///
/// # Example
/// ```ignore
/// let integration = ModuleIntegration::connect(...).await?;
/// let node_api = integration.node_api();
/// register_rpc_methods!(node_api, "hello_greet", "hello_status").await?;
/// ```
#[macro_export]
macro_rules! register_rpc_methods {
    ($api:expr, $($method:expr),* $(,)?) => {
        async {
            let api = $api;
            $(
                api.register_rpc_endpoint($method.to_string(), String::new()).await?;
            )*
            Ok::<(), blvm_node::module::traits::ModuleError>(())
        }
    };
}

#[cfg(feature = "node")]
pub mod ipc;
#[cfg(feature = "node")]
pub mod manifest;
#[cfg(feature = "node")]
pub mod prelude;
#[cfg(feature = "node")]
pub mod runner;
#[cfg(feature = "node")]
pub mod security;
#[cfg(feature = "node")]
pub mod traits;

// Re-export main types for convenience (requires node)
#[cfg(feature = "node")]
pub use bootstrap::{ModuleBootstrap, ModuleConfig};
#[cfg(feature = "node")]
pub use database::{
    open_module_db, run_migrations, run_migrations_down, run_migrations_with_down, Migration,
    MigrationContext, MigrationDown, MigrationUp,
};
#[cfg(feature = "node")]
pub use ipc::client::ModuleIpcClient;
#[cfg(feature = "node")]
pub use ipc::protocol::*;
#[cfg(feature = "node")]
pub use manifest::ModuleManifest;
#[cfg(feature = "node")]
pub use module_db::ModuleDb;
#[cfg(feature = "node")]
pub use runner::{
    run_async, run_module, run_module_with_setup, run_module_with_tick, InvocationContext,
};
#[cfg(feature = "node")]
pub use security::{Permission, PermissionSet};
#[cfg(feature = "node")]
pub use storage::{DatabaseStorageAdapter, ModuleStorage, ModuleStorageDatabaseBridge, ModuleTree};
#[cfg(feature = "node")]
pub use traits::*;
