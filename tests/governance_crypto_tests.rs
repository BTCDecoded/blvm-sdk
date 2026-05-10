//! # Governance Crypto Integration Tests
//!
//! Comprehensive integration tests for governance crypto operations.

use blvm_sdk::governance::{GovernanceKeypair, GovernanceMessage, Multisig};
use blvm_sdk::sign_message;

#[test]
fn test_complete_governance_workflow() {
    // Generate keypairs for a 3-of-5 multisig
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    // Create multisig
    let multisig = Multisig::new(3, 5, public_keys).unwrap();

    // Create a release message
    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    // Sign with 3 keys (meets threshold)
    let signatures: Vec<_> = keypairs[0..3]
        .iter()
        .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
        .collect();

    // Verify multisig
    let verified = multisig
        .verify(&message.to_signing_bytes(), &signatures)
        .unwrap();
    assert!(verified);
}

#[test]
fn test_insufficient_signatures() {
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    let multisig = Multisig::new(3, 5, public_keys).unwrap();
    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    // Sign with only 2 keys (below threshold)
    let signatures: Vec<_> = keypairs[0..2]
        .iter()
        .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
        .collect();

    let result = multisig.verify(&message.to_signing_bytes(), &signatures);
    assert!(result.is_err());
}

#[test]
fn test_different_message_types() {
    let keypair = GovernanceKeypair::generate().unwrap();

    // Test release message
    let release_msg = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };
    let release_sig = sign_message(&keypair.secret_key, &release_msg.to_signing_bytes()).unwrap();
    assert!(blvm_sdk::governance::verify_signature(
        &release_sig,
        &release_msg.to_signing_bytes(),
        &keypair.public_key(),
    )
    .unwrap());

    // Test module approval message
    let module_msg = GovernanceMessage::ModuleApproval {
        module_name: "lightning".to_string(),
        version: "v2.0.0".to_string(),
    };
    let module_sig = sign_message(&keypair.secret_key, &module_msg.to_signing_bytes()).unwrap();
    assert!(blvm_sdk::governance::verify_signature(
        &module_sig,
        &module_msg.to_signing_bytes(),
        &keypair.public_key(),
    )
    .unwrap());

    // Test budget decision message
    let budget_msg = GovernanceMessage::BudgetDecision {
        amount: 1000000,
        purpose: "development".to_string(),
    };
    let budget_sig = sign_message(&keypair.secret_key, &budget_msg.to_signing_bytes()).unwrap();
    assert!(blvm_sdk::governance::verify_signature(
        &budget_sig,
        &budget_msg.to_signing_bytes(),
        &keypair.public_key(),
    )
    .unwrap());
}

#[test]
fn test_signature_cross_verification() {
    let keypair1 = GovernanceKeypair::generate().unwrap();
    let keypair2 = GovernanceKeypair::generate().unwrap();

    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    // Sign with keypair1
    let signature = sign_message(&keypair1.secret_key, &message.to_signing_bytes()).unwrap();

    // Verify with keypair1 (should succeed)
    assert!(blvm_sdk::governance::verify_signature(
        &signature,
        &message.to_signing_bytes(),
        &keypair1.public_key(),
    )
    .unwrap());

    // Verify with keypair2 (should fail)
    assert!(!blvm_sdk::governance::verify_signature(
        &signature,
        &message.to_signing_bytes(),
        &keypair2.public_key(),
    )
    .unwrap());
}

#[test]
fn test_multisig_edge_cases() {
    let keypairs: Vec<_> = (0..7)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    // Test 6-of-7 multisig
    let multisig = Multisig::new(6, 7, public_keys).unwrap();
    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    // Sign with exactly 6 keys (meets threshold)
    let signatures: Vec<_> = keypairs[0..6]
        .iter()
        .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
        .collect();

    let verified = multisig
        .verify(&message.to_signing_bytes(), &signatures)
        .unwrap();
    assert!(verified);

    // Sign with 7 keys (above threshold, should still work)
    let signatures: Vec<_> = keypairs
        .iter()
        .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
        .collect();

    let verified = multisig
        .verify(&message.to_signing_bytes(), &signatures)
        .unwrap();
    assert!(verified);
}
