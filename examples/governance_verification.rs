//! # Governance Verification Example
//!
//! Example of verifying governance signatures.

use bllvm_sdk::governance::{GovernanceKeypair, GovernanceMessage, Multisig};
use bllvm_sdk::{sign_message, verify_signature};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Governance Verification Example ===\n");

    // Generate keypairs for 3-of-5 multisig
    println!("1. Generating keypairs for 3-of-5 multisig...");
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    for (i, keypair) in keypairs.iter().enumerate() {
        println!("   Keypair {}: {}", i + 1, keypair.public_key());
    }
    println!();

    // Create multisig
    println!("2. Creating multisig...");
    let multisig = Multisig::new(3, 5, public_keys)?;
    println!(
        "   Threshold: {}/{}",
        multisig.threshold(),
        multisig.total()
    );
    println!();

    // Create a release message
    println!("3. Creating release message...");
    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123def456".to_string(),
    };
    println!("   Message: {}", message.description());
    println!();

    // Sign with 3 keys (meets threshold)
    println!("4. Signing with 3 keys (meets threshold)...");
    let signatures: Vec<_> = keypairs[0..3]
        .iter()
        .map(|kp| {
            let sig = sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap();
            println!("   Signed with key: {}", kp.public_key());
            sig
        })
        .collect();
    println!("   Total signatures: {}", signatures.len());
    println!();

    // Verify multisig
    println!("5. Verifying multisig...");
    let verified = multisig.verify(&message.to_signing_bytes(), &signatures)?;
    println!("   Multisig verified: {}", verified);
    println!();

    // Test with insufficient signatures
    println!("6. Testing with insufficient signatures (2 keys)...");
    let insufficient_signatures: Vec<_> = keypairs[0..2]
        .iter()
        .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
        .collect();

    let insufficient_verified =
        multisig.verify(&message.to_signing_bytes(), &insufficient_signatures);
    match insufficient_verified {
        Ok(verified) => println!("   Insufficient signatures verified: {}", verified),
        Err(e) => println!("   Insufficient signatures error: {}", e),
    }
    println!();

    // Test individual signature verification
    println!("7. Testing individual signature verification...");
    for (i, signature) in signatures.iter().enumerate() {
        let verified = verify_signature(
            signature,
            &message.to_signing_bytes(),
            &keypairs[i].public_key(),
        )?;
        println!("   Signature {} verified: {}", i + 1, verified);
    }
    println!();

    // Test cross-verification (wrong key)
    println!("8. Testing cross-verification (wrong key)...");
    let wrong_key_verified = verify_signature(
        &signatures[0],
        &message.to_signing_bytes(),
        &keypairs[4].public_key(), // Different key
    )?;
    println!("   Wrong key verified: {}", wrong_key_verified);
    println!();

    println!("=== Example Complete ===");
    Ok(())
}
