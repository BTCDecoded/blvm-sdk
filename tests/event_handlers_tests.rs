//! Tests for #[event_handlers] and #[on_event] macros.
//!
//! Verifies: event_types() returns correct types, dispatch_event calls handlers,
//! and event payload is passed and extractable.

use blvm_node::module::ipc::protocol::{EventMessage, EventPayload};
use blvm_node::module::traits::{EventType, ModuleError};
use blvm_sdk::module::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
struct TestModule {
    new_block_count: AtomicU64,
    new_tx_count: AtomicU64,
    module_loaded_count: AtomicU64,
}

#[event_handlers]
impl TestModule {
    #[on_event(NewBlock)]
    async fn on_new_block(&self, event: &EventMessage) -> Result<(), ModuleError> {
        let (_block_hash, height) = event
            .payload
            .as_new_block()
            .expect("NewBlock event must have NewBlock payload");
        assert!(height > 0, "height must be positive");
        self.new_block_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    #[on_event(NewTransaction)]
    async fn on_new_tx(&self, event: &EventMessage) -> Result<(), ModuleError> {
        let _tx_hash = event
            .payload
            .as_new_transaction()
            .expect("NewTransaction event must have NewTransaction payload");
        self.new_tx_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    #[on_event(ModuleLoaded)]
    async fn on_loaded(&self, event: &EventMessage) -> Result<(), ModuleError> {
        let (name, version) = event
            .payload
            .as_module_loaded()
            .expect("ModuleLoaded event must have ModuleLoaded payload");
        assert_eq!(name, "test-module");
        assert_eq!(version, "1.0.0");
        self.module_loaded_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn test_event_types() {
    let types = TestModule::event_types();
    assert!(types.contains(&EventType::NewBlock));
    assert!(types.contains(&EventType::NewTransaction));
    assert!(types.contains(&EventType::ModuleLoaded));
    assert_eq!(types.len(), 3);
}

#[tokio::test]
async fn test_dispatch_new_block() {
    let module = TestModule::default();
    let event = EventMessage {
        event_type: EventType::NewBlock,
        payload: EventPayload::NewBlock {
            block_hash: [1u8; 32],
            height: 100,
        },
    };

    module.dispatch_event(event).await.unwrap();
    assert_eq!(module.new_block_count.load(Ordering::SeqCst), 1);

    // Dispatch again
    let event2 = EventMessage {
        event_type: EventType::NewBlock,
        payload: EventPayload::NewBlock {
            block_hash: [2u8; 32],
            height: 101,
        },
    };
    module.dispatch_event(event2).await.unwrap();
    assert_eq!(module.new_block_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_dispatch_new_transaction() {
    let module = TestModule::default();
    let event = EventMessage {
        event_type: EventType::NewTransaction,
        payload: EventPayload::NewTransaction { tx_hash: [3u8; 32] },
    };

    module.dispatch_event(event).await.unwrap();
    assert_eq!(module.new_tx_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_dispatch_module_loaded() {
    let module = TestModule::default();
    let event = EventMessage {
        event_type: EventType::ModuleLoaded,
        payload: EventPayload::ModuleLoaded {
            module_name: "test-module".to_string(),
            version: "1.0.0".to_string(),
        },
    };

    module.dispatch_event(event).await.unwrap();
    assert_eq!(module.module_loaded_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_dispatch_unknown_event_no_op() {
    let module = TestModule::default();
    let event = EventMessage {
        event_type: EventType::PeerConnected,
        payload: EventPayload::PeerConnected {
            peer_addr: "127.0.0.1:8333".to_string(),
            transport_type: "tcp".to_string(),
            services: 0,
            version: 70015,
        },
    };

    // Should not panic, handler not registered for PeerConnected
    module.dispatch_event(event).await.unwrap();
    assert_eq!(module.new_block_count.load(Ordering::SeqCst), 0);
    assert_eq!(module.new_tx_count.load(Ordering::SeqCst), 0);
}
