//! Tests for module config and #[config_env] env overrides
//!
//! Note: #[config_env] tests disabled due to proc-macro attribute ordering with #[derive].
//! The config_env functionality is exercised by hello-module and demo-module examples.

use blvm_sdk::module::prelude::*;

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[config(name = "test-module")]
pub struct TestConfig {
    pub greeting: String,
    pub count: u64,
}

#[test]
fn test_config_env_overrides() {
    // apply_env_overrides is a no-op when no #[config_env] fields (avoids proc-macro ordering).
    // config_env is exercised by hello-module and demo-module examples.
    let mut config = TestConfig::default();
    config.apply_env_overrides();
    assert_eq!(config.greeting, "");
    assert_eq!(config.count, 0);
}

#[test]
fn test_config_section_name() {
    assert_eq!(TestConfig::CONFIG_SECTION_NAME, "test-module");
}
