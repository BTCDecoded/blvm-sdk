//! Type Conversions
//!
//! Conversions between developer-sdk composition types and reference-node module types.

use std::collections::HashMap;
use crate::composition::types::ModuleInfo;
use reference_node::module::registry::DiscoveredModule as RefDiscoveredModule;
use reference_node::module::traits::ModuleMetadata as RefModuleMetadata;
use reference_node::module::traits::ModuleError as RefModuleError;

impl From<&RefDiscoveredModule> for ModuleInfo {
    fn from(discovered: &RefDiscoveredModule) -> Self {
        ModuleInfo {
            name: discovered.manifest.name.clone(),
            version: discovered.manifest.version.clone(),
            description: discovered.manifest.description.clone(),
            author: discovered.manifest.author.clone(),
            capabilities: discovered.manifest.capabilities.clone(),
            dependencies: discovered.manifest.dependencies.clone(),
            entry_point: discovered.manifest.entry_point.clone(),
            directory: Some(discovered.directory.clone()),
            binary_path: Some(discovered.binary_path.clone()),
            config_schema: discovered.manifest.config_schema.clone(),
        }
    }
}

impl From<RefDiscoveredModule> for ModuleInfo {
    fn from(discovered: RefDiscoveredModule) -> Self {
        Self::from(&discovered)
    }
}

impl From<&RefModuleMetadata> for ModuleInfo {
    fn from(metadata: &RefModuleMetadata) -> Self {
        ModuleInfo {
            name: metadata.name.clone(),
            version: metadata.version.clone(),
            description: Some(metadata.description.clone()),
            author: Some(metadata.author.clone()),
            capabilities: metadata.capabilities.clone(),
            dependencies: metadata.dependencies.clone(),
            entry_point: metadata.entry_point.clone(),
            directory: None,
            binary_path: None,
            config_schema: HashMap::new(),
        }
    }
}

impl From<RefModuleMetadata> for ModuleInfo {
    fn from(metadata: RefModuleMetadata) -> Self {
        Self::from(&metadata)
    }
}

impl From<ModuleInfo> for RefModuleMetadata {
    fn from(info: ModuleInfo) -> Self {
        RefModuleMetadata {
            name: info.name,
            version: info.version,
            description: info.description.unwrap_or_default(),
            author: info.author.unwrap_or_default(),
            capabilities: info.capabilities,
            dependencies: info.dependencies,
            entry_point: info.entry_point,
        }
    }
}

impl From<RefModuleError> for crate::composition::types::CompositionError {
    fn from(err: RefModuleError) -> Self {
        match err {
            RefModuleError::ModuleNotFound(name) => {
                crate::composition::types::CompositionError::ModuleNotFound(name)
            }
            RefModuleError::InvalidManifest(msg) => {
                crate::composition::types::CompositionError::InvalidConfiguration(msg)
            }
            RefModuleError::OperationError(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(msg)
            }
            RefModuleError::DependencyError(msg) => {
                crate::composition::types::CompositionError::DependencyResolutionFailed(msg)
            }
            RefModuleError::PermissionDenied(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(
                    format!("Permission denied: {}", msg)
                )
            }
            RefModuleError::IpcError(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(
                    format!("IPC error: {}", msg)
                )
            }
            RefModuleError::ProcessError(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(
                    format!("Process error: {}", msg)
                )
            }
        }
    }
}

