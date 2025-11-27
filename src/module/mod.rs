//! Module Development APIs
//!
//! Provides APIs for developing modules that extend bllvm-node.
//!
//! This module re-exports the necessary types and traits from `bllvm-node` to provide
//! a clean, developer-friendly interface for module development.

pub mod ipc;
pub mod manifest;
pub mod security;
pub mod traits;

// Re-export main types for convenience
pub use ipc::client::ModuleIpcClient;
pub use ipc::protocol::*;
pub use manifest::ModuleManifest;
pub use security::{Permission, PermissionSet};
pub use traits::*;
