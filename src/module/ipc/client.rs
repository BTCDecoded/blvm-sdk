//! IPC Client
//!
//! Re-export from blvm-node.
//!
//! Client-side IPC implementation that modules use to communicate with the node.

#[cfg(unix)]
pub use blvm_node::module::ipc::ModuleIpcClient;
