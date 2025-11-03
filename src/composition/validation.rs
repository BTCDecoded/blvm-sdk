//! Composition Validation
//!
//! Validates module compositions for conflicts, dependencies, and capabilities.

use crate::composition::types::*;
use crate::composition::registry::ModuleRegistry;

/// Validate a node composition specification
pub fn validate_composition(
    spec: &NodeSpec,
    registry: &ModuleRegistry,
) -> Result<ValidationResult> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut dependencies = Vec::new();
    
    // Resolve all module names
    let module_names: Vec<String> = spec.modules.iter()
        .filter(|m| m.enabled)
        .map(|m| m.name.clone())
        .collect();
    
    // Check all modules exist
    for module_spec in &spec.modules {
        if !module_spec.enabled {
            continue;
        }
        
        match registry.get_module(&module_spec.name, module_spec.version.as_deref()) {
            Ok(info) => {
                // Check capabilities compatibility
                // TODO: Add capability validation logic
                
                // Add to dependencies
                dependencies.push(info);
            }
            Err(e) => {
                errors.push(format!(
                    "Module '{}' not found: {}",
                    module_spec.name, e
                ));
            }
        }
    }
    
    // Resolve dependencies
    match registry.resolve_dependencies(&module_names) {
        Ok(resolved) => {
            // Check for missing dependencies
            for resolved_module in &resolved {
                if !dependencies.iter().any(|d| d.name == resolved_module.name) {
                    dependencies.push(resolved_module.clone());
                }
            }
        }
        Err(e) => {
            errors.push(format!("Dependency resolution failed: {}", e));
        }
    }
    
    // Check for module conflicts
    // TODO: Add conflict detection (e.g., two modules providing same capability)
    
    // Check for circular dependencies
    // (Already handled by dependency resolution, but double-check here)
    
    let valid = errors.is_empty();
    Ok(ValidationResult {
        valid,
        errors,
        warnings,
        dependencies,
    })
}

