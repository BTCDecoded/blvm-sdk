//! Composition Configuration Schema
//!
//! Schema validation for node composition configuration.

use crate::composition::config::NodeConfig;
use crate::composition::types::*;

/// Validate node configuration schema
pub fn validate_config_schema(config: &NodeConfig) -> Result<ValidationResult> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    
    // Validate node metadata
    if config.node.name.is_empty() {
        errors.push("Node name cannot be empty".to_string());
    }
    
    if !["mainnet", "testnet", "regtest"].contains(&config.node.network.as_str()) {
        errors.push(format!(
            "Invalid network type: {}. Must be one of: mainnet, testnet, regtest",
            config.node.network
        ));
    }
    
    // Validate modules
    for (name, module_cfg) in &config.modules {
        if module_cfg.enabled {
            if name.is_empty() {
                errors.push("Module name cannot be empty".to_string());
            }
            
            // Warn if version not specified
            if module_cfg.version.is_none() {
                warnings.push(format!(
                    "Module '{}' does not specify version, will use latest available",
                    name
                ));
            }
        }
    }
    
    let valid = errors.is_empty();
    Ok(ValidationResult {
        valid,
        errors,
        warnings,
        dependencies: Vec::new(), // Will be populated during dependency resolution
    })
}

