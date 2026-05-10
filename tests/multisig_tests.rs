//! # Multisig Tests
//!
//! Tests for multisig threshold validation and signature collection.

use blvm_sdk::governance::{GovernanceKeypair, GovernanceMessage, Multisig};
use blvm_sdk::sign_message;

#[test]
fn test_multisig_creation() {
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    let multisig = Multisig::new(3, 5, public_keys).unwrap();
    assert_eq!(multisig.threshold(), 3);
    assert_eq!(multisig.total(), 5);
}

#[test]
fn test_multisig_invalid_threshold() {
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    // Threshold too high
    let result = Multisig::new(6, 5, public_keys.clone());
    assert!(result.is_err());

    // Threshold zero
    let result = Multisig::new(0, 5, public_keys);
    assert!(result.is_err());
}

#[test]
fn test_multisig_wrong_key_count() {
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    // Wrong number of keys
    let result = Multisig::new(3, 5, public_keys[0..3].to_vec());
    assert!(result.is_err());
}

#[test]
fn test_multisig_duplicate_keys() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let public_key = keypair.public_key();

    // Create multisig with duplicate keys
    let public_keys = vec![public_key.clone(), public_key];

    let result = Multisig::new(2, 2, public_keys);
    assert!(result.is_err());
}

#[test]
fn test_multisig_verification() {
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    let multisig = Multisig::new(3, 5, public_keys).unwrap();
    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    // Sign with 3 keys (meets threshold)
    let signatures: Vec<_> = keypairs[0..3]
        .iter()
        .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
        .collect();

    let verified = multisig
        .verify(&message.to_signing_bytes(), &signatures)
        .unwrap();
    assert!(verified);
}

#[test]
fn test_multisig_insufficient_signatures() {
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
fn test_multisig_excess_signatures() {
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    let multisig = Multisig::new(3, 5, public_keys).unwrap();
    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    // Sign with all 5 keys (above threshold, should still work)
    let signatures: Vec<_> = keypairs
        .iter()
        .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
        .collect();

    let verified = multisig
        .verify(&message.to_signing_bytes(), &signatures)
        .unwrap();
    assert!(verified);
}

#[test]
fn test_multisig_collect_valid_signatures() {
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    let multisig = Multisig::new(3, 5, public_keys).unwrap();
    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    // Sign with 3 keys
    let signatures: Vec<_> = keypairs[0..3]
        .iter()
        .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
        .collect();

    let valid_indices = multisig
        .collect_valid_signatures(&message.to_signing_bytes(), &signatures)
        .unwrap();

    assert_eq!(valid_indices.len(), 3);
    assert_eq!(valid_indices, vec![0, 1, 2]);
}

#[test]
fn test_multisig_mixed_valid_invalid_signatures() {
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    let multisig = Multisig::new(3, 5, public_keys).unwrap();
    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    // Create a mix of valid and invalid signatures
    let mut signatures = Vec::new();

    // Add 2 valid signatures
    for kp in &keypairs[0..2] {
        let sig = sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap();
        signatures.push(sig);
    }

    // Add 1 invalid signature (wrong message)
    let wrong_message = GovernanceMessage::Release {
        version: "v2.0.0".to_string(),
        commit_hash: "def456".to_string(),
    };
    let invalid_sig =
        sign_message(&keypairs[2].secret_key, &wrong_message.to_signing_bytes()).unwrap();
    signatures.push(invalid_sig);

    // Add 1 more valid signature
    let valid_sig = sign_message(&keypairs[3].secret_key, &message.to_signing_bytes()).unwrap();
    signatures.push(valid_sig);

    let valid_indices = multisig
        .collect_valid_signatures(&message.to_signing_bytes(), &signatures)
        .unwrap();

    // Should have 3 valid signatures (meets threshold)
    assert_eq!(valid_indices.len(), 3);
    assert_eq!(valid_indices, vec![0, 1, 3]);
}

#[test]
fn test_multisig_edge_case_thresholds() {
    let keypairs: Vec<_> = (0..7)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    // Test various threshold configurations
    let thresholds = vec![(1, 1), (2, 3), (3, 5), (6, 7)];

    for (threshold, total) in thresholds {
        let multisig = Multisig::new(threshold, total, public_keys[0..total].to_vec()).unwrap();
        let message = GovernanceMessage::Release {
            version: "v1.0.0".to_string(),
            commit_hash: "abc123".to_string(),
        };

        // Sign with exactly the threshold number of keys
        let signatures: Vec<_> = keypairs[0..threshold]
            .iter()
            .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
            .collect();

        let verified = multisig
            .verify(&message.to_signing_bytes(), &signatures)
            .unwrap();
        assert!(verified);
    }
}
