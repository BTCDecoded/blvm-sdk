//! Additional tests for multisig module to improve coverage.

use blvm_sdk::governance::{GovernanceError, GovernanceKeypair, Multisig};
use blvm_sdk::sign_message;

#[test]
fn test_multisig_debug_format() {
    let keypairs: Vec<_> = (0..3)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();
    let multisig = Multisig::new(2, 3, public_keys).unwrap();

    let debug_str = format!("{multisig:?}");
    assert!(debug_str.contains("Multisig"));
    assert!(debug_str.contains("threshold"));
    assert!(debug_str.contains("total"));
    assert!(debug_str.contains("public_keys"));
}

#[test]
fn test_multisig_clone() {
    let keypairs: Vec<_> = (0..3)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    let multisig1 = Multisig::new(2, 3, public_keys).unwrap();
    let multisig2 = multisig1.clone();

    // Test that clone works and properties are preserved
    assert_eq!(multisig1.threshold(), multisig2.threshold());
    assert_eq!(multisig1.total(), multisig2.total());
    assert_eq!(multisig1.public_keys().len(), multisig2.public_keys().len());
}

#[test]
fn test_multisig_properties() {
    let keypairs: Vec<_> = (0..3)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    let multisig = Multisig::new(2, 3, public_keys).unwrap();

    // Test that properties are correctly set
    assert_eq!(multisig.threshold(), 2);
    assert_eq!(multisig.total(), 3);
    assert_eq!(multisig.public_keys().len(), 3);
}

#[test]
fn test_multisig_new_edge_cases() {
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    // Test with threshold = 0 (should be invalid)
    let result = Multisig::new(0, 5, public_keys.clone());
    assert!(result.is_err());
    if let Err(GovernanceError::InvalidThreshold { threshold, total }) = result {
        assert_eq!(threshold, 0);
        assert_eq!(total, 5);
    }

    // Test with threshold > total (should be invalid)
    let result = Multisig::new(6, 5, public_keys.clone());
    assert!(result.is_err());
    if let Err(GovernanceError::InvalidThreshold { threshold, total }) = result {
        assert_eq!(threshold, 6);
        assert_eq!(total, 5);
    }

    // Test with empty public keys (should be invalid)
    let result = Multisig::new(1, 0, vec![]);
    assert!(result.is_err());
    if let Err(GovernanceError::InvalidThreshold { threshold, total }) = result {
        assert_eq!(threshold, 1);
        assert_eq!(total, 0);
    }

    // Test with threshold = total (should be valid)
    let result = Multisig::new(5, 5, public_keys.clone());
    assert!(result.is_ok());

    // Test with threshold = 1 (should be valid)
    let result = Multisig::new(1, 5, public_keys.clone());
    assert!(result.is_ok());
}

#[test]
fn test_multisig_verify_edge_cases() {
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();
    let multisig = Multisig::new(3, 5, public_keys).unwrap();

    let message = b"test message";

    // Test with empty signatures
    let result = multisig.verify(message, &[]);
    assert!(result.is_err());
    if let Err(GovernanceError::InsufficientSignatures { got, need }) = result {
        assert_eq!(got, 0);
        assert_eq!(need, 3);
    }

    // Test with too many signatures (should still work, just ignore extras)
    let signatures: Vec<_> = (0..10)
        .map(|i| {
            let keypair = &keypairs[i % 5];
            sign_message(&keypair.secret_key, message).unwrap()
        })
        .collect();

    let result = multisig.verify(message, &signatures);
    // This should actually succeed since we have enough valid signatures
    assert!(result.is_ok());
}

#[test]
fn test_multisig_collect_valid_signatures_edge_cases() {
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();
    let multisig = Multisig::new(3, 5, public_keys).unwrap();

    let message = b"test message";

    // Test with empty signatures
    let result = multisig.collect_valid_signatures(message, &[]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), vec![] as Vec<usize>);

    // Test with all valid signatures
    let signatures: Vec<_> = keypairs
        .iter()
        .map(|kp| sign_message(&kp.secret_key, message).unwrap())
        .collect();

    let result = multisig.collect_valid_signatures(message, &signatures);
    assert!(result.is_ok());
    let valid_indices = result.unwrap();
    assert_eq!(valid_indices.len(), 5);
    assert_eq!(valid_indices, vec![0, 1, 2, 3, 4]);

    // Test with mixed valid/invalid signatures
    let mut mixed_signatures = signatures.clone();
    // Replace one signature with an invalid one
    mixed_signatures[2] = sign_message(&keypairs[0].secret_key, b"different message").unwrap();

    let result = multisig.collect_valid_signatures(message, &mixed_signatures);
    assert!(result.is_ok());
    let valid_indices = result.unwrap();
    assert_eq!(valid_indices.len(), 4);
    assert!(!valid_indices.contains(&2)); // Index 2 should be invalid
}

#[test]
fn test_multisig_threshold_properties() {
    let keypairs: Vec<_> = (0..7)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

    // Test 1-of-7
    let multisig = Multisig::new(1, 7, public_keys.clone()).unwrap();
    assert_eq!(multisig.threshold(), 1);
    assert_eq!(multisig.total(), 7);

    // Test 7-of-7
    let multisig = Multisig::new(7, 7, public_keys.clone()).unwrap();
    assert_eq!(multisig.threshold(), 7);
    assert_eq!(multisig.total(), 7);

    // Test 4-of-7
    let multisig = Multisig::new(4, 7, public_keys.clone()).unwrap();
    assert_eq!(multisig.threshold(), 4);
    assert_eq!(multisig.total(), 7);
}

#[test]
fn test_multisig_public_keys_access() {
    let keypairs: Vec<_> = (0..3)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();
    let multisig = Multisig::new(2, 3, public_keys.clone()).unwrap();

    let multisig_public_keys = multisig.public_keys();
    assert_eq!(multisig_public_keys.len(), 3);

    // Check that the public keys are the same
    for (i, pubkey) in multisig_public_keys.iter().enumerate() {
        assert_eq!(pubkey, &public_keys[i]);
    }
}

#[test]
fn test_multisig_duplicate_public_keys() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let public_key = keypair.public_key();

    // Create multisig with duplicate public keys
    let duplicate_keys = vec![public_key.clone(), public_key.clone(), public_key.clone()];
    let result = Multisig::new(2, 3, duplicate_keys);
    assert!(result.is_err());
    if let Err(GovernanceError::InvalidMultisig(msg)) = result {
        assert!(msg.contains("Duplicate"));
    }
}

#[test]
fn test_multisig_single_key() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let public_key = keypair.public_key();

    // Test 1-of-1 multisig
    let multisig = Multisig::new(1, 1, vec![public_key]).unwrap();
    assert_eq!(multisig.threshold(), 1);
    assert_eq!(multisig.total(), 1);

    let message = b"test message";
    let signature = sign_message(&keypair.secret_key, message).unwrap();
    let result = multisig.verify(message, &[signature]);
    assert!(result.is_ok());
    assert!(result.unwrap());
}
