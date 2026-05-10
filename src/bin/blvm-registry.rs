//! blvm-registry - Registry index generator
//!
//! Scans a modules directory for module.toml files and emits a JSON registry index.
//! Use for building a registry from local module sources.
//!
//! Example:
//!   blvm-registry index --dir ./modules --output index.json

use blvm_node::module::registry::ModuleDiscovery;
use blvm_node::module::traits::ModuleError;
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "blvm-registry")]
#[command(about = "BLVM module registry index generator")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate registry index from module.toml files in a directory
    Index {
        /// Directory containing modules (subdirs with module.toml)
        #[arg(short, long, default_value = "./modules")]
        dir: PathBuf,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Base URL for download_url (e.g. https://registry.example.com/)
        #[arg(long)]
        registry_url: Option<String>,
    },
}

#[derive(Serialize)]
struct RegistryIndex {
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    registry_url: Option<String>,
    updated_at: String,
    modules: Vec<RegistryModule>,
}

#[derive(Serialize)]
struct RegistryModule {
    name: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    download_url: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    capabilities: Vec<String>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    dependencies: std::collections::HashMap<String, String>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    optional_dependencies: std::collections::HashMap<String, String>,
}

fn run_index(
    dir: PathBuf,
    output: Option<PathBuf>,
    registry_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let discovery = ModuleDiscovery::new(&dir);
    let discovered = discovery
        .discover_modules()
        .map_err(|e: ModuleError| e.to_string())?;

    let modules: Vec<RegistryModule> = discovered
        .iter()
        .map(|d| {
            let download_url = registry_url.as_ref().map(|base| {
                let base = base.trim_end_matches('/');
                format!(
                    "{}/modules/{}-{}.tar.gz",
                    base, d.manifest.name, d.manifest.version
                )
            });
            RegistryModule {
                name: d.manifest.name.clone(),
                version: d.manifest.version.clone(),
                description: d.manifest.description.clone(),
                author: d.manifest.author.clone(),
                download_url,
                capabilities: d.manifest.capabilities.clone(),
                dependencies: d.manifest.dependencies.clone(),
                optional_dependencies: d.manifest.optional_dependencies.clone(),
            }
        })
        .collect();

    let index = RegistryIndex {
        version: "1".to_string(),
        registry_url: registry_url.clone(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        modules,
    };

    let json = serde_json::to_string_pretty(&index)?;

    if let Some(path) = output {
        std::fs::write(&path, json)?;
        println!(
            "Wrote {} modules to {}",
            index.modules.len(),
            path.display()
        );
    } else {
        println!("{json}");
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Index {
            dir,
            output,
            registry_url,
        }) => run_index(dir, output, registry_url),
        None => {
            println!("Run with --help for usage");
            Ok(())
        }
    }
}
