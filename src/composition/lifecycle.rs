//! Module Lifecycle Management
//!
//! Handles starting, stopping, restarting, and health checking of modules.

use crate::composition::registry::ModuleRegistry;
use crate::composition::types::*;
use blvm_node::module::manager::ModuleManager;
use blvm_node::module::traits::{
    ModuleMetadata as RefModuleMetadata, ModuleState as RefModuleState,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

fn module_state_to_status(s: RefModuleState) -> ModuleStatus {
    match s {
        RefModuleState::Running => ModuleStatus::Running,
        RefModuleState::Stopped => ModuleStatus::Stopped,
        RefModuleState::Initializing => ModuleStatus::Initializing,
        RefModuleState::Stopping => ModuleStatus::Stopping,
        RefModuleState::Error(msg) => ModuleStatus::Error(msg),
    }
}

/// Module lifecycle manager
pub struct ModuleLifecycle {
    /// Module registry reference
    pub(crate) registry: ModuleRegistry,
    /// Reference to blvm-node ModuleManager (if available)
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

    /// Start a module with optional config (from ModuleSpec.config)
    pub async fn start_module(
        &mut self,
        name: &str,
        config: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        if self.module_manager.is_none() {
            return Err(CompositionError::InstallationFailed(
                "ModuleManager required to start modules; use ModuleLifecycle::with_module_manager"
                    .to_string(),
            ));
        }

        let info = self.registry.get_module(name, None)?;

        let config_map: HashMap<String, String> = config
            .map(|c| {
                c.iter()
                    .map(|(k, v)| {
                        let s = match v {
                            serde_json::Value::String(s) => s.clone(),
                            _ => v.to_string(),
                        };
                        (k.clone(), s)
                    })
                    .collect()
            })
            .unwrap_or_default();

        if let Some(ref manager) = self.module_manager {
            let metadata: RefModuleMetadata = info.clone().into();

            let binary_path = info.binary_path.as_ref().ok_or_else(|| {
                CompositionError::ModuleNotFound(format!("Module {name} has no binary path"))
            })?;

            let mut mgr = manager.lock().await;
            mgr.load_module(&info.name, binary_path, metadata, config_map)
                .await
                .map_err(CompositionError::from)?;

            self.status_cache
                .insert(name.to_string(), ModuleStatus::Running);
        }

        Ok(())
    }

    /// Stop a module
    pub async fn stop_module(&mut self, name: &str) -> Result<()> {
        let _info = self.registry.get_module(name, None)?;

        if let Some(ref manager) = self.module_manager {
            let mut mgr = manager.lock().await;
            mgr.unload_module(name)
                .await
                .map_err(CompositionError::from)?;
        }

        self.status_cache
            .insert(name.to_string(), ModuleStatus::Stopped);
        Ok(())
    }

    /// Restart a module
    pub async fn restart_module(
        &mut self,
        name: &str,
        config: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        self.stop_module(name).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.start_module(name, config).await
    }

    /// Get module status (queries ModuleManager when available, else cache)
    pub async fn get_module_status(&self, name: &str) -> Result<ModuleStatus> {
        let _ = self.registry.get_module(name, None)?;

        if let Some(ref manager) = self.module_manager {
            if let Some(state) = manager.lock().await.get_module_state(name).await {
                return Ok(module_state_to_status(state));
            }
        }

        Ok(self
            .status_cache
            .get(name)
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
