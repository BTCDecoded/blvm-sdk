//! # Bitcoin Commons BLLVM Key Generator
//!
//! Generate governance keypairs for Bitcoin governance operations.

use clap::Parser;
use developer_sdk::governance::GovernanceKeypair;
use developer_sdk::cli::output::{OutputFormat, OutputFormatter};
use std::fs;
// No need for Path import

/// Generate governance keypairs
#[derive(Parser, Debug)]
#[command(name = "bllvm-keygen")]
#[command(about = "Generate governance keypairs for Bitcoin Commons governance operations")]
struct Args {
    /// Output file for the keypair
    #[arg(short, long, default_value = "governance.key")]
    output: String,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    format: OutputFormat,

    /// Generate deterministic keypair from seed
    #[arg(long)]
    seed: Option<String>,

    /// Show private key in output
    #[arg(long)]
    show_private: bool,
}

fn main() {
    let args = Args::parse();
    let formatter = OutputFormatter::new(args.format.clone());

    match generate_keypair(&args) {
        Ok(keypair) => {
            let output = format_keypair_output(&keypair, &args, &formatter);
            println!("{}", output);
        }
        Err(e) => {
            eprintln!("{}", formatter.format_error(&*e));
            std::process::exit(1);
        }
    }
}

fn generate_keypair(args: &Args) -> Result<GovernanceKeypair, Box<dyn std::error::Error>> {
    let keypair = if let Some(seed) = &args.seed {
        // Generate deterministic keypair from seed
        let seed_bytes = seed.as_bytes();
        if seed_bytes.len() < 32 {
            return Err("Seed must be at least 32 bytes".into());
        }
        
        let mut seed_array = [0u8; 32];
        seed_array.copy_from_slice(&seed_bytes[..32]);
        GovernanceKeypair::from_secret_key(&seed_array)?
    } else {
        // Generate random keypair
        GovernanceKeypair::generate()?
    };

    // Save keypair to file
    save_keypair(&keypair, &args.output)?;

    Ok(keypair)
}

fn save_keypair(keypair: &GovernanceKeypair, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let keypair_data = serde_json::json!({
        "public_key": hex::encode(keypair.public_key().to_bytes()),
        "secret_key": hex::encode(keypair.secret_key_bytes()),
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    let json_str = serde_json::to_string_pretty(&keypair_data)?;
    fs::write(output_path, json_str)?;

    Ok(())
}

fn format_keypair_output(
    keypair: &GovernanceKeypair,
    args: &Args,
    formatter: &OutputFormatter,
) -> String {
    if args.format == OutputFormat::Json {
        let output_data = serde_json::json!({
            "success": true,
            "public_key": hex::encode(keypair.public_key().to_bytes()),
            "secret_key": if args.show_private {
                Some(hex::encode(keypair.secret_key_bytes()))
            } else {
                None
            },
            "output_file": args.output,
        });
        formatter.format(&output_data).unwrap_or_else(|_| "{}".to_string())
    } else {
        let mut output = "Generated governance keypair\n".to_string();
        output.push_str(&format!("Public key: {}\n", keypair.public_key()));
        if args.show_private {
            output.push_str(&format!("Secret key: {}\n", hex::encode(keypair.secret_key_bytes())));
        }
        output.push_str(&format!("Saved to: {}\n", args.output));
        output
    }
}
