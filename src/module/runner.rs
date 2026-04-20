//! Module runner and invocation context.
//!
//! Provides `InvocationContext` for CLI/RPC handlers, `run_async` for sync-over-async CLI,
//! and `run_module` for the unified connect/dispatch/loop lifecycle.

use blvm_node::module::integration::ModuleIntegration;
use blvm_node::module::ipc::protocol::{
    CliSpec, InvocationMessage, InvocationResultMessage, ModuleMessage,
};
use blvm_node::module::traits::{ModuleError, NodeAPI};
use blvm_node::storage::database::Database;
use std::path::Path;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::info;

use crate::module::storage::{DatabaseStorageAdapter, ModuleStorage, ModuleStorageDatabaseBridge};

/// Run an async future from a sync context (e.g. CLI handler).
/// Blocks the current thread and executes the future on the current runtime.
/// Use when `#[command]` methods need to call async APIs.
///
/// When the future only returns `Ok(_)` with no error path, use `Ok::<_, String>(...)` to fix inference.
pub fn run_async<F, T, E>(f: F) -> Result<T, ModuleError>
where
    F: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(f))
        .map_err(|e| ModuleError::Other(e.to_string()))
}

/// Context passed to CLI handlers for database and config access.
///
/// Uses `ModuleStorage` internally; `ctx.db()` returns the database interface for compatibility.
/// When connected to a node, `node_api()` provides access to blockchain data (e.g. get_transaction).
#[derive(Clone)]
pub struct InvocationContext {
    db: Arc<dyn Database>,
    node_api: Option<Arc<dyn NodeAPI>>,
}

impl InvocationContext {
    /// Create a new invocation context from module storage.
    pub fn from_storage(storage: Arc<dyn ModuleStorage>) -> Self {
        let db = Arc::new(ModuleStorageDatabaseBridge::new(storage));
        Self { db, node_api: None }
    }

    /// Create a new invocation context from a database (legacy; wraps in ModuleStorage).
    pub fn new(db: Arc<dyn Database>) -> Self {
        let storage = Arc::new(DatabaseStorageAdapter::new(db));
        Self::from_storage(storage)
    }

    /// Create invocation context with NodeAPI for CLI commands that need blockchain access.
    pub fn with_node_api(db: Arc<dyn Database>, node_api: Arc<dyn NodeAPI>) -> Self {
        let storage = Arc::new(DatabaseStorageAdapter::new(db));
        Self {
            db: Arc::new(ModuleStorageDatabaseBridge::new(storage)),
            node_api: Some(node_api),
        }
    }

    /// Get the module's database.
    pub fn db(&self) -> &Arc<dyn Database> {
        &self.db
    }

    /// Get NodeAPI when connected to node (for fetch-by-txid, etc.).
    pub fn node_api(&self) -> Option<Arc<dyn NodeAPI>> {
        self.node_api.clone()
    }
}

/// Run a module with automatic connect, registration, event subscription, and dispatch.
///
/// Handles the full lifecycle: connect → register CLI/RPC/events → loop (invocations + events) → unload on disconnect.
pub async fn run_module<M, C, F, FE, Fut>(
    socket_path: impl AsRef<Path>,
    module_id: &str,
    module_name: &str,
    version: &str,
    cli_spec: CliSpec,
    rpc_methods: &[&str],
    event_types: Vec<blvm_node::module::traits::EventType>,
    dispatch: F,
    on_event: FE,
    module: M,
    cli: C,
    db: Arc<dyn Database>,
) -> Result<(), ModuleError>
where
    F: Fn(InvocationMessage, InvocationContext, &M, &C) -> InvocationResultMessage,
    FE: Fn(blvm_node::module::ipc::protocol::EventMessage, &M, &InvocationContext) -> Fut,
    Fut: std::future::Future<Output = Result<(), ModuleError>> + Send,
{
    let socket_path = socket_path.as_ref().to_path_buf();

    match ModuleIntegration::connect(
        socket_path.clone(),
        module_id.to_string(),
        module_name.to_string(),
        version.to_string(),
        Some(cli_spec),
    )
    .await
    {
        Ok(mut integration) => {
            info!("Connected to node");

            let node_api = integration.node_api();
            for method in rpc_methods {
                node_api
                    .register_rpc_endpoint((*method).to_string(), String::new())
                    .await?;
            }

            integration.subscribe_events(event_types).await?;

            let mut event_rx = integration.event_receiver();
            let invocation_rx = integration.invocation_receiver().ok_or_else(|| {
                ModuleError::IpcError(
                    "Invocation receiver not available for this module integration".to_string(),
                )
            })?;
            let ctx = InvocationContext::with_node_api(db, node_api);

            loop {
                tokio::select! {
                    msg = event_rx.recv() => {
                        if let Ok(ModuleMessage::Event(e)) = msg {
                            let _ = on_event(e, &module, &ctx).await;
                        }
                    }
                    inv = invocation_rx.recv() => {
                        if let Some((invocation, result_tx)) = inv {
                            let result = dispatch(invocation, ctx.clone(), &module, &cli);
                            let _ = result_tx.send(result);
                        } else {
                            info!("Invocation channel closed, module unloading");
                            break;
                        }
                    }
                    _ = sleep(Duration::from_secs(30)) => {
                        info!("Module running");
                    }
                }
            }
        }
        Err(e) => {
            info!("Node not running, standalone mode: {}", e);
            loop {
                sleep(Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

/// Run a module where (module, cli) are created after connect.
///
/// Use when the module depends on NodeAPI (e.g. datum creates DatumServer with node_api).
/// The setup receives (node_api, db, data_dir) and returns (module, cli).
pub async fn run_module_with_setup<M, C, F, FE, Fut, FSetup, FutSetup>(
    socket_path: impl AsRef<Path>,
    module_id: &str,
    module_name: &str,
    version: &str,
    cli_spec: CliSpec,
    rpc_methods: &[&str],
    event_types: Vec<blvm_node::module::traits::EventType>,
    dispatch: F,
    on_event: FE,
    setup: FSetup,
    db: Arc<dyn Database>,
    data_dir: &Path,
) -> Result<(), ModuleError>
where
    F: Fn(InvocationMessage, InvocationContext, &M, &C) -> InvocationResultMessage,
    FE: Fn(blvm_node::module::ipc::protocol::EventMessage, &M, &InvocationContext) -> Fut,
    Fut: std::future::Future<Output = Result<(), ModuleError>> + Send,
    FSetup: Fn(Arc<dyn NodeAPI>, Arc<dyn Database>, &Path) -> FutSetup,
    FutSetup: std::future::Future<Output = Result<(M, C), ModuleError>> + Send,
{
    let socket_path = socket_path.as_ref().to_path_buf();

    match ModuleIntegration::connect(
        socket_path.clone(),
        module_id.to_string(),
        module_name.to_string(),
        version.to_string(),
        Some(cli_spec),
    )
    .await
    {
        Ok(mut integration) => {
            info!("Connected to node");

            let node_api = integration.node_api();
            for method in rpc_methods {
                node_api
                    .register_rpc_endpoint((*method).to_string(), String::new())
                    .await?;
            }

            integration.subscribe_events(event_types).await?;

            let (module, cli) = setup(node_api.clone(), Arc::clone(&db), data_dir).await?;
            let module = Arc::new(module);

            let mut event_rx = integration.event_receiver();
            let invocation_rx = integration.invocation_receiver().ok_or_else(|| {
                ModuleError::IpcError(
                    "Invocation receiver not available for this module integration".to_string(),
                )
            })?;
            let ctx = InvocationContext::with_node_api(Arc::clone(&db), node_api);

            loop {
                tokio::select! {
                    msg = event_rx.recv() => {
                        if let Ok(ModuleMessage::Event(e)) = msg {
                            let _ = on_event(e, &*module, &ctx).await;
                        }
                    }
                    inv = invocation_rx.recv() => {
                        if let Some((invocation, result_tx)) = inv {
                            let result = dispatch(invocation, ctx.clone(), &*module, &cli);
                            let _ = result_tx.send(result);
                        } else {
                            info!("Invocation channel closed, module unloading");
                            break;
                        }
                    }
                    _ = sleep(Duration::from_secs(30)) => {
                        info!("Module running");
                    }
                }
            }
        }
        Err(e) => {
            info!("Node not running, standalone mode: {}", e);
            loop {
                sleep(Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

/// Run a module with optional on_connect (setup) and on_tick (periodic) callbacks.
pub async fn run_module_with_tick<M, C, F, FE, Fut, FConnect, FutConnect, FTick, FutTick>(
    socket_path: impl AsRef<Path>,
    module_id: &str,
    module_name: &str,
    version: &str,
    cli_spec: CliSpec,
    rpc_methods: &[&str],
    event_types: Vec<blvm_node::module::traits::EventType>,
    dispatch: F,
    on_event: FE,
    on_connect: Option<FConnect>,
    on_tick: Option<FTick>,
    module: M,
    cli: C,
    db: Arc<dyn Database>,
) -> Result<(), ModuleError>
where
    F: Fn(InvocationMessage, InvocationContext, &M, &C) -> InvocationResultMessage,
    FE: Fn(blvm_node::module::ipc::protocol::EventMessage, &M, &InvocationContext) -> Fut,
    Fut: std::future::Future<Output = Result<(), ModuleError>> + Send,
    FConnect: Fn(Arc<dyn NodeAPI>, Arc<dyn Database>) -> FutConnect,
    FutConnect: std::future::Future<Output = Result<(), ModuleError>> + Send,
    FTick: Fn(Arc<dyn NodeAPI>, Arc<dyn Database>) -> FutTick,
    FutTick: std::future::Future<Output = ()> + Send,
{
    let socket_path = socket_path.as_ref().to_path_buf();

    match ModuleIntegration::connect(
        socket_path.clone(),
        module_id.to_string(),
        module_name.to_string(),
        version.to_string(),
        Some(cli_spec),
    )
    .await
    {
        Ok(mut integration) => {
            info!("Connected to node");

            let node_api = integration.node_api();
            for method in rpc_methods {
                node_api
                    .register_rpc_endpoint((*method).to_string(), String::new())
                    .await?;
            }

            integration.subscribe_events(event_types).await?;

            if let Some(ref connect) = on_connect {
                connect(node_api.clone(), Arc::clone(&db)).await?;
            }

            let mut event_rx = integration.event_receiver();
            let invocation_rx = integration.invocation_receiver().ok_or_else(|| {
                ModuleError::IpcError(
                    "Invocation receiver not available for this module integration".to_string(),
                )
            })?;
            let ctx = InvocationContext::with_node_api(Arc::clone(&db), Arc::clone(&node_api));

            loop {
                tokio::select! {
                    msg = event_rx.recv() => {
                        if let Ok(ModuleMessage::Event(e)) = msg {
                            let _ = on_event(e, &module, &ctx).await;
                        }
                    }
                    inv = invocation_rx.recv() => {
                        if let Some((invocation, result_tx)) = inv {
                            let result = dispatch(invocation, ctx.clone(), &module, &cli);
                            let _ = result_tx.send(result);
                        } else {
                            info!("Invocation channel closed, module unloading");
                            break;
                        }
                    }
                    _ = sleep(Duration::from_secs(30)) => {
                        if let Some(ref tick) = on_tick {
                            tick(node_api.clone(), Arc::clone(&db)).await;
                        }
                        info!("Module running");
                    }
                }
            }
        }
        Err(e) => {
            info!("Node not running, standalone mode: {}", e);
            loop {
                sleep(Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}
