//! Composition Framework
//!
//! Provides declarative module composition and module registry management
//! for building custom Bitcoin nodes from modules.
//!
//! This module enables:
//! - Module discovery and registry management
//! - Declarative node composition from TOML configuration
//! - Module lifecycle management (start/stop/restart)
//! - Dependency resolution and validation

pub mod composer;
pub mod config;
pub mod conversion;
pub mod lifecycle;
pub mod registry;
pub mod schema;
pub mod types;
pub mod validation;

// Re-export main types for convenience
pub use composer::NodeComposer;
pub use config::NodeConfig;
pub use lifecycle::ModuleLifecycle;
pub use registry::ModuleRegistry;
pub use types::*;
