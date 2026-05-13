//! IPC Client
//!
//! Re-export from blvm-node.
//!
//! Client-side IPC implementation that modules use to communicate with the node.
//! `blvm-node` implements this with Unix domain sockets or Windows named pipes under the same type.

pub use blvm_node::module::ipc::ModuleIpcClient;
