//! # Bitcoin Commons BLLVM Verifier
//!
//! Verify governance signatures and multisig thresholds.

use clap::{Parser, Subcommand};
use developer_sdk::governance::{GovernanceMessage, Multisig, Signature, PublicKey};
use developer_sdk::cli::output::{OutputFormat, OutputFormatter};
use developer_sdk::cli::input::{parse_comma_separated, parse_threshold};
use std::fs;
use std::path::Path;

/// Verify governance signatures
#[derive(Parser, Debug)]
#[command(name = "bllvm-verify")]
#[command(about = "Verify governance signatures and multisig thresholds")]
struct Args {
    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    format: OutputFormat,

    /// Message to verify
    #[command(subcommand)]
    message: MessageCommand,

    /// Signature files (comma-separated)
    #[arg(short, long, required = true)]
    signatures: String,

    /// Threshold (e.g., "3-of-5")
    #[arg(short, long)]
    threshold: Option<String>,

    /// Public key files (comma-separated)
    #[arg(short, long)]
    pubkeys: Option<String>,
}

#[derive(Subcommand, Debug)]
enum MessageCommand {
    /// Verify a release message
    Release {
        /// Version string
        #[arg(short, long, required = true)]
        version: String,

        /// Commit hash
        #[arg(short, long, required = true)]
        commit: String,
    },
    /// Verify a module approval message
    Module {
        /// Module name
        #[arg(short, long, required = true)]
        name: String,

        /// Module version
        #[arg(short, long, required = true)]
        version: String,
    },
    /// Verify a budget decision message
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

    match verify_message(&args) {
        Ok(result) => {
            let output = format_verification_output(&result, &args, &formatter);
            println!("{}", output);
        }
        Err(e) => {
            eprintln!("{}", formatter.format_error(&*e));
            std::process::exit(1);
        }
    }
}

fn verify_message(args: &Args) -> Result<VerificationResult, Box<dyn std::error::Error>> {
    // Create the message
    let message = match &args.message {
        MessageCommand::Release { version, commit } => {
            GovernanceMessage::Release {
                version: version.clone(),
                commit_hash: commit.clone(),
            }
        }
        MessageCommand::Module { name, version } => {
            GovernanceMessage::ModuleApproval {
                module_name: name.clone(),
                version: version.clone(),
            }
        }
        MessageCommand::Budget { amount, purpose } => {
            GovernanceMessage::BudgetDecision {
                amount: *amount,
                purpose: purpose.clone(),
            }
        }
    };

    // Load signatures
    let signature_files = parse_comma_separated(&args.signatures);
    let signatures = load_signatures(&signature_files)?;

    // Load public keys if provided
    let public_keys = if let Some(pubkey_files) = &args.pubkeys {
        let pubkey_files = parse_comma_separated(pubkey_files);
        load_public_keys(&pubkey_files)?
    } else {
        Vec::new()
    };

    // Verify signatures
    let message_bytes = message.to_signing_bytes();
    let mut valid_signatures = 0;
    let mut invalid_signatures = 0;

    for signature in &signatures {
        let mut verified = false;
        for public_key in &public_keys {
            if developer_sdk::governance::verify_signature(signature, &message_bytes, public_key)? {
                verified = true;
                break;
            }
        }
        if verified {
            valid_signatures += 1;
        } else {
            invalid_signatures += 1;
        }
    }

    // Check multisig threshold if provided
    let threshold_met = if let Some(threshold_str) = &args.threshold {
        let (threshold, total) = parse_threshold(threshold_str)?;
        if public_keys.len() != total {
            return Err(format!("Expected {} public keys, got {}", total, public_keys.len()).into());
        }
        
        let multisig = Multisig::new(threshold, total, public_keys)?;
        multisig.verify(&message_bytes, &signatures)?
    } else {
        valid_signatures > 0
    };

    Ok(VerificationResult {
        message,
        valid_signatures,
        invalid_signatures,
        threshold_met,
    })
}

#[derive(Debug)]
struct VerificationResult {
    message: GovernanceMessage,
    valid_signatures: usize,
    invalid_signatures: usize,
    threshold_met: bool,
}

fn load_signatures(signature_files: &[String]) -> Result<Vec<Signature>, Box<dyn std::error::Error>> {
    let mut signatures = Vec::new();
    
    for file_path in signature_files {
        if !Path::new(file_path).exists() {
            return Err(format!("Signature file not found: {}", file_path).into());
        }

        let sig_data = fs::read_to_string(file_path)?;
        let sig_json: serde_json::Value = serde_json::from_str(&sig_data)?;

        let signature_hex = sig_json["signature"]
            .as_str()
            .ok_or("Invalid signature file format")?;

        let signature_bytes = hex::decode(signature_hex)?;
        let signature = Signature::from_bytes(&signature_bytes)?;
        signatures.push(signature);
    }

    Ok(signatures)
}

fn load_public_keys(pubkey_files: &[String]) -> Result<Vec<PublicKey>, Box<dyn std::error::Error>> {
    let mut public_keys = Vec::new();
    
    for file_path in pubkey_files {
        if !Path::new(file_path).exists() {
            return Err(format!("Public key file not found: {}", file_path).into());
        }

        let key_data = fs::read_to_string(file_path)?;
        let key_json: serde_json::Value = serde_json::from_str(&key_data)?;

        let pubkey_hex = key_json["public_key"]
            .as_str()
            .ok_or("Invalid public key file format")?;

        let pubkey_bytes = hex::decode(pubkey_hex)?;
        let public_key = PublicKey::from_bytes(&pubkey_bytes)?;
        public_keys.push(public_key);
    }

    Ok(public_keys)
}

fn format_verification_output(
    result: &VerificationResult,
    args: &Args,
    formatter: &OutputFormatter,
) -> String {
    if args.format == OutputFormat::Json {
        let output_data = serde_json::json!({
            "success": true,
            "message": result.message.description(),
            "valid_signatures": result.valid_signatures,
            "invalid_signatures": result.invalid_signatures,
            "threshold_met": result.threshold_met,
        });
        formatter.format(&output_data).unwrap_or_else(|_| "{}".to_string())
    } else {
        let mut output = "Verification Results\n".to_string();
        output.push_str(&format!("Message: {}\n", result.message.description()));
        output.push_str(&format!("Valid signatures: {}\n", result.valid_signatures));
        output.push_str(&format!("Invalid signatures: {}\n", result.invalid_signatures));
        output.push_str(&format!("Threshold met: {}\n", result.threshold_met));
        output
    }
}
