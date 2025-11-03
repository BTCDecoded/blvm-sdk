//! # Bitcoin Commons BLLVM Signer
//!
//! Sign governance messages for Bitcoin Commons governance operations.

use clap::{Parser, Subcommand};
use developer_sdk::cli::output::{OutputFormat, OutputFormatter};
use developer_sdk::governance::{GovernanceKeypair, GovernanceMessage, Signature};
use developer_sdk::sign_message as crypto_sign_message;
use std::fs;
use std::path::Path;

/// Sign governance messages
#[derive(Parser, Debug)]
#[command(name = "bllvm-sign")]
#[command(about = "Sign governance messages for Bitcoin Commons governance operations")]
struct Args {
    /// Output file for the signature
    #[arg(short, long, default_value = "signature.txt")]
    output: String,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    format: OutputFormat,

    /// Private key file
    #[arg(short, long, required = true)]
    key: String,

    /// Message to sign
    #[command(subcommand)]
    message: MessageCommand,
}

#[derive(Subcommand, Debug)]
enum MessageCommand {
    /// Sign a release message
    Release {
        /// Version string
        #[arg(short, long, required = true)]
        version: String,

        /// Commit hash
        #[arg(short, long, required = true)]
        commit: String,
    },
    /// Sign a module approval message
    Module {
        /// Module name
        #[arg(short, long, required = true)]
        name: String,

        /// Module version
        #[arg(short, long, required = true)]
        version: String,
    },
    /// Sign a budget decision message
    Budget {
        /// Amount in satoshis
        #[arg(short, long, required = true)]
        amount: u64,

        /// Purpose description
        #[arg(short, long, required = true)]
        purpose: String,
    },
}

fn main() {
    let args = Args::parse();
    let formatter = OutputFormatter::new(args.format.clone());

    match sign_message(&args) {
        Ok(signature) => {
            let output = format_signature_output(&signature, &args, &formatter);
            println!("{}", output);
        }
        Err(e) => {
            eprintln!("{}", formatter.format_error(&*e));
            std::process::exit(1);
        }
    }
}

fn sign_message(args: &Args) -> Result<Signature, Box<dyn std::error::Error>> {
    // Load the keypair
    let keypair = load_keypair(&args.key)?;

    // Create the message
    let message = match &args.message {
        MessageCommand::Release { version, commit } => GovernanceMessage::Release {
            version: version.clone(),
            commit_hash: commit.clone(),
        },
        MessageCommand::Module { name, version } => GovernanceMessage::ModuleApproval {
            module_name: name.clone(),
            version: version.clone(),
        },
        MessageCommand::Budget { amount, purpose } => GovernanceMessage::BudgetDecision {
            amount: *amount,
            purpose: purpose.clone(),
        },
    };

    // Sign the message
    let signature = crypto_sign_message(&keypair.secret_key, &message.to_signing_bytes())?;

    // Save signature to file
    save_signature(&signature, &args.output)?;

    Ok(signature)
}

fn load_keypair(key_path: &str) -> Result<GovernanceKeypair, Box<dyn std::error::Error>> {
    if !Path::new(key_path).exists() {
        return Err(format!("Key file not found: {}", key_path).into());
    }

    let key_data = fs::read_to_string(key_path)?;
    let key_json: serde_json::Value = serde_json::from_str(&key_data)?;

    let secret_key_hex = key_json["secret_key"]
        .as_str()
        .ok_or("Invalid key file format")?;

    let secret_key_bytes = hex::decode(secret_key_hex)?;
    GovernanceKeypair::from_secret_key(&secret_key_bytes)
        .map_err(|e| format!("Invalid secret key: {}", e).into())
}

fn save_signature(
    signature: &Signature,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signature_data = serde_json::json!({
        "signature": hex::encode(signature.to_bytes()),
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    let json_str = serde_json::to_string_pretty(&signature_data)?;
    fs::write(output_path, json_str)?;

    Ok(())
}

fn format_signature_output(
    signature: &Signature,
    args: &Args,
    formatter: &OutputFormatter,
) -> String {
    if args.format == OutputFormat::Json {
        let output_data = serde_json::json!({
            "success": true,
            "signature": hex::encode(signature.to_bytes()),
            "output_file": args.output,
        });
        formatter
            .format(&output_data)
            .unwrap_or_else(|_| "{}".to_string())
    } else {
        format!(
            "Signed message successfully\nSignature: {}\nSaved to: {}\n",
            signature, args.output
        )
    }
}
