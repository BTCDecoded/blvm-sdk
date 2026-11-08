//! The upstream-poll example runs under the same sandbox the node uses to spawn a module.
//!
//! Landlock and `prlimit` are the ones `ModuleProcessSpawner` applies. This test does not
//! open a second handshake: that client would take the timer route away from the module.

#![cfg(unix)]

use std::sync::Arc;
use std::time::Duration;

use blvm_node::module::api::node_api::NodeApiImpl;
use blvm_node::module::ipc::server::{IpcTimerTransport, ModuleIpcServer};
use blvm_node::module::sandbox::landlock::ModuleLandlock;
use blvm_node::module::sandbox::{ProcessSandbox, SandboxConfig};
use blvm_node::module::timers::service::TimerService;
use blvm_node::module::timers::types::TimerConfig;
use blvm_node::storage::Storage;

#[tokio::test]
async fn upstream_poll_example_runs_in_the_module_sandbox() {
    let root = tempfile::TempDir::new_in("/tmp").expect("temp dir");
    let modules_dir = root.path().join("modules");
    let data_dir = root.path().join("data");
    let module_data = data_dir.join("timerpoll");
    let socket = root.path().join("node.sock");
    std::fs::create_dir_all(&modules_dir).unwrap();
    std::fs::create_dir_all(&module_data).unwrap();

    let storage = Arc::new(Storage::new(root.path().join("chain")).unwrap());
    let mut api = NodeApiImpl::new(storage);
    let mut config = TimerConfig::default();
    config.min_period_ms = 1;
    let service = TimerService::spawn(config, std::collections::HashMap::new());
    api.set_timer_service(service.clone());
    let mut server = ModuleIpcServer::new(&socket).with_timer_service(service.clone());
    let ipc = server.handle();
    service
        .set_transport(Arc::new(IpcTimerTransport::new(ipc)))
        .await;
    let api = Arc::new(api);
    let server = tokio::spawn(async move { server.start(api).await });
    let bound = tokio::time::Instant::now() + Duration::from_secs(10);
    while !socket.exists() {
        if server.is_finished() || tokio::time::Instant::now() > bound {
            panic!("ipc server did not bind {socket:?}");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("timer-poll");
    let landlock = ModuleLandlock::prepare(&binary, &module_data, &socket, &modules_dir)
        .expect("landlock domain");
    let mut sandbox_config = SandboxConfig::new(&data_dir);
    // RLIMIT_NPROC counts every thread of this user. The default of 10 is already
    // exceeded on this host, so a new thread cannot start. Memory and file-descriptor
    // limits still come from the sandbox.
    sandbox_config.resource_limits.max_memory_bytes = Some(8 * 1024 * 1024 * 1024);
    sandbox_config.resource_limits.max_child_processes = None;
    let sandbox = ProcessSandbox::new(sandbox_config);
    let module_id = format!("timerpoll_{}", "ab".repeat(16));
    let mut command = std::process::Command::new(&binary);
    command
        .env_clear()
        .env("MODULE_NAME", "timerpoll")
        .env("MODULE_ID", &module_id)
        .env("SOCKET_PATH", &socket)
        .env("DATA_DIR", &module_data)
        .env("MALLOC_CONF", "background_thread:false")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    landlock.arm_std(&mut command);
    let mut child = command.spawn().expect("spawn example");
    sandbox
        .apply_limits(Some(child.id()))
        .expect("resource limits");

    let fired = module_data.join("fired");
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !fired.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            let err = child_output(&mut child);
            panic!("example exited before the fire: {status}\n{err}");
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            let err = child_output(&mut child);
            panic!("example did not fire under the sandbox\n{err}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let body = std::fs::read_to_string(&fired).unwrap();
    assert_eq!(body, "1");
    let _ = child.kill();
    let _ = child.wait();
    server.abort();
}

fn child_output(child: &mut std::process::Child) -> String {
    let mut out = String::new();
    if let Some(stdout) = child.stdout.take() {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut std::io::BufReader::new(stdout), &mut text);
        out.push_str(&text);
    }
    if let Some(stderr) = child.stderr.take() {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut std::io::BufReader::new(stderr), &mut text);
        out.push_str(&text);
    }
    out
}
