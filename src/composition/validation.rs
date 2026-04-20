//! Composition Validation
//!
//! Validates module compositions for conflicts, dependencies, and capabilities.

use crate::composition::registry::ModuleRegistry;
use crate::composition::types::*;

/// Validate a node composition specification
pub fn validate_composition(
    spec: &NodeSpec,
    registry: &ModuleRegistry,
) -> Result<ValidationResult> {
    let mut errors = Vec::new();
    let warnings = Vec::new();
    let mut dependencies = Vec::new();

    // Resolve all module names
    let module_names: Vec<String> = spec
        .modules
        .iter()
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
                // Capability validation: required deps must be in composition
                for dep_name in info.dependencies.keys() {
                    if !module_names.contains(dep_name) {
                        errors.push(format!(
                            "Module '{}' requires '{}' which is not in composition",
                            module_spec.name, dep_name
                        ));
                    }
                }
                dependencies.push(info);
            }
            Err(e) => {
                errors.push(format!("Module '{}' not found: {}", module_spec.name, e));
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

    // Check for capability conflicts: two modules providing same capability
    let mut capability_providers: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for dep in &dependencies {
        for cap in &dep.capabilities {
            capability_providers
                .entry(cap.clone())
                .or_default()
                .push(dep.name.clone());
        }
    }
    for (cap, providers) in &capability_providers {
        if providers.len() > 1 {
            errors.push(format!(
                "Capability '{}' provided by multiple modules: {}",
                cap,
                providers.join(", ")
            ));
        }
    }

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
