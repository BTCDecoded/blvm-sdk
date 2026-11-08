//! Upstream poll timer running as a module.
//!
//! The node spawns this with `MODULE_ID`, `MODULE_NAME`, `SOCKET_PATH`, and
//! `DATA_DIR`. The first fire writes `DATA_DIR/fired`.

use std::sync::Arc;
use std::time::Duration;

use blvm_node::module::api::node_api_ipc::NodeApiIpc;
use blvm_node::module::ipc::client::ModuleIpcClient;
use blvm_node::module::ipc::protocol::{
    InvocationType, MessageType, ModuleMessage, RequestMessage, RequestPayload,
};
use blvm_node::module::timers::types::{JitterShape, Overlap, TimerAck};
use blvm_sdk::module::{Exec, TimerBuilder, TimerRegistry};
use tokio::sync::{oneshot, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let module_id = std::env::var("MODULE_ID")?;
    let module_name = std::env::var("MODULE_NAME")?;
    let socket_path = std::env::var("SOCKET_PATH")?;
    let data_dir = std::path::PathBuf::from(std::env::var("DATA_DIR")?);

    let mut client = ModuleIpcClient::connect(&socket_path).await?;
    let correlation_id = client.next_correlation_id();
    let handshake = client
        .request(RequestMessage {
            correlation_id,
            request_type: MessageType::Handshake,
            payload: RequestPayload::Handshake {
                module_id: module_id.clone(),
                module_name,
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        })
        .await?;
    if !handshake.success {
        return Err(handshake
            .error
            .unwrap_or_else(|| "handshake failed".into())
            .into());
    }

    let client = Arc::new(Mutex::new(client));
    let api = Arc::new(NodeApiIpc::new(Arc::clone(&client), module_id));
    let registry = TimerRegistry::new();
    let fired = data_dir.join("fired");
    let fired_for_handler = fired.clone();
    TimerBuilder::with_runtime(api, Arc::clone(&registry))
        .every(Duration::from_secs(10))
        .fixed_rate()
        .jitter(Duration::from_millis(20), JitterShape::Symmetric)
        .max_fires(1)
        .on_overlap(Overlap::Skip)
        .key("upstream.poll")
        .exec(Exec::Task)
        .spawn(move |fire| {
            let path = fired_for_handler.clone();
            async move {
                std::fs::write(&path, fire.seq.to_string()).map_err(|err| err.to_string())?;
                Ok(TimerAck::Stop)
            }
        })
        .await?;

    loop {
        let message = {
            let mut client = client.lock().await;
            client.receive_message().await?
        };
        let Some(ModuleMessage::Invocation(invocation)) = message else {
            continue;
        };
        let InvocationType::Timer(fire) = invocation.invocation_type else {
            continue;
        };
        let (result_tx, result_rx) = oneshot::channel();
        registry.start_detached(fire, invocation.correlation_id, result_tx);
        if let Ok(result) = result_rx.await {
            client.lock().await.send_invocation_result(result).await?;
        }
        if fired.exists() {
            break;
        }
    }
    Ok(())
}
