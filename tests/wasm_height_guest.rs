//! One `.wat` guest: `NewBlock` reaches `on_event`, which calls `get_block_height`.

#![cfg(all(feature = "node", feature = "wasm-modules"))]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use blvm_node::module::ipc::protocol::EventPayload;
use blvm_node::module::manager::ModuleManager;
use blvm_node::module::traits::{EventType, ModuleMetadata};
use blvm_sdk::module::wasm::BlvmSdkWasmLoader;

#[path = "../../blvm-node/tests/stub_node_api.rs"]
mod stub_node_api;

const GUEST_WAT: &str = r#"
(module
  (import "env" "host_node_call" (func $host (param i32 i32 i32 i32) (result i32 i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "height-guest")
  (data (i32.const 16) "0.1.0")
  (data (i32.const 32) "{\"name\":\"height-guest\",\"version\":1,\"subcommands\":[]}")
  (data (i32.const 96) "{\"stdout\":\"\",\"stderr\":\"\",\"exit_code\":0}")
  (data (i32.const 160) "null")
  (data (i32.const 168) "[\"NewBlock\"]")
  (data (i32.const 192) "get_block_height")
  (func (export "module_name") (result i32 i32) (i32.const 0) (i32.const 12))
  (func (export "module_version") (result i32 i32) (i32.const 16) (i32.const 5))
  (func (export "cli_spec") (result i32 i32) (i32.const 32) (i32.const 52))
  (func (export "dispatch_cli") (param i32 i32 i32 i32 i32) (result i32 i32)
    (i32.const 96) (i32.const 39))
  (func (export "dispatch_rpc") (param i32 i32 i32 i32 i32) (result i32 i32)
    (i32.const 160) (i32.const 4))
  (func (export "event_types") (result i32 i32) (i32.const 168) (i32.const 12))
  (func (export "on_event") (param i32 i32) (result i32 i32)
    (call $host (i32.const 192) (i32.const 16) (i32.const 0) (i32.const 0)))
)
"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wasm_guest_receives_new_block_and_reads_height() {
    let manifest = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../blvm/Cargo.toml"
    ))
    .expect("blvm Cargo.toml");
    let default = manifest
        .split("default = [")
        .nth(1)
        .expect("default features")
        .split(']')
        .next()
        .expect("default feature list");
    assert!(
        !default.contains("wasm-modules"),
        "wasm-modules stays off the default blvm binary"
    );

    let temp = tempfile::tempdir().expect("tempdir");
    let wasm_path = temp.path().join("height-guest.wasm");
    std::fs::write(&wasm_path, GUEST_WAT).expect("write wat");

    let mut manager = ModuleManager::new(temp.path(), temp.path(), temp.path());
    manager.set_wasm_loader(Arc::new(BlvmSdkWasmLoader));
    manager
        .start(temp.path().join("node.sock"), Arc::new(stub_node_api::MockNodeAPI))
        .await
        .expect("start module manager");

    let metadata = ModuleMetadata {
        name: "height-guest".to_string(),
        version: "0.1.0".to_string(),
        description: "wasm height guest".to_string(),
        author: "test".to_string(),
        capabilities: vec!["read_chain_state".to_string()],
        rpc_overrides: Vec::new(),
        dependencies: HashMap::new(),
        optional_dependencies: HashMap::new(),
        entry_point: "height-guest.wasm".to_string(),
    };
    manager
        .load_module("height-guest", &wasm_path, metadata, HashMap::new())
        .await
        .expect("load wasm guest");

    manager
        .event_manager()
        .publish_event(
            EventType::NewBlock,
            EventPayload::NewBlock {
                block_hash: blvm_node::Hash::default(),
                height: 1,
            },
        )
        .await
        .expect("publish NewBlock");

    let deadline = Instant::now() + Duration::from_secs(2);
    let result = loop {
        if let Some(result) = manager.wasm_last_on_event_result("height-guest").await {
            break result;
        }
        if Instant::now() > deadline {
            panic!("guest did not complete on_event");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    assert_eq!(result, "100");
}

const BLOCK_GUEST_WAT: &str = r#"
(module
  (import "env" "host_node_call" (func $host (param i32 i32 i32 i32) (result i32 i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "block-guest")
  (data (i32.const 16) "0.1.0")
  (data (i32.const 32) "{\"name\":\"block-guest\",\"version\":1,\"subcommands\":[]}")
  (data (i32.const 96) "{\"stdout\":\"\",\"stderr\":\"\",\"exit_code\":0}")
  (data (i32.const 160) "null")
  (data (i32.const 168) "[\"NewBlock\"]")
  (data (i32.const 192) "get_block")
  (data (i32.const 208) "{\"hash\":\"0000000000000000000000000000000000000000000000000000000000000000\"}")
  (func (export "module_name") (result i32 i32) (i32.const 0) (i32.const 11))
  (func (export "module_version") (result i32 i32) (i32.const 16) (i32.const 5))
  (func (export "cli_spec") (result i32 i32) (i32.const 32) (i32.const 51))
  (func (export "dispatch_cli") (param i32 i32 i32 i32 i32) (result i32 i32)
    (i32.const 96) (i32.const 39))
  (func (export "dispatch_rpc") (param i32 i32 i32 i32 i32) (result i32 i32)
    (i32.const 160) (i32.const 4))
  (func (export "event_types") (result i32 i32) (i32.const 168) (i32.const 12))
  (func (export "on_event") (param i32 i32) (result i32 i32)
    (call $host (i32.const 192) (i32.const 9) (i32.const 208) (i32.const 75)))
)
"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wasm_guest_reads_block_with_read_blockchain() {
    let temp = tempfile::tempdir().expect("tempdir");
    let wasm_path = temp.path().join("block-guest.wasm");
    std::fs::write(&wasm_path, BLOCK_GUEST_WAT).expect("write wat");

    let mut manager = ModuleManager::new(temp.path(), temp.path(), temp.path());
    manager.set_wasm_loader(Arc::new(BlvmSdkWasmLoader));
    manager
        .start(temp.path().join("node.sock"), Arc::new(stub_node_api::MockNodeAPI))
        .await
        .expect("start module manager");

    let metadata = ModuleMetadata {
        name: "block-guest".to_string(),
        version: "0.1.0".to_string(),
        description: "wasm block guest".to_string(),
        author: "test".to_string(),
        capabilities: vec!["read_blockchain".to_string()],
        rpc_overrides: Vec::new(),
        dependencies: HashMap::new(),
        optional_dependencies: HashMap::new(),
        entry_point: "block-guest.wasm".to_string(),
    };
    manager
        .load_module("block-guest", &wasm_path, metadata, HashMap::new())
        .await
        .expect("load wasm guest");

    manager
        .event_manager()
        .publish_event(
            EventType::NewBlock,
            EventPayload::NewBlock {
                block_hash: blvm_node::Hash::default(),
                height: 1,
            },
        )
        .await
        .expect("publish NewBlock");

    let deadline = Instant::now() + Duration::from_secs(2);
    let result = loop {
        if let Some(result) = manager.wasm_last_on_event_result("block-guest").await {
            break result;
        }
        if Instant::now() > deadline {
            panic!("guest did not complete on_event");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    let block: serde_json::Value = serde_json::from_str(&result).expect(&result);
    assert_eq!(block["header"]["version"], 1);
}
