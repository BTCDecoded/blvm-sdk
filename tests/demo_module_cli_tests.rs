//! Unit tests for #[module(name)] CLI spec generation and dispatch.
//!
//! Verifies that #[command] methods with ctx: &InvocationContext produce correct
//! cli_spec subcommands and dispatch_cli routing.

use blvm_sdk::migrations;
use blvm_sdk::module::prelude::*;
use blvm_sdk::module::{open_module_db, run_migrations, MigrationContext};
use tempfile::TempDir;

#[migration(version = 1)]
fn up_test(ctx: &MigrationContext) -> anyhow::Result<()> {
    ctx.put(b"schema_version", b"1")?;
    Ok(())
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[config(name = "test-cli")]
pub struct TestConfig {}

#[derive(Clone)]
#[module(name = "test-cli")]
pub struct TestCliModule {
    #[allow(dead_code)]
    config: TestConfig,
}

#[module(name = "test-cli")]
impl TestCliModule {
    #[command]
    fn set(
        &self,
        ctx: &InvocationContext,
        key: String,
        value: String,
    ) -> Result<String, ModuleError> {
        let tree = ctx
            .db()
            .open_tree("items")
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        tree.insert(key.as_bytes(), value.as_bytes())
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        Ok(format!("Set {key}={value}\n"))
    }

    #[command]
    fn get(&self, ctx: &InvocationContext, key: String) -> Result<String, ModuleError> {
        let tree = ctx
            .db()
            .open_tree("items")
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        let value = tree
            .get(key.as_bytes())
            .map_err(|e| ModuleError::Other(e.to_string()))?
            .map(|v| String::from_utf8_lossy(&v).into_owned())
            .unwrap_or_else(|| "<not found>".into());
        Ok(format!("{key}={value}\n"))
    }

    #[command]
    fn list(&self, ctx: &InvocationContext) -> Result<String, ModuleError> {
        let tree = ctx
            .db()
            .open_tree("items")
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        let items: Vec<String> = tree
            .iter()
            .filter_map(|r| r.ok())
            .map(|(k, v)| {
                format!(
                    "{}={}",
                    String::from_utf8_lossy(&k),
                    String::from_utf8_lossy(&v)
                )
            })
            .collect();
        Ok(if items.is_empty() {
            "(empty)\n".into()
        } else {
            items.join("\n") + "\n"
        })
    }

    #[command]
    fn delete(&self, ctx: &InvocationContext, key: String) -> Result<String, ModuleError> {
        let tree = ctx
            .db()
            .open_tree("items")
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        tree.remove(key.as_bytes())
            .map_err(|e| ModuleError::Other(e.to_string()))?;
        Ok(format!("Deleted {key}\n"))
    }
}

#[test]
fn test_cli_spec_has_subcommands() {
    let spec = TestCliModule::cli_spec();
    assert_eq!(spec.name, "test-cli");
    assert_eq!(spec.version, 1);
    let sub_names: Vec<&str> = spec.subcommands.iter().map(|s| s.name.as_str()).collect();
    assert!(
        sub_names.contains(&"set"),
        "cli_spec should include 'set' subcommand, got: {sub_names:?}"
    );
    assert!(
        sub_names.contains(&"get"),
        "cli_spec should include 'get' subcommand, got: {sub_names:?}"
    );
    assert!(
        sub_names.contains(&"list"),
        "cli_spec should include 'list' subcommand, got: {sub_names:?}"
    );
    assert!(
        sub_names.contains(&"delete"),
        "cli_spec should include 'delete' subcommand, got: {sub_names:?}"
    );
    assert_eq!(
        sub_names.len(),
        4,
        "expected 4 subcommands, got: {sub_names:?}"
    );
}

#[test]
fn test_dispatch_cli_set_get_list_delete() {
    let temp = TempDir::new().unwrap();
    let db = open_module_db(temp.path()).unwrap();
    run_migrations(&db, migrations!(1 => up_test)).unwrap();

    let module = TestCliModule {
        config: TestConfig::default(),
    };
    let ctx = InvocationContext::new(db);

    // set key=value
    let out = module
        .dispatch_cli(
            &ctx,
            "set",
            &["--key".into(), "foo".into(), "--value".into(), "bar".into()],
        )
        .unwrap();
    assert!(out.contains("Set foo=bar"), "set output: {out}");

    // get key
    let out = module
        .dispatch_cli(&ctx, "get", &["--key".into(), "foo".into()])
        .unwrap();
    assert!(out.contains("foo=bar"), "get output: {out}");

    // list
    let out = module.dispatch_cli(&ctx, "list", &[]).unwrap();
    assert!(out.contains("foo=bar"), "list output: {out}");

    // delete
    let out = module
        .dispatch_cli(&ctx, "delete", &["--key".into(), "foo".into()])
        .unwrap();
    assert!(out.contains("Deleted foo"), "delete output: {out}");

    // get after delete
    let out = module
        .dispatch_cli(&ctx, "get", &["--key".into(), "foo".into()])
        .unwrap();
    assert!(out.contains("<not found>"), "get after delete: {out}");

    // unknown subcommand
    let err = module.dispatch_cli(&ctx, "unknown", &[]).unwrap_err();
    assert!(err.to_string().contains("Unknown subcommand"));
}

#[test]
fn test_dispatch_cli_positional_args() {
    let temp = TempDir::new().unwrap();
    let db = open_module_db(temp.path()).unwrap();
    run_migrations(&db, migrations!(1 => up_test)).unwrap();

    let module = TestCliModule {
        config: TestConfig::default(),
    };
    let ctx = InvocationContext::new(db);

    // positional: key value
    let out = module
        .dispatch_cli(&ctx, "set", &["x".into(), "y".into()])
        .unwrap();
    assert!(out.contains("Set x=y"), "positional set: {out}");

    let out = module.dispatch_cli(&ctx, "get", &["x".into()]).unwrap();
    assert!(out.contains("x=y"), "positional get: {out}");
}
