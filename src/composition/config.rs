//! Composition Configuration
//!
//! TOML-based declarative configuration format for node composition.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::composition::types::*;

/// Node configuration from TOML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Node metadata
    #[serde(default)]
    pub node: NodeMetadata,
    /// Module configurations
    #[serde(default)]
    pub modules: HashMap<String, ModuleConfig>,
}

/// Node metadata section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    /// Node name
    pub name: String,
    /// Node version
    #[serde(default)]
    pub version: Option<String>,
    /// Network type
    pub network: String,
}

impl Default for NodeMetadata {
    fn default() -> Self {
        Self {
            name: "custom-node".to_string(),
            version: None,
            network: "mainnet".to_string(),
        }
    }
}

/// Module configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// Whether module is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Module version (optional)
    #[serde(default)]
    pub version: Option<String>,
    /// Module-specific configuration
    #[serde(default)]
    pub config: HashMap<String, toml::Value>,
}

fn default_true() -> bool {
    true
}

impl NodeConfig {
    /// Load configuration from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path.as_ref())
            .map_err(|e| CompositionError::IoError(e))?;
        
        let config: NodeConfig = toml::from_str(&contents)
            .map_err(|e| CompositionError::InvalidConfiguration(
                format!("Failed to parse TOML: {}", e)
            ))?;
        
        Ok(config)
    }
    
    /// Save configuration to TOML file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let toml_string = toml::to_string_pretty(self)
            .map_err(|e| CompositionError::SerializationError(
                format!("Failed to serialize config: {}", e)
            ))?;
        
        std::fs::write(path.as_ref(), toml_string)
            .map_err(|e| CompositionError::IoError(e))?;
        
        Ok(())
    }
    
    /// Convert to NodeSpec
    pub fn to_spec(&self) -> Result<NodeSpec> {
        let network = match self.node.network.as_str() {
            "mainnet" => NetworkType::Mainnet,
            "testnet" => NetworkType::Testnet,
            "regtest" => NetworkType::Regtest,
            _ => return Err(CompositionError::InvalidConfiguration(
                format!("Unknown network type: {}", self.node.network)
            )),
        };
        
        let modules: Result<Vec<ModuleSpec>> = self.modules.iter()
            .filter(|(_, cfg)| cfg.enabled)
            .map(|(name, cfg)| {
                // Convert toml::Value to serde_json::Value
                let config: HashMap<String, serde_json::Value> = cfg.config.iter()
                    .map(|(k, v)| {
                        let json_value = toml_to_json_value(v);
                        (k.clone(), json_value)
                    })
                    .collect();
                
                Ok(ModuleSpec {
                    name: name.clone(),
                    version: cfg.version.clone(),
                    enabled: cfg.enabled,
                    config,
                })
            })
            .collect();
        
        Ok(NodeSpec {
            name: self.node.name.clone(),
            version: self.node.version.clone(),
            network,
            modules: modules?,
        })
    }
    
    /// Generate template configuration
    pub fn template() -> Self {
        let mut modules = HashMap::new();
        
        // Add example modules
        modules.insert("lightning".to_string(), ModuleConfig {
            enabled: false,
            version: Some("0.1.0".to_string()),
            config: HashMap::new(),
        });
        
        modules.insert("privacy".to_string(), ModuleConfig {
            enabled: false,
            version: Some("0.2.0".to_string()),
            config: HashMap::new(),
        });
        
        Self {
            node: NodeMetadata {
                name: "my-custom-node".to_string(),
                version: Some("1.0.0".to_string()),
                network: "mainnet".to_string(),
            },
            modules,
        }
    }
}

/// Convert toml::Value to serde_json::Value
fn toml_to_json_value(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        toml::Value::Float(f) => {
            serde_json::Value::Number(
                serde_json::Number::from_f64(*f).unwrap_or_else(|| serde_json::Number::from(0))
            )
        }
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(toml_to_json_value).collect())
        }
        toml::Value::Table(table) => {
            let map: serde_json::Map<String, serde_json::Value> = table.iter()
                .map(|(k, v)| (k.clone(), toml_to_json_value(v)))
                .collect();
            serde_json::Value::Object(map)
        }
    }
}

