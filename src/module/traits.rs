//! Module Traits
//!
//! Re-export from bllvm-node for module developers.
//!
//! These traits define the core interfaces that modules must implement
//! and the APIs they can use to interact with the node.

pub use bllvm_node::module::traits::{
    EventType, Module, ModuleContext, ModuleError, ModuleMetadata, ModuleState, NodeAPI,
};
