//! Node Composer
//!
//! High-level API for composing Bitcoin nodes from modules.

use std::path::Path;
use crate::composition::types::*;
use crate::composition::config::NodeConfig;
use crate::composition::validation::validate_composition;
use crate::composition::schema::validate_config_schema;
use crate::composition::registry::ModuleRegistry;
use crate::composition::lifecycle::ModuleLifecycle;

/// Node composer for building nodes from modules
pub struct NodeComposer {
    /// Module lifecycle manager (owns the registry)
    lifecycle: ModuleLifecycle,
}

impl NodeComposer {
    /// Create a new node composer
    pub fn new<P: AsRef<Path>>(modules_dir: P) -> Self {
        let registry = ModuleRegistry::new(modules_dir);
        let lifecycle = ModuleLifecycle::new(registry);
        
        Self {
            lifecycle,
        }
    }
    
    /// Compose node from configuration file
    pub async fn compose_from_config<P: AsRef<Path>>(
        &mut self,
        config_path: P
    ) -> Result<ComposedNode> {
        // Load configuration
        let config = NodeConfig::from_file(config_path)?;
        
        // Validate schema
        let schema_validation = validate_config_schema(&config)?;
        if !schema_validation.valid {
            return Err(CompositionError::ValidationFailed(
                format!("Schema validation failed: {:?}", schema_validation.errors)
            ));
        }
        
        // Convert to spec
        let spec = config.to_spec()?;
        
        // Compose from spec
        self.compose_node(spec).await
    }
    
    /// Compose node from specification
    pub async fn compose_node(&mut self, spec: NodeSpec) -> Result<ComposedNode> {
        // Validate composition
        let validation = self.validate_composition(&spec)?;
        if !validation.valid {
            return Err(CompositionError::ValidationFailed(
                format!("Composition validation failed: {:?}", validation.errors)
            ));
        }
        
        // Load all modules
        let mut loaded_modules = Vec::new();
        for module_spec in &spec.modules {
            if !module_spec.enabled {
                continue;
            }
            
            let info = self.lifecycle.registry.get_module(
                &module_spec.name,
                module_spec.version.as_deref()
            )?;
            
            // Start module via lifecycle (now async)
            self.lifecycle_mut().start_module(&info.name).await?;
            let status = self.lifecycle().get_module_status(&info.name).await?;
            let health = self.lifecycle().health_check(&info.name).await?;
            
            loaded_modules.push(LoadedModule {
                info,
                status,
                health,
            });
        }
        
        Ok(ComposedNode {
            spec,
            modules: loaded_modules,
            status: NodeStatus::Running,
        })
    }
    
    /// Validate composition
    pub fn validate_composition(&self, spec: &NodeSpec) -> Result<ValidationResult> {
        validate_composition(spec, &self.lifecycle.registry)
    }
    
    /// Generate configuration template
    pub fn generate_config(&self) -> String {
        let config = NodeConfig::template();
        toml::to_string_pretty(&config)
            .unwrap_or_else(|_| "# Error generating config template".to_string())
    }
    
    /// Get module registry (via lifecycle)
    pub fn registry(&self) -> &ModuleRegistry {
        &self.lifecycle.registry
    }
    
    /// Get mutable module registry (via lifecycle)
    pub fn registry_mut(&mut self) -> &mut ModuleRegistry {
        &mut self.lifecycle.registry
    }
    
    /// Get module lifecycle manager
    pub fn lifecycle(&self) -> &ModuleLifecycle {
        &self.lifecycle
    }
    
    /// Get mutable module lifecycle manager
    pub fn lifecycle_mut(&mut self) -> &mut ModuleLifecycle {
        &mut self.lifecycle
    }
}

