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

pub mod types;
pub mod registry;
pub mod lifecycle;
pub mod config;
pub mod schema;
pub mod composer;
pub mod validation;
pub mod conversion;

// Re-export main types for convenience
pub use types::*;
pub use registry::ModuleRegistry;
pub use lifecycle::ModuleLifecycle;
pub use composer::NodeComposer;
pub use config::NodeConfig;

