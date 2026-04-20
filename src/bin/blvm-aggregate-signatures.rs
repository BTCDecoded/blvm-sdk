//! # Bitcoin Commons BLLVM Signature Aggregator
//!
//! Aggregate multiple signatures into a single multisig signature file.
//!
//! This tool collects signatures from multiple maintainers and creates a
//! single signature file that can be verified against a multisig threshold.

use blvm_sdk::cli::input::parse_comma_separated;
use blvm_sdk::cli::output::{OutputFormat, OutputFormatter};
use clap::Parser;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Aggregate multiple signatures into a single file
#[derive(Parser, Debug)]
#[command(name = "blvm-aggregate-signatures")]
#[command(about = "Aggregate multiple signatures into a single multisig signature file")]
struct Args {
    /// Output file for aggregated signatures
    #[arg(short, long, default_value = "signatures.json")]
    output: String,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    format: OutputFormat,

    /// Signature files to aggregate (comma-separated)
    #[arg(short, long, required = true)]
    signatures: String,

    /// Threshold (e.g., "6-of-7")
    #[arg(short, long)]
    threshold: Option<String>,

    /// Public key files (comma-separated, for verification)
    #[arg(short, long)]
    pubkeys: Option<String>,
}

fn main() {
    let args = Args::parse();
    let formatter = OutputFormatter::new(args.format.clone());

    match aggregate_signatures(&args) {
        Ok(result) => {
            let output = format_aggregation_output(&result, &args, &formatter);
            println!("{}", output);
        }
        Err(e) => {
            eprintln!("{}", formatter.format_error(&*e));
            std::process::exit(1);
        }
    }
}

#[derive(Debug)]
struct AggregationResult {
    signature_count: usize,
    output_file: String,
    threshold_met: bool,
}

fn aggregate_signatures(args: &Args) -> Result<AggregationResult, Box<dyn std::error::Error>> {
    // Parse signature files
    let signature_files = parse_comma_separated(&args.signatures);
    let mut signatures = Vec::new();
    let mut metadata = None;

    for file_path in &signature_files {
        if !Path::new(file_path).exists() {
            return Err(format!("Signature file not found: {}", file_path).into());
        }

        let sig_data = fs::read_to_string(file_path)?;
        let sig_json: Value = serde_json::from_str(&sig_data)?;

        // Extract signature
        let signature_entry = serde_json::json!({
            "signature": sig_json.get("signature"),
            "signer": sig_json.get("signer").or_else(|| sig_json.get("metadata").and_then(|m| m.get("signer"))),
            "signed_at": sig_json.get("created_at").or_else(|| sig_json.get("metadata").and_then(|m| m.get("signed_at"))),
            "public_key": sig_json.get("public_key"),
        });

        signatures.push(signature_entry);

        // Use first signature's metadata as base
        if metadata.is_none() {
            metadata = sig_json.get("metadata").cloned();
        }
    }

    // Create aggregated signature file
    let aggregated = serde_json::json!({
        "version": "1.0",
        "signature_count": signatures.len(),
        "signatures": signatures,
        "threshold": args.threshold,
        "metadata": metadata,
        "aggregated_at": chrono::Utc::now().to_rfc3339(),
    });

    // Save aggregated signatures
    let json_str = serde_json::to_string_pretty(&aggregated)?;
    fs::write(&args.output, json_str)?;

    // Check threshold if provided
    let threshold_met = if let Some(threshold_str) = &args.threshold {
        let parts: Vec<&str> = threshold_str.split("-of-").collect();
        if parts.len() == 2 {
            if let (Ok(required), Ok(_total)) =
                (parts[0].parse::<usize>(), parts[1].parse::<usize>())
            {
                signatures.len() >= required
            } else {
                false
            }
        } else {
            false
        }
    } else {
        true // No threshold specified, assume met if we have signatures
    };

    Ok(AggregationResult {
        signature_count: signatures.len(),
        output_file: args.output.clone(),
        threshold_met,
    })
}

fn format_aggregation_output(
    result: &AggregationResult,
    args: &Args,
    formatter: &OutputFormatter,
) -> String {
    if args.format == OutputFormat::Json {
        let output_data = serde_json::json!({
            "success": true,
            "signature_count": result.signature_count,
            "threshold_met": result.threshold_met,
            "output_file": result.output_file,
        });
        formatter
            .format(&output_data)
            .unwrap_or_else(|_| "{}".to_string())
    } else {
        format!(
            "Aggregated {} signatures\n\
             Threshold met: {}\n\
             Saved to: {}\n",
            result.signature_count,
            if result.threshold_met { "Yes" } else { "No" },
            result.output_file
        )
    }
}
