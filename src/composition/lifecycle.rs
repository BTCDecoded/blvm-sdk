//! Module Lifecycle Management
//!
//! Handles starting, stopping, restarting, and health checking of modules.

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use crate::composition::types::*;
use crate::composition::registry::ModuleRegistry;
use crate::composition::conversion::*;
use reference_node::module::manager::ModuleManager;
use reference_node::module::traits::ModuleMetadata as RefModuleMetadata;

/// Module lifecycle manager
pub struct ModuleLifecycle {
    /// Module registry reference
    pub(crate) registry: ModuleRegistry,
    /// Reference to reference-node ModuleManager (if available)
    module_manager: Option<Arc<Mutex<ModuleManager>>>,
    /// Module status cache
    status_cache: HashMap<String, ModuleStatus>,
}

impl ModuleLifecycle {
    /// Create a new module lifecycle manager
    pub fn new(registry: ModuleRegistry) -> Self {
        Self {
            registry,
            module_manager: None,
            status_cache: HashMap::new(),
        }
    }
    
    /// Set the ModuleManager for actual module operations
    pub fn with_module_manager(mut self, manager: Arc<Mutex<ModuleManager>>) -> Self {
        self.module_manager = Some(manager);
        self
    }
    
    /// Start a module
    pub async fn start_module(&mut self, name: &str) -> Result<()> {
        let info = self.registry.get_module(name, None)?;
        
        if let Some(ref manager) = self.module_manager {
            // Convert ModuleInfo to ModuleMetadata
            let metadata: RefModuleMetadata = info.clone().into();
            
            let binary_path = info.binary_path.as_ref()
                .ok_or_else(|| CompositionError::ModuleNotFound(
                    format!("Module {} has no binary path", name)
                ))?;
            
            // Load module via ModuleManager
            let mut mgr = manager.lock().await;
            mgr.load_module(
                &info.name,
                binary_path,
                metadata,
                HashMap::new(), // TODO: Get config from ModuleSpec
            ).await
            .map_err(|e| CompositionError::from(e))?;
            
            self.status_cache.insert(name.to_string(), ModuleStatus::Running);
        } else {
            // Fallback: just cache status
            self.status_cache.insert(name.to_string(), ModuleStatus::Running);
        }
        
        Ok(())
    }
    
    /// Stop a module
    pub async fn stop_module(&mut self, name: &str) -> Result<()> {
        let _info = self.registry.get_module(name, None)?;
        
        if let Some(ref manager) = self.module_manager {
            let mut mgr = manager.lock().await;
            mgr.unload_module(name).await
                .map_err(|e| CompositionError::from(e))?;
        }
        
        self.status_cache.insert(name.to_string(), ModuleStatus::Stopped);
        Ok(())
    }
    
    /// Restart a module
    pub async fn restart_module(&mut self, name: &str) -> Result<()> {
        self.stop_module(name).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.start_module(name).await
    }
    
    /// Get module status
    pub async fn get_module_status(&self, name: &str) -> Result<ModuleStatus> {
        let _module = self.registry.get_module(name, None)?;
        
        // TODO: Query actual ModuleManager state if available
        Ok(self.status_cache.get(name)
            .cloned()
            .unwrap_or(ModuleStatus::NotInstalled))
    }
    
    /// Perform health check on module
    pub async fn health_check(&self, name: &str) -> Result<ModuleHealth> {
        let status = self.get_module_status(name).await?;
        match status {
            ModuleStatus::Running => Ok(ModuleHealth::Healthy),
            ModuleStatus::Error(msg) => Ok(ModuleHealth::Unhealthy(msg)),
            ModuleStatus::Stopped | ModuleStatus::NotInstalled => Ok(ModuleHealth::Unknown),
            _ => Ok(ModuleHealth::Degraded),
        }
    }
    
    /// Get the module registry
    pub fn registry(&self) -> &ModuleRegistry {
        &self.registry
    }
    
    /// Get mutable access to the module registry
    pub fn registry_mut(&mut self) -> &mut ModuleRegistry {
        &mut self.registry
    }
}

