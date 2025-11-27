//! Type Conversions
//!
//! Conversions between bllvm-sdk composition types and bllvm-node module types.

use crate::composition::types::ModuleInfo;
use bllvm_node::module::registry::DiscoveredModule as RefDiscoveredModule;
use bllvm_node::module::traits::ModuleError as RefModuleError;
use bllvm_node::module::traits::ModuleMetadata as RefModuleMetadata;
use std::collections::HashMap;

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
            RefModuleError::CryptoError(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(format!(
                    "Crypto error: {msg}"
                ))
            }
            RefModuleError::ModuleNotFound(name) => {
                crate::composition::types::CompositionError::ModuleNotFound(name)
            }
            RefModuleError::InvalidManifest(msg) => {
                crate::composition::types::CompositionError::InvalidConfiguration(msg)
            }
            RefModuleError::OperationError(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(msg)
            }
            RefModuleError::DependencyMissing(msg) => {
                crate::composition::types::CompositionError::DependencyResolutionFailed(msg)
            }
            RefModuleError::PermissionDenied(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(format!(
                    "Permission denied: {}",
                    msg
                ))
            }
            RefModuleError::IpcError(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(format!(
                    "IPC error: {}",
                    msg
                ))
            }
            RefModuleError::InitializationError(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(format!(
                    "Initialization error: {}",
                    msg
                ))
            }
            RefModuleError::VersionIncompatible(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(format!(
                    "Version incompatible: {}",
                    msg
                ))
            }
            RefModuleError::ModuleCrashed(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(format!(
                    "Module crashed: {}",
                    msg
                ))
            }
            RefModuleError::SerializationError(msg) => {
                crate::composition::types::CompositionError::SerializationError(msg)
            }
            RefModuleError::RateLimitExceeded(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(format!(
                    "Rate limit exceeded: {}",
                    msg
                ))
            }
            RefModuleError::Timeout => {
                crate::composition::types::CompositionError::InstallationFailed(
                    "Timeout waiting for module response".to_string(),
                )
            }
            RefModuleError::ResourceLimitExceeded(msg) => {
                crate::composition::types::CompositionError::InstallationFailed(format!(
                    "Resource limit exceeded: {}",
                    msg
                ))
            }
        }
    }
}
