//! IPC Communication
//!
//! IPC protocol and client for module-to-node communication.
//!
//! Modules communicate with the node via Inter-Process Communication (IPC)
//! using Unix domain sockets. This module provides the protocol types and
//! client implementation.

pub mod client;
pub mod protocol;

pub use client::ModuleIpcClient;
pub use protocol::*;
