//! Composition Framework Tests
//!
//! Tests for node composition, module registry, lifecycle, and configuration.

use bllvm_sdk::composition::{
    NodeComposer, ModuleRegistry, ModuleLifecycle,
    NodeConfig, NodeSpec, ModuleSpec, NetworkType,
    ModuleStatus, ModuleHealth, NodeStatus,
    ModuleSource, ValidationResult, Result,
};
use bllvm_sdk::composition::config::NodeMetadata;
use bllvm_sdk::composition::validation::validate_composition;
use bllvm_sdk::composition::schema::validate_config_schema;
use std::collections::HashMap;
use tempfile::TempDir;

/// Test helper: Create a temporary directory for modules
fn create_temp_modules_dir() -> TempDir {
    tempfile::tempdir().unwrap()
}

// ============================================================================
// Phase 1: ModuleRegistry Tests
// ============================================================================

#[test]
fn test_module_registry_creation() {
    // Test creating a module registry
    let temp_dir = create_temp_modules_dir();
    let registry = ModuleRegistry::new(temp_dir.path());
    
    // Registry should be created
    // Note: We can't easily test discovery without actual modules, but we can test structure
    // Registry is created successfully
    assert!(temp_dir.path().exists());
}

#[test]
fn test_module_registry_discover_modules() {
    // Test discovering modules (may be empty if no modules present)
    let temp_dir = create_temp_modules_dir();
    let mut registry = ModuleRegistry::new(temp_dir.path());
    
    let result = registry.discover_modules();
    // Should succeed even if no modules found
    assert!(result.is_ok());
}

#[test]
fn test_module_registry_get_module_not_found() {
    // Test getting non-existent module
    let temp_dir = create_temp_modules_dir();
    let registry = ModuleRegistry::new(temp_dir.path());
    
    let result = registry.get_module("nonexistent", None);
    assert!(result.is_err());
}

#[test]
fn test_module_registry_install_from_path() {
    // Test installing module from path
    let temp_dir = create_temp_modules_dir();
    let mut registry = ModuleRegistry::new(temp_dir.path());
    
    // Try to install from non-existent path (should fail)
    let source = ModuleSource::Path(temp_dir.path().join("nonexistent"));
    let result = registry.install_module(source);
    assert!(result.is_err());
}

// ============================================================================
// Phase 2: ModuleLifecycle Tests
// ============================================================================

#[test]
fn test_module_lifecycle_creation() {
    // Test creating a module lifecycle manager
    let temp_dir = create_temp_modules_dir();
    let registry = ModuleRegistry::new(temp_dir.path());
    let lifecycle = ModuleLifecycle::new(registry);
    
    // Lifecycle should be created
    // Note: We can't easily test start/stop without actual modules
    // Lifecycle is created successfully
    assert!(temp_dir.path().exists());
}

// ============================================================================
// Phase 3: NodeConfig Tests
// ============================================================================

#[test]
fn test_node_config_default() {
    // Test default node metadata
    let metadata = NodeMetadata::default();
    
    assert_eq!(metadata.name, "custom-node");
    assert_eq!(metadata.network, "mainnet");
    assert!(metadata.version.is_none());
}

#[test]
fn test_node_config_creation() {
    // Test creating a node config
    let config = NodeConfig {
        node: NodeMetadata {
            name: "test-node".to_string(),
            version: Some("1.0.0".to_string()),
            network: "testnet".to_string(),
        },
        modules: HashMap::new(),
    };
    
    assert_eq!(config.node.name, "test-node");
    assert_eq!(config.node.network, "testnet");
}

#[test]
fn test_node_config_to_spec() {
    // Test converting config to spec
    let config = NodeConfig {
        node: NodeMetadata {
            name: "test-node".to_string(),
            version: Some("1.0.0".to_string()),
            network: "mainnet".to_string(),
        },
        modules: HashMap::new(),
    };
    
    let spec = config.to_spec().unwrap();
    assert_eq!(spec.name, "test-node");
    assert_eq!(spec.network, NetworkType::Mainnet);
}

#[test]
fn test_node_config_to_spec_testnet() {
    // Test converting testnet config to spec
    let config = NodeConfig {
        node: NodeMetadata {
            name: "test-node".to_string(),
            version: None,
            network: "testnet".to_string(),
        },
        modules: HashMap::new(),
    };
    
    let spec = config.to_spec().unwrap();
    assert_eq!(spec.network, NetworkType::Testnet);
}

#[test]
fn test_node_config_to_spec_regtest() {
    // Test converting regtest config to spec
    let config = NodeConfig {
        node: NodeMetadata {
            name: "test-node".to_string(),
            version: None,
            network: "regtest".to_string(),
        },
        modules: HashMap::new(),
    };
    
    let spec = config.to_spec().unwrap();
    assert_eq!(spec.network, NetworkType::Regtest);
}

#[test]
fn test_node_config_invalid_network() {
    // Test invalid network type
    let config = NodeConfig {
        node: NodeMetadata {
            name: "test-node".to_string(),
            version: None,
            network: "invalid".to_string(),
        },
        modules: HashMap::new(),
    };
    
    let result = config.to_spec();
    assert!(result.is_err());
}

// ============================================================================
// Phase 4: NodeSpec Tests
// ============================================================================

#[test]
fn test_node_spec_creation() {
    // Test creating a node spec
    let spec = NodeSpec {
        name: "test-node".to_string(),
        version: Some("1.0.0".to_string()),
        network: NetworkType::Mainnet,
        modules: vec![],
    };
    
    assert_eq!(spec.name, "test-node");
    assert_eq!(spec.network, NetworkType::Mainnet);
}

#[test]
fn test_node_spec_with_modules() {
    // Test node spec with modules
    let spec = NodeSpec {
        name: "test-node".to_string(),
        version: None,
        network: NetworkType::Testnet,
        modules: vec![
            ModuleSpec {
                name: "module1".to_string(),
                version: Some("1.0.0".to_string()),
                enabled: true,
                config: HashMap::new(),
            },
            ModuleSpec {
                name: "module2".to_string(),
                version: None,
                enabled: false,
                config: HashMap::new(),
            },
        ],
    };
    
    assert_eq!(spec.modules.len(), 2);
    assert!(spec.modules[0].enabled);
    assert!(!spec.modules[1].enabled);
}

// ============================================================================
// Phase 5: ModuleSpec Tests
// ============================================================================

#[test]
fn test_module_spec_creation() {
    // Test creating a module spec
    let module_spec = ModuleSpec {
        name: "test-module".to_string(),
        version: Some("1.0.0".to_string()),
        enabled: true,
        config: HashMap::new(),
    };
    
    assert_eq!(module_spec.name, "test-module");
    assert_eq!(module_spec.version, Some("1.0.0".to_string()));
    assert!(module_spec.enabled);
}

#[test]
fn test_module_spec_disabled() {
    // Test disabled module spec
    let module_spec = ModuleSpec {
        name: "test-module".to_string(),
        version: None,
        enabled: false,
        config: HashMap::new(),
    };
    
    assert!(!module_spec.enabled);
}

#[test]
fn test_module_spec_with_config() {
    // Test module spec with configuration
    let mut config = HashMap::new();
    config.insert("key1".to_string(), serde_json::json!("value1"));
    config.insert("key2".to_string(), serde_json::json!(42));
    
    let module_spec = ModuleSpec {
        name: "test-module".to_string(),
        version: None,
        enabled: true,
        config,
    };
    
    assert_eq!(module_spec.config.len(), 2);
}

// ============================================================================
// Phase 6: NetworkType Tests
// ============================================================================

#[test]
fn test_network_type_variants() {
    // Test all network type variants
    assert_eq!(NetworkType::Mainnet as u8, 0);
    assert_eq!(NetworkType::Testnet as u8, 1);
    assert_eq!(NetworkType::Regtest as u8, 2);
}

#[test]
fn test_network_type_equality() {
    // Test network type equality
    let mainnet1 = NetworkType::Mainnet;
    let mainnet2 = NetworkType::Mainnet;
    let testnet = NetworkType::Testnet;
    
    assert_eq!(mainnet1, mainnet2);
    assert_ne!(mainnet1, testnet);
}

// ============================================================================
// Phase 7: ModuleStatus Tests
// ============================================================================

#[test]
fn test_module_status_variants() {
    // Test module status variants
    let not_installed = ModuleStatus::NotInstalled;
    let stopped = ModuleStatus::Stopped;
    let running = ModuleStatus::Running;
    let error = ModuleStatus::Error("test error".to_string());
    
    assert_eq!(not_installed, ModuleStatus::NotInstalled);
    assert_eq!(stopped, ModuleStatus::Stopped);
    assert_eq!(running, ModuleStatus::Running);
    assert_eq!(error, ModuleStatus::Error("test error".to_string()));
}

#[test]
fn test_module_health_variants() {
    // Test module health variants
    let healthy = ModuleHealth::Healthy;
    let degraded = ModuleHealth::Degraded;
    let unhealthy = ModuleHealth::Unhealthy("test".to_string());
    let unknown = ModuleHealth::Unknown;
    
    assert_eq!(healthy, ModuleHealth::Healthy);
    assert_eq!(degraded, ModuleHealth::Degraded);
    assert_eq!(unhealthy, ModuleHealth::Unhealthy("test".to_string()));
    assert_eq!(unknown, ModuleHealth::Unknown);
}

#[test]
fn test_node_status_variants() {
    // Test node status variants
    let stopped = NodeStatus::Stopped;
    let starting = NodeStatus::Starting;
    let running = NodeStatus::Running;
    let error = NodeStatus::Error("test error".to_string());
    
    assert_eq!(stopped, NodeStatus::Stopped);
    assert_eq!(starting, NodeStatus::Starting);
    assert_eq!(running, NodeStatus::Running);
    assert_eq!(error, NodeStatus::Error("test error".to_string()));
}

// ============================================================================
// Phase 8: Schema Validation Tests
// ============================================================================

#[test]
fn test_validate_config_schema_valid() {
    // Test validating a valid config schema
    let config = NodeConfig {
        node: NodeMetadata {
            name: "test-node".to_string(),
            version: Some("1.0.0".to_string()),
            network: "mainnet".to_string(),
        },
        modules: HashMap::new(),
    };
    
    let result = validate_config_schema(&config).unwrap();
    assert!(result.valid);
    assert!(result.errors.is_empty());
}

#[test]
fn test_validate_config_schema_empty_name() {
    // Test validation fails with empty node name
    let config = NodeConfig {
        node: NodeMetadata {
            name: "".to_string(),
            version: None,
            network: "mainnet".to_string(),
        },
        modules: HashMap::new(),
    };
    
    let result = validate_config_schema(&config).unwrap();
    assert!(!result.valid);
    assert!(!result.errors.is_empty());
}

#[test]
fn test_validate_config_schema_invalid_network() {
    // Test validation fails with invalid network
    let config = NodeConfig {
        node: NodeMetadata {
            name: "test-node".to_string(),
            version: None,
            network: "invalid".to_string(),
        },
        modules: HashMap::new(),
    };
    
    let result = validate_config_schema(&config).unwrap();
    assert!(!result.valid);
    assert!(!result.errors.is_empty());
}

#[test]
fn test_validate_config_schema_module_warning() {
    // Test validation warns about missing module version
    use bllvm_sdk::composition::config::ModuleConfig;
    let mut modules = HashMap::new();
    modules.insert("test-module".to_string(), ModuleConfig {
        enabled: true,
        version: None,
        config: HashMap::new(),
    });
    
    let config = NodeConfig {
        node: NodeMetadata {
            name: "test-node".to_string(),
            version: None,
            network: "mainnet".to_string(),
        },
        modules,
    };
    
    let result = validate_config_schema(&config).unwrap();
    assert!(result.valid);
    assert!(!result.warnings.is_empty());
}

// ============================================================================
// Phase 9: Composition Validation Tests
// ============================================================================

#[test]
fn test_validate_composition_empty() {
    // Test validating empty composition
    let temp_dir = create_temp_modules_dir();
    let registry = ModuleRegistry::new(temp_dir.path());
    
    let spec = NodeSpec {
        name: "test-node".to_string(),
        version: None,
        network: NetworkType::Mainnet,
        modules: vec![],
    };
    
    let result = validate_composition(&spec, &registry).unwrap();
    // Empty composition should be valid
    assert!(result.valid);
}

#[test]
fn test_validate_composition_nonexistent_module() {
    // Test validation fails with non-existent module
    let temp_dir = create_temp_modules_dir();
    let registry = ModuleRegistry::new(temp_dir.path());
    
    let spec = NodeSpec {
        name: "test-node".to_string(),
        version: None,
        network: NetworkType::Mainnet,
        modules: vec![
            ModuleSpec {
                name: "nonexistent".to_string(),
                version: None,
                enabled: true,
                config: HashMap::new(),
            },
        ],
    };
    
    let result = validate_composition(&spec, &registry).unwrap();
    // Should fail because module doesn't exist
    assert!(!result.valid);
    assert!(!result.errors.is_empty());
}

#[test]
fn test_validate_composition_disabled_module() {
    // Test validation skips disabled modules
    let temp_dir = create_temp_modules_dir();
    let registry = ModuleRegistry::new(temp_dir.path());
    
    let spec = NodeSpec {
        name: "test-node".to_string(),
        version: None,
        network: NetworkType::Mainnet,
        modules: vec![
            ModuleSpec {
                name: "nonexistent".to_string(),
                version: None,
                enabled: false, // Disabled, should be skipped
                config: HashMap::new(),
            },
        ],
    };
    
    let result = validate_composition(&spec, &registry).unwrap();
    // Should be valid because disabled module is skipped
    assert!(result.valid);
}

// ============================================================================
// Phase 10: NodeComposer Tests
// ============================================================================

#[test]
fn test_node_composer_creation() {
    // Test creating a node composer
    let temp_dir = create_temp_modules_dir();
    let composer = NodeComposer::new(temp_dir.path());
    
    // Composer should be created
    // Note: We can't easily test composition without actual modules
    assert!(composer.validate_composition(&NodeSpec {
        name: "test".to_string(),
        version: None,
        network: NetworkType::Mainnet,
        modules: vec![],
    }).is_ok());
}

#[test]
fn test_node_composer_validate_composition() {
    // Test validating composition via composer
    let temp_dir = create_temp_modules_dir();
    let composer = NodeComposer::new(temp_dir.path());
    
    let spec = NodeSpec {
        name: "test-node".to_string(),
        version: None,
        network: NetworkType::Mainnet,
        modules: vec![],
    };
    
    let result = composer.validate_composition(&spec).unwrap();
    assert!(result.valid);
}

// ============================================================================
// Phase 11: ModuleSource Tests
// ============================================================================

#[test]
fn test_module_source_path() {
    // Test ModuleSource::Path variant
    let temp_dir = create_temp_modules_dir();
    let source = ModuleSource::Path(temp_dir.path().to_path_buf());
    
    match source {
        ModuleSource::Path(path) => {
            assert_eq!(path, temp_dir.path());
        }
        _ => panic!("Expected Path variant"),
    }
}

#[test]
fn test_module_source_registry() {
    // Test ModuleSource::Registry variant
    let source = ModuleSource::Registry("https://example.com/registry".to_string());
    
    match source {
        ModuleSource::Registry(url) => {
            assert_eq!(url, "https://example.com/registry");
        }
        _ => panic!("Expected Registry variant"),
    }
}

#[test]
fn test_module_source_git() {
    // Test ModuleSource::Git variant
    let source = ModuleSource::Git {
        url: "https://github.com/example/repo".to_string(),
        tag: Some("v1.0.0".to_string()),
    };
    
    match source {
        ModuleSource::Git { url, tag } => {
            assert_eq!(url, "https://github.com/example/repo");
            assert_eq!(tag, Some("v1.0.0".to_string()));
        }
        _ => panic!("Expected Git variant"),
    }
}

// ============================================================================
// Phase 12: ValidationResult Tests
// ============================================================================

#[test]
fn test_validation_result_valid() {
    // Test valid validation result
    let result = ValidationResult {
        valid: true,
        errors: vec![],
        warnings: vec![],
        dependencies: vec![],
    };
    
    assert!(result.valid);
    assert!(result.errors.is_empty());
}

#[test]
fn test_validation_result_invalid() {
    // Test invalid validation result
    let result = ValidationResult {
        valid: false,
        errors: vec!["Error 1".to_string(), "Error 2".to_string()],
        warnings: vec!["Warning 1".to_string()],
        dependencies: vec![],
    };
    
    assert!(!result.valid);
    assert_eq!(result.errors.len(), 2);
    assert_eq!(result.warnings.len(), 1);
}

