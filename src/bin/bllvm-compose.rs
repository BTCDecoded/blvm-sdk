//! bllvm-compose - Node Composition CLI Tool
//!
//! Command-line interface for composing Bitcoin nodes from modules.

use clap::{Parser, Subcommand};
use developer_sdk::composition::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "bllvm-compose")]
#[command(about = "Compose Bitcoin nodes from modules", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Modules directory path
    #[arg(long, default_value = "./modules")]
    modules_dir: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Compose a node from configuration file
    Compose {
        /// Configuration file path
        #[arg(short, long)]
        config: PathBuf,
    },
    
    /// Validate a composition configuration
    Validate {
        /// Configuration file path
        #[arg(short, long)]
        config: PathBuf,
    },
    
    /// Generate a configuration template
    GenerateTemplate {
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Module registry operations
    #[command(subcommand)]
    Modules(ModuleCommands),
}

#[derive(Subcommand)]
enum ModuleCommands {
    /// List available modules
    List,
    
    /// Install a module
    Install {
        /// Module source (path, registry URL, or git URL)
        source: String,
        
        /// Module version (optional)
        #[arg(short, long)]
        version: Option<String>,
    },
    
    /// Update a module
    Update {
        /// Module name
        name: String,
        
        /// New version
        version: String,
    },
    
    /// Remove a module
    Remove {
        /// Module name
        name: String,
    },
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    let mut composer = NodeComposer::new(&cli.modules_dir);
    
    match cli.command {
        Some(Commands::Compose { config }) => {
            println!("Composing node from configuration: {:?}", config);
            let composed = composer.compose_from_config(&config).await?;
            println!("Successfully composed node: {}", composed.spec.name);
            println!("Modules: {}", composed.modules.len());
            for module in &composed.modules {
                println!("  - {} ({}): {:?}", module.info.name, module.info.version, module.status);
            }
            Ok(())
        }
        
        Some(Commands::Validate { config }) => {
            println!("Validating configuration: {:?}", config);
            let node_config = NodeConfig::from_file(&config)?;
            let validation = composer.validate_composition(&node_config.to_spec()?)?;
            
            if validation.valid {
                println!("✓ Configuration is valid");
                if !validation.warnings.is_empty() {
                    println!("Warnings:");
                    for warning in &validation.warnings {
                        println!("  - {}", warning);
                    }
                }
                Ok(())
            } else {
                println!("✗ Configuration is invalid:");
                for error in &validation.errors {
                    println!("  - {}", error);
                }
                std::process::exit(1)
            }
        }
        
        Some(Commands::GenerateTemplate { output }) => {
            let template = composer.generate_config();
            
            if let Some(path) = output {
                std::fs::write(&path, template)?;
                println!("Template written to: {:?}", path);
            } else {
                print!("{}", template);
            }
            Ok(())
        }
        
        Some(Commands::Modules(ModuleCommands::List)) => {
            composer.registry_mut().discover_modules()?;
            let modules = composer.registry().list_modules();
            
            if modules.is_empty() {
                println!("No modules found in {:?}", cli.modules_dir);
            } else {
                println!("Available modules:");
                for module in modules {
                    println!("  - {} ({})", module.name, module.version);
                    if let Some(desc) = &module.description {
                        println!("    {}", desc);
                    }
                }
            }
            Ok(())
        }
        
        Some(Commands::Modules(ModuleCommands::Install { source, version: _ })) => {
            let module_source = if source.starts_with("http://") || source.starts_with("https://") {
                ModuleSource::Registry(source)
            } else if source.starts_with("git+") || source.contains("github.com") {
                ModuleSource::Git {
                    url: source,
                    tag: None,
                }
            } else {
                ModuleSource::Path(PathBuf::from(source))
            };
            
            println!("Installing module from: {:?}", module_source);
            let module = composer.registry_mut().install_module(module_source)?;
            println!("Successfully installed: {} ({})", module.name, module.version);
            Ok(())
        }
        
        Some(Commands::Modules(ModuleCommands::Update { name, version })) => {
            println!("Updating module {} to version {}", name, version);
            let module = composer.registry_mut().update_module(&name, &version)?;
            println!("Successfully updated: {} ({})", module.name, module.version);
            Ok(())
        }
        
        Some(Commands::Modules(ModuleCommands::Remove { name })) => {
            println!("Removing module: {}", name);
            composer.registry_mut().remove_module(&name)?;
            println!("Successfully removed: {}", name);
            Ok(())
        }
        
        None => {
            println!("No command specified. Use --help for usage.");
            Ok(())
        }
    }
}

