//! Module bootstrap from environment.
//!
//! When the node spawns a module, it passes `MODULE_ID`, `SOCKET_PATH`, and `DATA_DIR`
//! via environment variables. Use `ModuleBootstrap::from_env()` to read them—no clap needed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Config types that can be loaded and converted to context map.
/// Use `impl_module_config!(ConfigType)` to implement by delegating to inherent `load` and `to_context_map`.
#[cfg(feature = "node")]
pub trait ModuleConfig: Default {
    fn load_config(path: impl AsRef<Path>) -> Result<Self, anyhow::Error>
    where
        Self: Sized;
    /// Return config as key-value map for ModuleContext. Delegates to inherent `to_context_map` via macro.
    fn config_map(&self) -> HashMap<String, String>;
}

/// Implement ModuleConfig by delegating to inherent `load` and `to_context_map`.
#[macro_export]
macro_rules! impl_module_config {
    ($config:ty) => {
        impl $crate::module::ModuleConfig for $config {
            fn load_config(
                path: impl std::convert::AsRef<std::path::Path>,
            ) -> Result<Self, anyhow::Error> {
                <$config>::load(path)
            }
            fn config_map(&self) -> std::collections::HashMap<String, String> {
                self.to_context_map()
            }
        }
    };
}

/// Bootstrap parameters for a module, read from environment.
///
/// The node sets these when spawning. For standalone testing, set them manually:
/// `MODULE_ID`, `SOCKET_PATH`, `DATA_DIR`.
#[derive(Clone, Debug)]
pub struct ModuleBootstrap {
    pub module_id: String,
    pub socket_path: PathBuf,
    pub data_dir: PathBuf,
}

impl ModuleBootstrap {
    /// Read bootstrap params from environment.
    ///
    /// Expects: `MODULE_ID`, `SOCKET_PATH`, `DATA_DIR`.
    pub fn from_env() -> Result<Self, std::env::VarError> {
        Ok(Self {
            module_id: std::env::var("MODULE_ID")?,
            socket_path: PathBuf::from(std::env::var("SOCKET_PATH")?),
            data_dir: PathBuf::from(std::env::var("DATA_DIR")?),
        })
    }

    /// Bootstrap from env, or use defaults when env vars are unset (standalone/testing).
    pub fn from_env_or_defaults(
        module_id: impl Into<String>,
        socket_path: impl Into<PathBuf>,
        data_dir: impl Into<PathBuf>,
    ) -> Self {
        Self::from_env().unwrap_or_else(|_| Self {
            module_id: module_id.into(),
            socket_path: socket_path.into(),
            data_dir: data_dir.into(),
        })
    }

    /// Bootstrap from env, or defaults derived from module name (standalone/testing).
    ///
    /// Uses `data/modules/{name}.sock` and `data/modules/{name}` when env vars are unset.
    pub fn for_module(module_name: &str) -> Self {
        Self::from_env().unwrap_or_else(|_| Self {
            module_id: module_name.to_string(),
            socket_path: PathBuf::from(format!("data/modules/{module_name}.sock")),
            data_dir: PathBuf::from(format!("data/modules/{module_name}")),
        })
    }

    /// Init logging, set DATA_DIR, log startup, and return bootstrap. One-liner for module mains.
    #[cfg(feature = "node")]
    pub fn init_module(module_name: &str) -> Self {
        blvm_node::utils::init_module_logging(module_name.replace('-', "_").as_str(), None);
        let bootstrap = Self::for_module(module_name);
        unsafe {
            std::env::set_var("DATA_DIR", bootstrap.data_dir.to_string_lossy().as_ref());
        }
        tracing::info!(
            "{} starting... (module_id: {}, socket: {:?})",
            module_name,
            bootstrap.module_id,
            bootstrap.socket_path
        );
        bootstrap
    }

    /// Build ModuleContext from bootstrap + data_dir + config (reduces setup boilerplate).
    #[cfg(feature = "node")]
    pub fn context(
        &self,
        data_dir: &Path,
        config: HashMap<String, String>,
    ) -> blvm_node::module::traits::ModuleContext {
        blvm_node::module::traits::ModuleContext {
            module_id: self.module_id.clone(),
            socket_path: self.socket_path.to_string_lossy().to_string(),
            data_dir: data_dir.to_string_lossy().to_string(),
            config,
        }
    }

    /// Load config from `{data_dir}/config.toml` and build ModuleContext. Convention for setup closures.
    #[cfg(feature = "node")]
    pub fn context_with_config<C: ModuleConfig>(
        &self,
        data_dir: &Path,
    ) -> (blvm_node::module::traits::ModuleContext, C) {
        let config = C::load_config(data_dir.join("config.toml")).unwrap_or_default();
        let ctx = self.context(data_dir, config.config_map());
        (ctx, config)
    }
}
