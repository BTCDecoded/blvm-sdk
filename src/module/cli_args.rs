//! CLI argument parsing and type coercion for module commands.
//!
//! Parses `["--key", "value", "-n", "2"]` into a map and coerces values to handler types.

use blvm_node::module::ipc::protocol::CliArgSpec;
use blvm_node::module::traits::ModuleError;
use std::collections::HashMap;

/// Parse CLI args into a map keyed by param name.
///
/// Supports:
/// - `--long_name value` and `-short value`
/// - `--flag` (bool, no value) → param = "true"
/// - Positional fallback: if no `-` prefix, treat as positional by arg order
pub fn parse_args(
    args: &[String],
    arg_specs: &[CliArgSpec],
) -> Result<HashMap<String, String>, ModuleError> {
    let mut map = HashMap::new();

    // Check if we have any -- or - prefixed args (named style)
    let has_named = args.iter().any(|a| {
        a.starts_with("--")
            || (a.starts_with('-')
                && a.len() > 1
                && !a
                    .chars()
                    .nth(1)
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false))
    });

    if has_named {
        // Named parsing: --key value, -k value
        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            if arg.starts_with("--") {
                let key = arg.trim_start_matches('-');
                if key.is_empty() {
                    i += 1;
                    continue;
                }
                let param_name = find_param_by_long(arg_specs, key);
                i += 1;
                if i < args.len() && !args[i].starts_with('-') {
                    let value = args[i].clone();
                    i += 1;
                    map.insert(param_name.to_string(), value);
                } else {
                    // Flag without value (bool)
                    map.insert(param_name.to_string(), "true".to_string());
                }
            } else if arg.starts_with('-') && arg.len() > 1 {
                let short = &arg[1..];
                let param_name = find_param_by_short(arg_specs, short);
                i += 1;
                if i < args.len() && !args[i].starts_with('-') {
                    let value = args[i].clone();
                    i += 1;
                    map.insert(param_name.to_string(), value);
                } else {
                    map.insert(param_name.to_string(), "true".to_string());
                }
            } else {
                i += 1;
            }
        }
    } else {
        // Positional: map by arg order
        for (i, spec) in arg_specs.iter().enumerate() {
            if let Some(v) = args.get(i) {
                map.insert(spec.name.clone(), v.clone());
            }
        }
    }

    Ok(map)
}

fn find_param_by_long<'a>(specs: &'a [CliArgSpec], long: &'a str) -> &'a str {
    for spec in specs {
        let long_form = spec.long_name.as_deref().unwrap_or(&spec.name);
        if long_form == long {
            return &spec.name;
        }
    }
    long
}

fn find_param_by_short<'a>(specs: &'a [CliArgSpec], short: &'a str) -> &'a str {
    for spec in specs {
        if let Some(s) = &spec.short_name {
            if s == short {
                return &spec.name;
            }
        }
        if let Some(c) = short.chars().next() {
            if spec.name.starts_with(c) {
                return &spec.name;
            }
        }
    }
    short
}

/// Coerce a string value to the target type.
///
/// Used by the macro-generated dispatch. For custom types, implement FromStr.
pub fn coerce_bool(s: &str) -> Result<bool, ModuleError> {
    let s = s.to_lowercase();
    if s == "true" || s == "1" || s == "yes" || s == "on" {
        Ok(true)
    } else if s == "false" || s == "0" || s == "no" || s == "off" {
        Ok(false)
    } else {
        Err(ModuleError::Other(format!("invalid bool: {s}")))
    }
}
