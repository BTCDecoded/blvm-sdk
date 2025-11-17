//! # Governance Signing Example
//!
//! Example of signing governance messages.

use bllvm_sdk::governance::verify_signature;
use bllvm_sdk::{sign_message, GovernanceKeypair, GovernanceMessage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Governance Signing Example ===\n");

    // Generate a keypair
    println!("1. Generating governance keypair...");
    let keypair = GovernanceKeypair::generate()?;
    println!("   Public key: {}", keypair.public_key());
    println!(
        "   Secret key: {}...",
        hex::encode(&keypair.secret_key_bytes()[..8])
    );
    println!();

    // Create a release message
    println!("2. Creating release message...");
    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123def456".to_string(),
    };
    println!("   Message: {}", message.description());
    println!(
        "   Signing bytes: {}",
        String::from_utf8_lossy(&message.to_signing_bytes())
    );
    println!();

    // Sign the message
    println!("3. Signing message...");
    let signature = sign_message(&keypair.secret_key, &message.to_signing_bytes())?;
    println!("   Signature: {}", signature);
    println!();

    // Verify the signature
    println!("4. Verifying signature...");
    let verified = verify_signature(
        &signature,
        &message.to_signing_bytes(),
        &keypair.public_key(),
    )?;
    println!("   Verified: {}", verified);
    println!();

    // Test with different message
    println!("5. Testing with different message...");
    let different_message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "def456ghi789".to_string(),
    };
    let verified_different = verify_signature(
        &signature,
        &different_message.to_signing_bytes(),
        &keypair.public_key(),
    )?;
    println!("   Different message verified: {}", verified_different);
    println!();

    // Test with different key
    println!("6. Testing with different key...");
    let different_keypair = GovernanceKeypair::generate()?;
    let verified_different_key = verify_signature(
        &signature,
        &message.to_signing_bytes(),
        &different_keypair.public_key(),
    )?;
    println!("   Different key verified: {}", verified_different_key);
    println!();

    println!("=== Example Complete ===");
    Ok(())
}
