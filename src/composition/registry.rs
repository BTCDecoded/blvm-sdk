//! Module Registry
//!
//! High-level module registry API for discovering, installing, updating,
//! and removing modules. Wraps bllvm-node module registry functionality.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::composition::types::*;
use crate::composition::conversion::*;
use bllvm_node::module::registry::{
    ModuleDiscovery as RefModuleDiscovery,
    DiscoveredModule as RefDiscoveredModule,
    ModuleDependencies as RefModuleDependencies,
    ModuleManifest as RefModuleManifest,
};
use bllvm_node::module::traits::ModuleError as RefModuleError;

/// Module registry for managing module lifecycle
pub struct ModuleRegistry {
    /// Base directory for modules
    modules_dir: PathBuf,
    /// Discovered modules cache
    discovered: Vec<ModuleInfo>,
}

impl ModuleRegistry {
    /// Create a new module registry
    pub fn new<P: AsRef<Path>>(modules_dir: P) -> Self {
        Self {
            modules_dir: modules_dir.as_ref().to_path_buf(),
            discovered: Vec::new(),
        }
    }
    
    /// Discover available modules in the modules directory
    pub fn discover_modules(&mut self) -> Result<Vec<ModuleInfo>> {
        let discovery = RefModuleDiscovery::new(&self.modules_dir);
        let discovered = discovery.discover_modules()
            .map_err(|e: RefModuleError| CompositionError::from(e))?;
        
        self.discovered = discovered.iter()
            .map(|d| ModuleInfo::from(d))
            .collect();
        
        Ok(self.discovered.clone())
    }
    
    /// Get module by name and optional version
    pub fn get_module(&self, name: &str, version: Option<&str>) -> Result<ModuleInfo> {
        let module = self.discovered.iter()
            .find(|m| {
                m.name == name && 
                version.map_or(true, |v| m.version == v)
            })
            .ok_or_else(|| {
                let msg = if let Some(v) = version {
                    format!("Module {} version {} not found", name, v)
                } else {
                    format!("Module {} not found", name)
                };
                CompositionError::ModuleNotFound(msg)
            })?;
        
        Ok(module.clone())
    }
    
    /// Install module from source
    pub fn install_module(&mut self, source: ModuleSource) -> Result<ModuleInfo> {
        match source {
            ModuleSource::Path(path) => {
                // Validate path exists
                if !path.exists() {
                    return Err(CompositionError::InstallationFailed(
                        format!("Module path does not exist: {:?}", path)
                    ));
                }
                
                // For now, we'll just discover from the path
                // In a full implementation, this would copy/install the module
                let discovery = RefModuleDiscovery::new(&path);
                let discovered = discovery.discover_modules()
                    .map_err(|e| CompositionError::from(e))?;
                
                if discovered.is_empty() {
                    return Err(CompositionError::InstallationFailed(
                        "No module found at path".to_string()
                    ));
                }
                
                // Refresh discovered modules
                self.discover_modules()?;
                
                Ok(ModuleInfo::from(&discovered[0]))
            }
            ModuleSource::Registry(_url) => {
                // TODO: Implement registry download
                Err(CompositionError::InstallationFailed(
                    "Registry installation not yet implemented".to_string()
                ))
            }
            ModuleSource::Git { url: _, tag: _ } => {
                // TODO: Implement git clone
                Err(CompositionError::InstallationFailed(
                    "Git installation not yet implemented".to_string()
                ))
            }
        }
    }
    
    /// Update module to new version
    pub fn update_module(&mut self, name: &str, new_version: &str) -> Result<ModuleInfo> {
        // Check if module exists
        let _current = self.get_module(name, None)?;
        
        // For now, this is a placeholder
        // In a full implementation, this would:
        // 1. Download new version
        // 2. Verify compatibility
        // 3. Replace old version
        // 4. Restart module if running
        
        Err(CompositionError::InstallationFailed(
            "Module update not yet implemented".to_string()
        ))
    }
    
    /// Remove module
    pub fn remove_module(&mut self, name: &str) -> Result<()> {
        let module = self.get_module(name, None)?;
        
        if let Some(dir) = &module.directory {
            // TODO: Check if module is running and stop it first
            // For now, this is a placeholder
            std::fs::remove_dir_all(dir)
                .map_err(|e| CompositionError::IoError(e))?;
        }
        
        // Refresh discovered modules
        self.discover_modules()?;
        
        Ok(())
    }
    
    /// List all installed modules
    pub fn list_modules(&self) -> Vec<ModuleInfo> {
        self.discovered.clone()
    }
    
    /// Resolve dependencies for a set of modules
    pub fn resolve_dependencies(&self, module_names: &[String]) -> Result<Vec<ModuleInfo>> {
        // First, we need to get the actual RefDiscoveredModule objects
        // We'll need to re-discover or cache them. For now, let's re-discover.
        let discovery = RefModuleDiscovery::new(&self.modules_dir);
        let all_discovered = discovery.discover_modules()
            .map_err(|e| CompositionError::from(e))?;
        
        // Filter to only requested modules and convert to owned values
        let requested: Vec<_> = all_discovered.iter()
            .filter(|d| module_names.contains(&d.manifest.name))
            .cloned()
            .collect();
        
        let resolution = RefModuleDependencies::resolve(&requested)
            .map_err(|e| CompositionError::from(e))?;
        
        // Build result with resolved modules
        let mut resolved = Vec::new();
        for name in &resolution.load_order {
            let module = self.get_module(name, None)?;
            resolved.push(module);
        }
        
        Ok(resolved)
    }
}

