use blvm_sdk_macros::config;
use serde::{Deserialize, Serialize};

fn default_n() -> u32 {
    7
}

#[config(name = "probe")]
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
struct ProbeConfig {
    #[serde(default = "default_n")]
    n: u32,
    #[serde(default)]
    name: String,
}

#[test]
fn a_missing_file_is_default_and_a_bad_value_is_an_error() {
    let dir = std::env::temp_dir().join(format!("sdk-cfg-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let missing = dir.join("missing.toml");
    let loaded = ProbeConfig::load(&missing).unwrap();
    assert_eq!(loaded, ProbeConfig::default());

    let empty = dir.join("empty.toml");
    std::fs::write(&empty, "").unwrap();
    let loaded = ProbeConfig::load(&empty).unwrap();
    assert_eq!(loaded.n, 7);
    assert_eq!(loaded.name, "");

    let bad = dir.join("bad.toml");
    std::fs::write(&bad, "n = \"nope\"\nname = \"peer\"\n").unwrap();
    assert!(ProbeConfig::load(&bad).is_err());

    let _ = std::fs::remove_dir_all(&dir);
}
