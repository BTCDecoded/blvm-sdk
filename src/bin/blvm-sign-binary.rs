//! # Bitcoin Commons BLLVM Binary Signer
//!
//! Sign binaries and verification bundles for Bitcoin Commons releases.
//!
//! This tool signs binaries and verification bundles with maintainer multisig,
//! creating cryptographic proof that binaries match verified code.

use blvm_sdk::cli::output::{OutputFormat, OutputFormatter};
use blvm_sdk::governance::{GovernanceKeypair, Signature};
use blvm_sdk::sign_message as crypto_sign_message;
use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Sign binaries and verification bundles
#[derive(Parser, Debug)]
#[command(name = "blvm-sign-binary")]
#[command(about = "Sign binaries and verification bundles for Bitcoin Commons releases")]
struct Args {
    /// Output file for the signature
    #[arg(short, long, default_value = "signature.json")]
    output: String,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    format: OutputFormat,

    /// Private key file
    #[arg(short, long, required = true)]
    key: String,

    /// What to sign
    #[command(subcommand)]
    target: SignTarget,
}

#[derive(Subcommand, Debug)]
enum SignTarget {
    /// Sign a binary file
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
    /// Sign a verification bundle
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
    /// Sign a SHA256SUMS file
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

    match sign_target(&args) {
        Ok(result) => {
            let output = format_signature_output(&result, &args, &formatter);
            println!("{output}");
        }
        Err(e) => {
            eprintln!("{}", formatter.format_error(&*e));
            std::process::exit(1);
        }
    }
}

#[derive(Debug)]
struct SignResult {
    signature: Signature,
    file_hash: String,
    file_path: String,
    metadata: serde_json::Value,
}

fn sign_target(args: &Args) -> Result<SignResult, Box<dyn std::error::Error>> {
    // Load the keypair
    let keypair = load_keypair(&args.key)?;

    match &args.target {
        SignTarget::Binary {
            file,
            binary_type,
            version,
            commit,
        } => sign_binary(
            &keypair,
            file,
            binary_type,
            version.as_deref(),
            commit.as_deref(),
        ),
        SignTarget::Bundle {
            file,
            source_hash,
            build_config_hash,
            spec_hash,
        } => sign_bundle(
            &keypair,
            file,
            source_hash.as_deref(),
            build_config_hash.as_deref(),
            spec_hash.as_deref(),
        ),
        SignTarget::Checksums { file, version } => {
            sign_checksums(&keypair, file, version.as_deref())
        }
    }
    .and_then(|result| {
        // Save signature to file
        save_signature(&result, &args.output)?;
        Ok(result)
    })
}

fn sign_binary(
    keypair: &GovernanceKeypair,
    file_path: &str,
    binary_type: &str,
    version: Option<&str>,
    commit: Option<&str>,
) -> Result<SignResult, Box<dyn std::error::Error>> {
    if !Path::new(file_path).exists() {
        return Err(format!("Binary file not found: {file_path}").into());
    }

    // Read binary file
    let binary_data = fs::read(file_path)?;

    // Compute SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(&binary_data);
    let file_hash = hex::encode(hasher.finalize());

    // Create message to sign: binary_type:file_hash:version:commit
    let mut message_parts = vec![
        "binary".to_string(),
        binary_type.to_string(),
        file_hash.clone(),
    ];
    if let Some(v) = version {
        message_parts.push(v.to_string());
    }
    if let Some(c) = commit {
        message_parts.push(c.to_string());
    }
    let message = message_parts.join(":");

    // Sign the message
    let signature = crypto_sign_message(&keypair.secret_key, message.as_bytes())?;

    // Create metadata
    let metadata = serde_json::json!({
        "type": "binary",
        "binary_type": binary_type,
        "file_path": file_path,
        "file_hash": file_hash,
        "version": version,
        "commit": commit,
        "signed_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(SignResult {
        signature,
        file_hash,
        file_path: file_path.to_string(),
        metadata,
    })
}

fn sign_bundle(
    keypair: &GovernanceKeypair,
    file_path: &str,
    source_hash: Option<&str>,
    build_config_hash: Option<&str>,
    spec_hash: Option<&str>,
) -> Result<SignResult, Box<dyn std::error::Error>> {
    if !Path::new(file_path).exists() {
        return Err(format!("Bundle file not found: {file_path}").into());
    }

    // Read bundle file
    let bundle_data = fs::read(file_path)?;

    // Compute SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(&bundle_data);
    let file_hash = hex::encode(hasher.finalize());

    // Create message to sign: bundle:file_hash:source_hash:build_config_hash:spec_hash
    let mut message_parts = vec!["bundle".to_string(), file_hash.clone()];
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

    // Sign the message
    let signature = crypto_sign_message(&keypair.secret_key, message.as_bytes())?;

    // Create metadata
    let metadata = serde_json::json!({
        "type": "bundle",
        "file_path": file_path,
        "file_hash": file_hash,
        "source_hash": source_hash,
        "build_config_hash": build_config_hash,
        "spec_hash": spec_hash,
        "signed_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(SignResult {
        signature,
        file_hash,
        file_path: file_path.to_string(),
        metadata,
    })
}

fn sign_checksums(
    keypair: &GovernanceKeypair,
    file_path: &str,
    version: Option<&str>,
) -> Result<SignResult, Box<dyn std::error::Error>> {
    if !Path::new(file_path).exists() {
        return Err(format!("Checksums file not found: {file_path}").into());
    }

    // Read checksums file
    let checksums_data = fs::read_to_string(file_path)?;

    // Compute SHA256 hash of file contents
    let mut hasher = Sha256::new();
    hasher.update(checksums_data.as_bytes());
    let file_hash = hex::encode(hasher.finalize());

    // Create message to sign: checksums:file_hash:version
    let mut message_parts = vec!["checksums".to_string(), file_hash.clone()];
    if let Some(v) = version {
        message_parts.push(v.to_string());
    }
    let message = message_parts.join(":");

    // Sign the message
    let signature = crypto_sign_message(&keypair.secret_key, message.as_bytes())?;

    // Create metadata
    let metadata = serde_json::json!({
        "type": "checksums",
        "file_path": file_path,
        "file_hash": file_hash,
        "version": version,
        "signed_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(SignResult {
        signature,
        file_hash,
        file_path: file_path.to_string(),
        metadata,
    })
}

fn load_keypair(key_path: &str) -> Result<GovernanceKeypair, Box<dyn std::error::Error>> {
    if !Path::new(key_path).exists() {
        return Err(format!("Key file not found: {key_path}").into());
    }

    let key_data = fs::read_to_string(key_path)?;
    let key_json: serde_json::Value = serde_json::from_str(&key_data)?;

    let secret_key_hex = key_json["secret_key"]
        .as_str()
        .ok_or("Invalid key file format")?;

    let secret_key_bytes = hex::decode(secret_key_hex)?;
    GovernanceKeypair::from_secret_key(&secret_key_bytes)
        .map_err(|e| format!("Invalid secret key: {e}").into())
}

fn save_signature(
    result: &SignResult,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signature_data = serde_json::json!({
        "signature": hex::encode(result.signature.to_bytes()),
        "signer": hex::encode(result.metadata.get("signer").and_then(|s| s.as_str()).unwrap_or("unknown")),
        "file_path": result.file_path,
        "file_hash": result.file_hash,
        "metadata": result.metadata,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    let json_str = serde_json::to_string_pretty(&signature_data)?;
    fs::write(output_path, json_str)?;

    Ok(())
}

fn format_signature_output(
    result: &SignResult,
    args: &Args,
    formatter: &OutputFormatter,
) -> String {
    if args.format == OutputFormat::Json {
        let output_data = serde_json::json!({
            "success": true,
            "signature": hex::encode(result.signature.to_bytes()),
            "file_path": result.file_path,
            "file_hash": result.file_hash,
            "output_file": args.output,
            "metadata": result.metadata,
        });
        formatter
            .format(&output_data)
            .unwrap_or_else(|_| "{}".to_string())
    } else {
        format!(
            "Signed {} successfully\n\
             File: {}\n\
             Hash: {}\n\
             Signature: {}\n\
             Saved to: {}\n",
            result
                .metadata
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("file"),
            result.file_path,
            result.file_hash,
            result.signature,
            args.output
        )
    }
}
