//! Module runner and invocation context.
//!
//! Provides `InvocationContext` for CLI/RPC handlers, `run_async` for sync-over-async CLI,
//! and `run_module` for the unified connect/dispatch/loop lifecycle.

use blvm_node::module::integration::ModuleIntegration;
use blvm_node::module::ipc::protocol::{
    CliSpec, InvocationMessage, InvocationResultMessage, InvocationResultPayload, InvocationType,
    ModuleMessage,
};
use blvm_node::module::traits::{ModuleError, NodeAPI};
use blvm_node::storage::database::Database;
use std::collections::VecDeque;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::time::{Duration, sleep};
use tracing::info;

use crate::module::timers::{Exec, TimerRegistry, sdk_tick_spec};

use crate::module::storage::{DatabaseStorageAdapter, ModuleStorage, ModuleStorageDatabaseBridge};

/// Core RPC allowlist: registered with [`NodeAPI::register_core_rpc_override`] in module setup,
/// not via [`NodeAPI::register_rpc_endpoint`] (the RPC server rejects extension registration for them).
fn is_overrideable_core_rpc_method(method: &str) -> bool {
    blvm_node::rpc::methods::OVERRIDABLE_CORE_RPC_METHODS.contains(&method)
}

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
    timers: Option<Arc<TimerRegistry>>,
}

impl InvocationContext {
    /// Create a new invocation context from module storage.
    pub fn from_storage(storage: Arc<dyn ModuleStorage>) -> Self {
        let db = Arc::new(ModuleStorageDatabaseBridge::new(storage));
        Self {
            db,
            node_api: None,
            timers: None,
        }
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
            timers: None,
        }
    }

    pub(crate) fn with_registry(mut self, timers: Arc<TimerRegistry>) -> Self {
        self.timers = Some(timers);
        self
    }

    /// Builder bound to this module's node connection and timer registry.
    pub fn timers(&self) -> crate::module::timers::TimerBuilder {
        match (&self.node_api, &self.timers) {
            (Some(api), Some(registry)) => crate::module::timers::TimerBuilder::with_runtime(
                Arc::clone(api),
                Arc::clone(registry),
            ),
            _ => crate::module::timers::TimerBuilder::new(),
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
#[allow(clippy::too_many_arguments)] // Explicit wiring for embedders; splitting would obscure the lifecycle.
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
                if is_overrideable_core_rpc_method(method) {
                    continue;
                }
                node_api
                    .register_rpc_endpoint((*method).to_string(), String::new())
                    .await?;
            }

            integration.subscribe_events(event_types).await?;

            let event_rx = integration.event_receiver();
            let invocation_rx = integration.invocation_receiver().ok_or_else(|| {
                ModuleError::IpcError(
                    "Invocation receiver not available for this module integration".to_string(),
                )
            })?;
            let timers = TimerRegistry::new();
            let ctx =
                InvocationContext::with_node_api(db, node_api).with_registry(Arc::clone(&timers));
            module_loop(
                event_rx,
                invocation_rx,
                &module,
                &cli,
                ctx,
                dispatch,
                on_event,
                None,
                timers,
            )
            .await;
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
#[allow(clippy::too_many_arguments)]
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
                if is_overrideable_core_rpc_method(method) {
                    continue;
                }
                node_api
                    .register_rpc_endpoint((*method).to_string(), String::new())
                    .await?;
            }

            integration.subscribe_events(event_types).await?;

            let (module, cli) = setup(node_api.clone(), Arc::clone(&db), data_dir).await?;
            let module = Arc::new(module);

            let event_rx = integration.event_receiver();
            let invocation_rx = integration.invocation_receiver().ok_or_else(|| {
                ModuleError::IpcError(
                    "Invocation receiver not available for this module integration".to_string(),
                )
            })?;
            let timers = TimerRegistry::new();
            let ctx = InvocationContext::with_node_api(Arc::clone(&db), node_api)
                .with_registry(Arc::clone(&timers));
            module_loop(
                event_rx,
                invocation_rx,
                &*module,
                &cli,
                ctx,
                dispatch,
                on_event,
                None,
                timers,
            )
            .await;
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

/// Like [`run_module_with_setup`], but setup also returns a `ModuleAPI` for IPC forwarding.
#[allow(clippy::too_many_arguments)]
pub async fn run_module_with_setup_and_api<M, C, F, FE, Fut, FSetup, FutSetup>(
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
    FutSetup: std::future::Future<
            Output = Result<
                (
                    M,
                    C,
                    Arc<dyn blvm_node::module::inter_module::api::ModuleAPI>,
                ),
                ModuleError,
            >,
        > + Send,
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
                if is_overrideable_core_rpc_method(method) {
                    continue;
                }
                node_api
                    .register_rpc_endpoint((*method).to_string(), String::new())
                    .await?;
            }

            integration.subscribe_events(event_types).await?;

            let (module, cli, module_api) =
                setup(node_api.clone(), Arc::clone(&db), data_dir).await?;
            let module = Arc::new(module);
            let module_api = Arc::clone(&module_api);

            if let Err(e) = node_api.register_module_api(module_api.clone()).await {
                return Err(ModuleError::Other(format!(
                    "Failed to register module API descriptor: {e}"
                )));
            }
            info!("Module API descriptor registered with node");

            let event_rx = integration.event_receiver();
            let invocation_rx = integration.invocation_receiver().ok_or_else(|| {
                ModuleError::IpcError(
                    "Invocation receiver not available for this module integration".to_string(),
                )
            })?;
            let timers = TimerRegistry::new();
            let ctx = InvocationContext::with_node_api(Arc::clone(&db), node_api)
                .with_registry(Arc::clone(&timers));
            module_loop(
                event_rx,
                invocation_rx,
                &*module,
                &cli,
                ctx,
                dispatch,
                on_event,
                Some(module_api),
                timers,
            )
            .await;
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
#[allow(clippy::too_many_arguments)]
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
    FTick: Fn(Arc<dyn NodeAPI>, Arc<dyn Database>) -> FutTick + Send + Sync + 'static,
    FutTick: std::future::Future<Output = ()> + Send + 'static,
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
                if is_overrideable_core_rpc_method(method) {
                    continue;
                }
                node_api
                    .register_rpc_endpoint((*method).to_string(), String::new())
                    .await?;
            }

            integration.subscribe_events(event_types).await?;

            if let Some(ref connect) = on_connect {
                connect(node_api.clone(), Arc::clone(&db)).await?;
            }

            let event_rx = integration.event_receiver();
            let invocation_rx = integration.invocation_receiver().ok_or_else(|| {
                ModuleError::IpcError(
                    "Invocation receiver not available for this module integration".to_string(),
                )
            })?;
            let timers = TimerRegistry::new();
            if let Some(on_tick) = on_tick {
                register_sdk_tick(&node_api, &db, &timers, on_tick).await;
            }
            let ctx = InvocationContext::with_node_api(Arc::clone(&db), Arc::clone(&node_api))
                .with_registry(Arc::clone(&timers));
            module_loop(
                event_rx,
                invocation_rx,
                &module,
                &cli,
                ctx,
                dispatch,
                on_event,
                None,
                timers,
            )
            .await;
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

async fn register_sdk_tick<FTick, FutTick>(
    node_api: &Arc<dyn NodeAPI>,
    db: &Arc<dyn Database>,
    timers: &TimerRegistry,
    on_tick: FTick,
) where
    FTick: Fn(Arc<dyn NodeAPI>, Arc<dyn Database>) -> FutTick + Send + Sync + 'static,
    FutTick: std::future::Future<Output = ()> + Send + 'static,
{
    use blvm_node::module::timers::types::{AckMode, CreateMode, TimerAck};

    let info = match node_api
        .timer_create(sdk_tick_spec(), CreateMode::Ensure)
        .await
    {
        Ok(info) => info,
        Err(err) => {
            info!("sdk tick was not registered: {err}");
            return;
        }
    };
    let api = Arc::clone(node_api);
    let db = Arc::clone(db);
    let tick = Arc::new(on_tick);
    timers.register(
        info.timer_id,
        Exec::Serial,
        AckMode::Completion,
        Arc::new(move |_fire| {
            let api = Arc::clone(&api);
            let db = Arc::clone(&db);
            let tick = Arc::clone(&tick);
            Box::pin(async move {
                tick(api, db).await;
                info!("Module running");
                Ok(TimerAck::Continue)
            })
        }),
    );
}

enum LoopWake {
    SerialDone,
    Event(Box<Result<ModuleMessage, tokio::sync::broadcast::error::RecvError>>),
    Invocation(Option<(InvocationMessage, oneshot::Sender<InvocationResultMessage>)>),
}

/// One module loop. A `Serial` timer is awaited here. `Task`, `Blocking`, and
/// `Dedicated` fires are started without waiting, including while a serial handler runs.
/// Events that arrive during a serial handler wait until it returns.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn module_loop<M, C, F, FE, Fut>(
    mut event_rx: tokio::sync::broadcast::Receiver<ModuleMessage>,
    invocation_rx: &mut tokio::sync::mpsc::Receiver<(
        InvocationMessage,
        oneshot::Sender<InvocationResultMessage>,
    )>,
    module: &M,
    cli: &C,
    ctx: InvocationContext,
    dispatch: F,
    on_event: FE,
    module_api: Option<Arc<dyn blvm_node::module::inter_module::api::ModuleAPI>>,
    timers: Arc<TimerRegistry>,
) where
    F: Fn(InvocationMessage, InvocationContext, &M, &C) -> InvocationResultMessage,
    FE: Fn(blvm_node::module::ipc::protocol::EventMessage, &M, &InvocationContext) -> Fut,
    Fut: std::future::Future<Output = Result<(), ModuleError>> + Send,
{
    let mut queued_events = VecDeque::new();
    let mut queued_invocations = VecDeque::new();
    let mut serial: Option<Pin<Box<dyn std::future::Future<Output = ()> + Send>>> = None;
    let mut closing = false;

    loop {
        if serial.is_none() {
            if let Some(event) = queued_events.pop_front() {
                let _ = on_event(event, module, &ctx).await;
                continue;
            }
            if let Some((invocation, result_tx)) = queued_invocations.pop_front() {
                if let Some(job) = dispatch_invocation(
                    invocation,
                    result_tx,
                    module,
                    cli,
                    &ctx,
                    &dispatch,
                    module_api.as_ref(),
                    &timers,
                )
                .await
                {
                    serial = Some(job);
                }
                continue;
            }
            if closing {
                info!("Invocation channel closed, module unloading");
                break;
            }
        }

        if let Some(mut job) = serial.take() {
            let wake = tokio::select! {
                _ = &mut job => LoopWake::SerialDone,
                msg = event_rx.recv() => LoopWake::Event(Box::new(msg)),
                inv = invocation_rx.recv() => LoopWake::Invocation(inv),
            };
            if !matches!(wake, LoopWake::SerialDone) {
                serial = Some(job);
            }
            match wake {
                LoopWake::SerialDone => {}
                LoopWake::Event(msg) => {
                    if let Ok(ModuleMessage::Event(event)) = *msg {
                        queued_events.push_back(event);
                    }
                }
                LoopWake::Invocation(Some((invocation, result_tx))) => {
                    if detached_timer(&timers, &invocation) {
                        if let InvocationType::Timer(fire) = invocation.invocation_type {
                            timers.start_detached(fire, invocation.correlation_id, result_tx);
                        }
                    } else {
                        queued_invocations.push_back((invocation, result_tx));
                    }
                }
                LoopWake::Invocation(None) => closing = true,
            }
            continue;
        }

        tokio::select! {
            msg = event_rx.recv() => {
                if let Ok(ModuleMessage::Event(event)) = msg {
                    let _ = on_event(event, module, &ctx).await;
                }
            }
            inv = invocation_rx.recv() => {
                match inv {
                    Some((invocation, result_tx)) => {
                        if let Some(job) = dispatch_invocation(
                            invocation,
                            result_tx,
                            module,
                            cli,
                            &ctx,
                            &dispatch,
                            module_api.as_ref(),
                            &timers,
                        )
                        .await
                        {
                            serial = Some(job);
                        }
                    }
                    None => {
                        info!("Invocation channel closed, module unloading");
                        break;
                    }
                }
            }
        }
    }
}

fn detached_timer(timers: &TimerRegistry, invocation: &InvocationMessage) -> bool {
    match &invocation.invocation_type {
        InvocationType::Timer(fire) => timers.is_detached(&fire.timer_id),
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_invocation<M, C, F>(
    invocation: InvocationMessage,
    result_tx: oneshot::Sender<InvocationResultMessage>,
    module: &M,
    cli: &C,
    ctx: &InvocationContext,
    dispatch: &F,
    module_api: Option<&Arc<dyn blvm_node::module::inter_module::api::ModuleAPI>>,
    timers: &Arc<TimerRegistry>,
) -> Option<Pin<Box<dyn std::future::Future<Output = ()> + Send>>>
where
    F: Fn(InvocationMessage, InvocationContext, &M, &C) -> InvocationResultMessage,
{
    if let InvocationType::Timer(fire) = invocation.invocation_type.clone() {
        if timers.is_detached(&fire.timer_id) {
            timers.start_detached(fire, invocation.correlation_id, result_tx);
            return None;
        }
        let correlation_id = invocation.correlation_id;
        let timers = Arc::clone(timers);
        return Some(Box::pin(async move {
            timers.run_serial(correlation_id, &fire, result_tx).await;
        }));
    }

    if let Some(module_api) = module_api
        && let InvocationType::ModuleApi {
            method,
            params,
            caller_module_id,
        } = &invocation.invocation_type
    {
        let result = match module_api
            .handle_request(method, params, caller_module_id)
            .await
        {
            Ok(data) => InvocationResultMessage {
                correlation_id: invocation.correlation_id,
                success: true,
                payload: Some(InvocationResultPayload::ModuleApi(data)),
                error: None,
            },
            Err(err) => InvocationResultMessage {
                correlation_id: invocation.correlation_id,
                success: false,
                payload: None,
                error: Some(err.to_string()),
            },
        };
        let _ = result_tx.send(result);
        return None;
    }

    let result = dispatch(invocation, ctx.clone(), module, cli);
    let _ = result_tx.send(result);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::storage::ModuleStorage;
    use crate::module::timers::Exec;
    use blvm_node::module::ipc::protocol::{EventMessage, EventPayload};
    use blvm_node::module::timers::types::{AckMode, Clock, TimerAck, TimerFire, TimerId};
    use blvm_node::module::traits::EventType;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::{Notify, broadcast, mpsc};

    struct NoStorage;

    impl ModuleStorage for NoStorage {
        fn open_tree(
            &self,
            _: &str,
        ) -> anyhow::Result<Arc<dyn crate::module::storage::ModuleTree>> {
            anyhow::bail!("unused")
        }
    }

    fn context(timers: Arc<TimerRegistry>) -> InvocationContext {
        InvocationContext::from_storage(Arc::new(NoStorage)).with_registry(timers)
    }

    fn fire(id: TimerId) -> TimerFire {
        TimerFire {
            timer_id: id,
            key: None,
            payload: Vec::new(),
            seq: 1,
            attempt: 1,
            clock: Clock::Monotonic,
            scheduled_unix_ms: 0,
            planned_unix_ms: 0,
            fired_unix_ms: 0,
            lateness_ms: 0,
            missed: 0,
            coalesced: 0,
            skipped_overlap: 0,
            clock_step_ms: 0,
            tip_height: None,
            tip_hash: None,
            tip_mtp: None,
            is_last: false,
        }
    }

    fn timer_message(id: TimerId, correlation_id: u64) -> InvocationMessage {
        InvocationMessage {
            correlation_id,
            invocation_type: InvocationType::Timer(fire(id)),
        }
    }

    fn event_message() -> ModuleMessage {
        ModuleMessage::Event(EventMessage {
            event_type: EventType::NewTransaction,
            payload: EventPayload::NewTransaction {
                tx_hash: blvm_node::Hash::default(),
            },
        })
    }

    fn dummy_result(invocation: InvocationMessage) -> InvocationResultMessage {
        InvocationResultMessage {
            correlation_id: invocation.correlation_id,
            success: true,
            payload: None,
            error: None,
        }
    }

    async fn until(order: &Mutex<Vec<&'static str>>, needle: &'static str) {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if order.lock().unwrap().contains(&needle) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect(needle);
    }

    #[tokio::test]
    async fn serial_blocks_events_until_the_handler_returns() {
        let timers = TimerRegistry::new();
        let serial_id = TimerId([1; 16]);
        let order = Arc::new(Mutex::new(Vec::new()));
        let release = Arc::new(Notify::new());
        let order_s = Arc::clone(&order);
        let release_s = Arc::clone(&release);
        timers.register(
            serial_id,
            Exec::Serial,
            AckMode::Completion,
            Arc::new(move |_fire| {
                let order_s = Arc::clone(&order_s);
                let release_s = Arc::clone(&release_s);
                Box::pin(async move {
                    order_s.lock().unwrap().push("serial-start");
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    release_s.notified().await;
                    order_s.lock().unwrap().push("serial-end");
                    Ok(TimerAck::Continue)
                })
            }),
        );
        let (event_tx, event_rx) = broadcast::channel(32);
        let (inv_tx, mut inv_rx) = mpsc::channel(8);
        let order_e = Arc::clone(&order);
        let timers_loop = Arc::clone(&timers);
        let ctx = context(Arc::clone(&timers));
        let running = tokio::spawn(async move {
            module_loop(
                event_rx,
                &mut inv_rx,
                &(),
                &(),
                ctx,
                |invocation, _, _, _| dummy_result(invocation),
                move |_event, _, _| {
                    let order_e = Arc::clone(&order_e);
                    async move {
                        order_e.lock().unwrap().push("event");
                        Ok(())
                    }
                },
                None,
                timers_loop,
            )
            .await;
        });
        inv_tx
            .send((timer_message(serial_id, 1), oneshot::channel().0))
            .await
            .unwrap();
        until(&order, "serial-start").await;
        event_tx.send(event_message()).unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(order.lock().unwrap().as_slice(), ["serial-start"]);
        release.notify_one();
        until(&order, "event").await;
        assert_eq!(
            order.lock().unwrap().as_slice(),
            ["serial-start", "serial-end", "event"]
        );
        drop(inv_tx);
        running.await.unwrap();
    }

    #[tokio::test]
    async fn task_does_not_block_events() {
        let timers = TimerRegistry::new();
        let task_id = TimerId([2; 16]);
        let order = Arc::new(Mutex::new(Vec::new()));
        let order_t = Arc::clone(&order);
        timers.register(
            task_id,
            Exec::Task,
            AckMode::Completion,
            Arc::new(move |_fire| {
                let order_t = Arc::clone(&order_t);
                Box::pin(async move {
                    order_t.lock().unwrap().push("task-start");
                    tokio::time::sleep(Duration::from_millis(40)).await;
                    order_t.lock().unwrap().push("task-end");
                    Ok(TimerAck::Continue)
                })
            }),
        );
        let (event_tx, event_rx) = broadcast::channel(32);
        let (inv_tx, mut inv_rx) = mpsc::channel(8);
        let order_e = Arc::clone(&order);
        let timers_loop = Arc::clone(&timers);
        let ctx = context(Arc::clone(&timers));
        let running = tokio::spawn(async move {
            module_loop(
                event_rx,
                &mut inv_rx,
                &(),
                &(),
                ctx,
                |invocation, _, _, _| dummy_result(invocation),
                move |_event, _, _| {
                    let order_e = Arc::clone(&order_e);
                    async move {
                        order_e.lock().unwrap().push("event");
                        Ok(())
                    }
                },
                None,
                timers_loop,
            )
            .await;
        });
        let (result_tx, result_rx) = oneshot::channel();
        inv_tx
            .send((timer_message(task_id, 2), result_tx))
            .await
            .unwrap();
        until(&order, "task-start").await;
        event_tx.send(event_message()).unwrap();
        until(&order, "event").await;
        until(&order, "task-end").await;
        let order = order.lock().unwrap().clone();
        let event_at = order.iter().position(|item| *item == "event").unwrap();
        let end_at = order.iter().position(|item| *item == "task-end").unwrap();
        assert!(event_at < end_at);
        assert!(result_rx.await.unwrap().success);
        drop(inv_tx);
        running.await.unwrap();
    }

    #[tokio::test]
    async fn dedicated_is_not_starved_by_a_serial_handler() {
        let timers = TimerRegistry::new();
        let serial_id = TimerId([3; 16]);
        let dedicated_id = TimerId([4; 16]);
        let release = Arc::new(Notify::new());
        let saw = Arc::new(AtomicUsize::new(0));
        let release_s = Arc::clone(&release);
        timers.register(
            serial_id,
            Exec::Serial,
            AckMode::Completion,
            Arc::new(move |_fire| {
                let release_s = Arc::clone(&release_s);
                Box::pin(async move {
                    release_s.notified().await;
                    Ok(TimerAck::Continue)
                })
            }),
        );
        let saw_d = Arc::clone(&saw);
        timers.register(
            dedicated_id,
            Exec::Dedicated("poll".into()),
            AckMode::Completion,
            Arc::new(move |_fire| {
                let saw_d = Arc::clone(&saw_d);
                Box::pin(async move {
                    saw_d.fetch_add(1, Ordering::SeqCst);
                    Ok(TimerAck::Continue)
                })
            }),
        );
        let (_event_tx, event_rx) = broadcast::channel(8);
        let (inv_tx, mut inv_rx) = mpsc::channel(8);
        let timers_loop = Arc::clone(&timers);
        let ctx = context(Arc::clone(&timers));
        let running = tokio::spawn(async move {
            module_loop(
                event_rx,
                &mut inv_rx,
                &(),
                &(),
                ctx,
                |invocation, _, _, _| dummy_result(invocation),
                |_event, _, _| async { Ok(()) },
                None,
                timers_loop,
            )
            .await;
        });
        inv_tx
            .send((timer_message(serial_id, 3), oneshot::channel().0))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        let (result_tx, result_rx) = oneshot::channel();
        inv_tx
            .send((timer_message(dedicated_id, 4), result_tx))
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while saw.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("dedicated handler waited for the serial handler");
        release.notify_one();
        assert!(result_rx.await.unwrap().success);
        drop(inv_tx);
        running.await.unwrap();
    }

    #[tokio::test]
    async fn a_registered_tick_runs_under_constant_event_traffic() {
        let timers = TimerRegistry::new();
        let tick_id = TimerId([5; 16]);
        let ticks = Arc::new(AtomicUsize::new(0));
        let events = Arc::new(AtomicUsize::new(0));
        let ticks_h = Arc::clone(&ticks);
        timers.register(
            tick_id,
            Exec::Serial,
            AckMode::Completion,
            Arc::new(move |_fire| {
                let ticks_h = Arc::clone(&ticks_h);
                Box::pin(async move {
                    ticks_h.fetch_add(1, Ordering::SeqCst);
                    Ok(TimerAck::Continue)
                })
            }),
        );
        let spec = sdk_tick_spec();
        assert_eq!(
            spec.clock,
            blvm_node::module::timers::types::Clock::Monotonic
        );
        assert_eq!(
            spec.first,
            blvm_node::module::timers::types::First::Delay(Duration::from_secs(30))
        );
        assert_eq!(
            spec.overlap,
            blvm_node::module::timers::types::Overlap::Skip
        );
        assert_eq!(spec.key.as_deref(), Some("sdk.tick"));

        let (event_tx, event_rx) = broadcast::channel(64);
        let (inv_tx, mut inv_rx) = mpsc::channel(8);
        let events_h = Arc::clone(&events);
        let timers_loop = Arc::clone(&timers);
        let ctx = context(Arc::clone(&timers));
        let running = tokio::spawn(async move {
            module_loop(
                event_rx,
                &mut inv_rx,
                &(),
                &(),
                ctx,
                |invocation, _, _, _| dummy_result(invocation),
                move |_event, _, _| {
                    let events_h = Arc::clone(&events_h);
                    async move {
                        events_h.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    }
                },
                None,
                timers_loop,
            )
            .await;
        });
        for _ in 0..20 {
            event_tx.send(event_message()).unwrap();
        }
        let (result_tx, result_rx) = oneshot::channel();
        inv_tx
            .send((timer_message(tick_id, 9), result_tx))
            .await
            .unwrap();
        for _ in 0..20 {
            event_tx.send(event_message()).unwrap();
        }
        assert!(result_rx.await.unwrap().success);
        tokio::time::timeout(Duration::from_secs(2), async {
            while events.load(Ordering::SeqCst) < 40 || ticks.load(Ordering::SeqCst) < 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("tick or events stalled under traffic");
        assert_eq!(ticks.load(Ordering::SeqCst), 1);
        assert_eq!(events.load(Ordering::SeqCst), 40);
        drop(inv_tx);
        running.await.unwrap();
    }
}
