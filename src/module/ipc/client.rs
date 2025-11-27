//! IPC Client
//!
//! Re-export from bllvm-node.
//!
//! Client-side IPC implementation that modules use to communicate with the node.

#[cfg(unix)]
pub use bllvm_node::module::ipc::ModuleIpcClient;
