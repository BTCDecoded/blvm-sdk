//! # Multisig Workflow Example
//!
//! Complete multisig workflow example.

use bllvm_sdk::governance::{GovernanceKeypair, GovernanceMessage, Multisig};
use bllvm_sdk::{sign_message, verify_signature};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Multisig Workflow Example ===\n");

    // Generate keypairs for 6-of-7 multisig
    println!("1. Generating keypairs for 6-of-7 multisig...");
    let keypairs: Vec<_> = (0..7)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    for (i, keypair) in keypairs.iter().enumerate() {
        println!("   Keypair {}: {}", i + 1, keypair.public_key());
    }
    println!();

    // Create multisig
    println!("2. Creating multisig...");
    let multisig = Multisig::new(6, 7, public_keys)?;
    println!(
        "   Threshold: {}/{}",
        multisig.threshold(),
        multisig.total()
    );
    println!();

    // Create different types of messages
    println!("3. Creating governance messages...");
    let messages = vec![
        GovernanceMessage::Release {
            version: "v1.0.0".to_string(),
            commit_hash: "abc123def456".to_string(),
        },
        GovernanceMessage::ModuleApproval {
            module_name: "lightning-network".to_string(),
            version: "v2.0.0".to_string(),
        },
        GovernanceMessage::BudgetDecision {
            amount: 1000000,
            purpose: "development and maintenance".to_string(),
        },
    ];

    for (i, message) in messages.iter().enumerate() {
        println!("   Message {}: {}", i + 1, message.description());
    }
    println!();

    // Sign each message with 6 keys
    println!("4. Signing messages with 6 keys...");
    for (msg_idx, message) in messages.iter().enumerate() {
        println!("   Signing message {}...", msg_idx + 1);
        let signatures: Vec<_> = keypairs[0..6]
            .iter()
            .map(|kp| {
                let sig = sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap();
                println!("     Signed with key: {}", kp.public_key());
                sig
            })
            .collect();

        // Verify multisig
        let verified = multisig.verify(&message.to_signing_bytes(), &signatures)?;
        println!("     Multisig verified: {}", verified);
        println!();
    }

    // Test with mixed valid/invalid signatures
    println!("5. Testing with mixed valid/invalid signatures...");
    let message = &messages[0]; // Use release message
    let mut signatures = Vec::new();

    // Add 4 valid signatures
    for kp in &keypairs[0..4] {
        let sig = sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap();
        signatures.push(sig);
        println!("   Added valid signature from: {}", kp.public_key());
    }

    // Add 2 invalid signatures (wrong message)
    let wrong_message = GovernanceMessage::Release {
        version: "v2.0.0".to_string(),
        commit_hash: "def456ghi789".to_string(),
    };
    for kp in &keypairs[4..6] {
        let sig = sign_message(&kp.secret_key, &wrong_message.to_signing_bytes()).unwrap();
        signatures.push(sig);
        println!("   Added invalid signature from: {}", kp.public_key());
    }

    // Add 1 more valid signature
    let sig = sign_message(&keypairs[6].secret_key, &message.to_signing_bytes()).unwrap();
    signatures.push(sig);
    println!(
        "   Added valid signature from: {}",
        keypairs[6].public_key()
    );
    println!();

    // Verify multisig
    let verified = multisig.verify(&message.to_signing_bytes(), &signatures)?;
    println!("   Mixed signatures verified: {}", verified);
    println!();

    // Test signature collection
    println!("6. Testing signature collection...");
    let valid_indices =
        multisig.collect_valid_signatures(&message.to_signing_bytes(), &signatures)?;
    println!("   Valid signature indices: {:?}", valid_indices);
    println!("   Valid signatures count: {}", valid_indices.len());
    println!();

    // Test individual signature validation
    println!("7. Testing individual signature validation...");
    for (i, signature) in signatures.iter().enumerate() {
        let mut verified = false;
        for (j, public_key) in multisig.public_keys().iter().enumerate() {
            if verify_signature(signature, &message.to_signing_bytes(), public_key).unwrap() {
                verified = true;
                println!("   Signature {} verified with key {}", i + 1, j + 1);
                break;
            }
        }
        if !verified {
            println!("   Signature {} not verified", i + 1);
        }
    }
    println!();

    // Test edge cases
    println!("8. Testing edge cases...");

    // Test with exactly threshold number of signatures
    let exact_signatures: Vec<_> = keypairs[0..6]
        .iter()
        .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
        .collect();
    let exact_verified = multisig.verify(&message.to_signing_bytes(), &exact_signatures)?;
    println!(
        "   Exactly threshold signatures verified: {}",
        exact_verified
    );

    // Test with all signatures
    let all_signatures: Vec<_> = keypairs
        .iter()
        .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
        .collect();
    let all_verified = multisig.verify(&message.to_signing_bytes(), &all_signatures)?;
    println!("   All signatures verified: {}", all_verified);
    println!();

    println!("=== Example Complete ===");
    Ok(())
}
