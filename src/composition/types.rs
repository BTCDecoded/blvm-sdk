//! Composition Framework Types
//!
//! Core types for module registry and node composition.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Module information from registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleInfo {
    /// Module name (unique identifier)
    pub name: String,
    /// Module version (semantic versioning)
    pub version: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Module author
    pub author: Option<String>,
    /// Capabilities this module declares it can use
    pub capabilities: Vec<String>,
    /// Required dependencies (module names with versions)
    pub dependencies: HashMap<String, String>,
    /// Module entry point (binary name or path)
    pub entry_point: String,
    /// Path to module directory
    pub directory: Option<PathBuf>,
    /// Path to module binary
    pub binary_path: Option<PathBuf>,
    /// Module configuration schema (optional)
    pub config_schema: HashMap<String, String>,
}

/// Module source for installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleSource {
    /// Install from local path
    Path(PathBuf),
    /// Install from remote registry (URL)
    Registry(String),
    /// Install from git repository
    Git { url: String, tag: Option<String> },
}

/// Module lifecycle status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleStatus {
    /// Module is not installed
    NotInstalled,
    /// Module is installed but not running
    Stopped,
    /// Module is initializing
    Initializing,
    /// Module is running normally
    Running,
    /// Module is stopping
    Stopping,
    /// Module has crashed or errored
    Error(String),
}

/// Module health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleHealth {
    /// Module is healthy and responding
    Healthy,
    /// Module is degraded but functioning
    Degraded,
    /// Module is unhealthy or not responding
    Unhealthy(String),
    /// Health status unknown
    Unknown,
}

/// Network type for node composition
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkType {
    /// Bitcoin mainnet
    Mainnet,
    /// Bitcoin testnet
    Testnet,
    /// Regression test network
    Regtest,
}

/// Node specification for composition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSpec {
    /// Node name
    pub name: String,
    /// Node version
    pub version: Option<String>,
    /// Network type
    pub network: NetworkType,
    /// Modules to include
    pub modules: Vec<ModuleSpec>,
}

/// Module specification in node composition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSpec {
    /// Module name
    pub name: String,
    /// Module version (optional, uses latest if not specified)
    pub version: Option<String>,
    /// Whether module is enabled
    pub enabled: bool,
    /// Module-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

/// Loaded module information
#[derive(Debug, Clone)]
pub struct LoadedModule {
    /// Module information
    pub info: ModuleInfo,
    /// Module status
    pub status: ModuleStatus,
    /// Module health
    pub health: ModuleHealth,
}

/// Composed node result
#[derive(Debug, Clone)]
pub struct ComposedNode {
    /// Node specification
    pub spec: NodeSpec,
    /// Loaded modules
    pub modules: Vec<LoadedModule>,
    /// Overall node status
    pub status: NodeStatus,
}

/// Node status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeStatus {
    /// Node is stopped
    Stopped,
    /// Node is starting up
    Starting,
    /// Node is running
    Running,
    /// Node is stopping
    Stopping,
    /// Node has errors
    Error(String),
}

/// Composition validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether composition is valid
    pub valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Resolved dependencies
    pub dependencies: Vec<ModuleInfo>,
}

/// Composition errors
#[derive(Debug, Error)]
pub enum CompositionError {
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    
    #[error("Module version not found: {0} {1}")]
    ModuleVersionNotFound(String, String),
    
    #[error("Dependency resolution failed: {0}")]
    DependencyResolutionFailed(String),
    
    #[error("Module installation failed: {0}")]
    InstallationFailed(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("Composition validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, CompositionError>;

