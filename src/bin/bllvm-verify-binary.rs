//! # Bitcoin Commons BLLVM Binary Verifier
//!
//! Verify binary and verification bundle signatures for Bitcoin Commons releases.
//!
//! This tool verifies that binaries and verification bundles are signed by
//! authorized maintainers and match their cryptographic hashes.

use bllvm_sdk::cli::input::{parse_comma_separated, parse_threshold};
use bllvm_sdk::cli::output::{OutputFormat, OutputFormatter};
use bllvm_sdk::governance::{Multisig, PublicKey, Signature};
use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Verify binary and verification bundle signatures
#[derive(Parser, Debug)]
#[command(name = "bllvm-verify-binary")]
#[command(about = "Verify binary and verification bundle signatures for Bitcoin Commons releases")]
struct Args {
    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    format: OutputFormat,

    /// What to verify
    #[command(subcommand)]
    target: VerifyTarget,

    /// Signature files (comma-separated)
    #[arg(short, long, required = true)]
    signatures: String,

    /// Threshold (e.g., "6-of-7")
    #[arg(short, long)]
    threshold: Option<String>,

    /// Public key files (comma-separated)
    #[arg(short, long)]
    pubkeys: Option<String>,
}

#[derive(Subcommand, Debug)]
enum VerifyTarget {
    /// Verify a binary file
    Binary {
        /// Path to the binary file
        #[arg(short, long, required = true)]
        file: String,

        /// Binary type (consensus, protocol, application)
        #[arg(short, long, default_value = "application")]
        binary_type: String,

        /// Version string
        #[arg(short, long)]
        version: Option<String>,

        /// Git commit hash
        #[arg(short, long)]
        commit: Option<String>,
    },
    /// Verify a verification bundle
    Bundle {
        /// Path to the verification bundle file (.tar.gz)
        #[arg(short, long, required = true)]
        file: String,

        /// Source code hash (SHA256)
        #[arg(short, long)]
        source_hash: Option<String>,

        /// Build configuration hash (SHA256)
        #[arg(short, long)]
        build_config_hash: Option<String>,

        /// Orange Paper specification hash (SHA256)
        #[arg(short, long)]
        spec_hash: Option<String>,
    },
    /// Verify a SHA256SUMS file
    Checksums {
        /// Path to the SHA256SUMS file
        #[arg(short, long, required = true)]
        file: String,

        /// Version string
        #[arg(short, long)]
        version: Option<String>,
    },
}

fn main() {
    let args = Args::parse();
    let formatter = OutputFormatter::new(args.format.clone());

    match verify_target(&args) {
        Ok(result) => {
            let output = format_verification_output(&result, &args, &formatter);
            println!("{}", output);
            if !result.valid {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{}", formatter.format_error(&*e));
            std::process::exit(1);
        }
    }
}

#[derive(Debug)]
struct VerificationResult {
    valid: bool,
    file_path: String,
    file_hash: String,
    valid_signatures: usize,
    invalid_signatures: usize,
    threshold_met: bool,
    errors: Vec<String>,
}

fn verify_target(args: &Args) -> Result<VerificationResult, Box<dyn std::error::Error>> {
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

    // Create message to verify based on target type
    let (message_bytes, file_hash, file_path) = match &args.target {
        VerifyTarget::Binary {
            file,
            binary_type,
            version,
            commit,
        } => {
            let binary_data = fs::read(file)?;
            let mut hasher = Sha256::new();
            hasher.update(&binary_data);
            let hash = hex::encode(hasher.finalize());

            let mut message_parts =
                vec!["binary".to_string(), binary_type.to_string(), hash.clone()];
            if let Some(v) = version {
                message_parts.push(v.to_string());
            }
            if let Some(c) = commit {
                message_parts.push(c.to_string());
            }
            let message = message_parts.join(":");
            (message.into_bytes(), hash, file.clone())
        }
        VerifyTarget::Bundle {
            file,
            source_hash,
            build_config_hash,
            spec_hash,
        } => {
            let bundle_data = fs::read(file)?;
            let mut hasher = Sha256::new();
            hasher.update(&bundle_data);
            let hash = hex::encode(hasher.finalize());

            let mut message_parts = vec!["bundle".to_string(), hash.clone()];
            if let Some(sh) = source_hash {
                message_parts.push(sh.to_string());
            }
            if let Some(bch) = build_config_hash {
                message_parts.push(bch.to_string());
            }
            if let Some(sph) = spec_hash {
                message_parts.push(sph.to_string());
            }
            let message = message_parts.join(":");
            (message.into_bytes(), hash, file.clone())
        }
        VerifyTarget::Checksums { file, version } => {
            let checksums_data = fs::read_to_string(file)?;
            let mut hasher = Sha256::new();
            hasher.update(checksums_data.as_bytes());
            let hash = hex::encode(hasher.finalize());

            let mut message_parts = vec!["checksums".to_string(), hash.clone()];
            if let Some(v) = version {
                message_parts.push(v.to_string());
            }
            let message = message_parts.join(":");
            (message.into_bytes(), hash, file.clone())
        }
    };

    // Verify signatures
    let mut valid_signatures = 0;
    let mut invalid_signatures = 0;
    let mut errors = Vec::new();

    for signature in &signatures {
        let mut verified = false;
        for public_key in &public_keys {
            match bllvm_sdk::governance::verify_signature(signature, &message_bytes, public_key) {
                Ok(true) => {
                    verified = true;
                    break;
                }
                Ok(false) => continue,
                Err(e) => {
                    errors.push(format!("Verification error: {}", e));
                    continue;
                }
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
            errors.push(format!(
                "Expected {} public keys, got {}",
                total,
                public_keys.len()
            ));
            false
        } else {
            let multisig = Multisig::new(threshold, total, public_keys)?;
            match multisig.verify(&message_bytes, &signatures) {
                Ok(result) => result,
                Err(e) => {
                    errors.push(format!("Multisig verification error: {}", e));
                    false
                }
            }
        }
    } else {
        valid_signatures > 0
    };

    Ok(VerificationResult {
        valid: threshold_met && invalid_signatures == 0,
        file_path,
        file_hash,
        valid_signatures,
        invalid_signatures,
        threshold_met,
        errors,
    })
}

fn load_signatures(
    signature_files: &[String],
) -> Result<Vec<Signature>, Box<dyn std::error::Error>> {
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
            "success": result.valid,
            "file_path": result.file_path,
            "file_hash": result.file_hash,
            "valid_signatures": result.valid_signatures,
            "invalid_signatures": result.invalid_signatures,
            "threshold_met": result.threshold_met,
            "errors": result.errors,
        });
        formatter
            .format(&output_data)
            .unwrap_or_else(|_| "{}".to_string())
    } else {
        let mut output = "Verification Results\n".to_string();
        output.push_str(&format!("File: {}\n", result.file_path));
        output.push_str(&format!("Hash: {}\n", result.file_hash));
        output.push_str(&format!("Valid signatures: {}\n", result.valid_signatures));
        output.push_str(&format!(
            "Invalid signatures: {}\n",
            result.invalid_signatures
        ));
        output.push_str(&format!("Threshold met: {}\n", result.threshold_met));
        if !result.errors.is_empty() {
            output.push_str("\nErrors:\n");
            for error in &result.errors {
                output.push_str(&format!("  - {}\n", error));
            }
        }
        if result.valid {
            output.push_str("\n✅ Verification PASSED\n");
        } else {
            output.push_str("\n❌ Verification FAILED\n");
        }
        output
    }
}
