//! # Key Generation Tests
//!
//! Tests for key generation edge cases and validation.

use blvm_sdk::governance::GovernanceKeypair;

#[test]
fn test_keypair_generation_randomness() {
    // Generate multiple keypairs and ensure they're different
    let keypairs: Vec<_> = (0..100)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();

    // All public keys should be unique
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();
    let unique_keys: std::collections::HashSet<_> = public_keys.iter().collect();
    assert_eq!(unique_keys.len(), public_keys.len());

    // All secret keys should be unique
    let secret_keys: Vec<_> = keypairs.iter().map(|kp| kp.secret_key_bytes()).collect();
    let unique_secrets: std::collections::HashSet<_> = secret_keys.iter().collect();
    assert_eq!(unique_secrets.len(), secret_keys.len());
}

#[test]
fn test_deterministic_keypair_generation() {
    let seed = [1u8; 32];

    // Generate keypair from seed
    let keypair1 = GovernanceKeypair::from_secret_key(&seed).unwrap();
    let keypair2 = GovernanceKeypair::from_secret_key(&seed).unwrap();

    // Should be identical
    assert_eq!(keypair1.public_key(), keypair2.public_key());
    assert_eq!(keypair1.secret_key_bytes(), keypair2.secret_key_bytes());
}

#[test]
fn test_keypair_serialization_roundtrip() {
    let keypair = GovernanceKeypair::generate().unwrap();

    // Test public key serialization
    let pubkey_bytes = keypair.public_key().to_bytes();
    let reconstructed_pubkey = blvm_sdk::governance::PublicKey::from_bytes(&pubkey_bytes).unwrap();
    assert_eq!(keypair.public_key(), reconstructed_pubkey);

    // Test secret key serialization
    let secret_bytes = keypair.secret_key_bytes();
    let reconstructed_keypair = GovernanceKeypair::from_secret_key(&secret_bytes).unwrap();
    assert_eq!(keypair.public_key(), reconstructed_keypair.public_key());
}

#[test]
fn test_invalid_secret_key_handling() {
    // Test with invalid key lengths
    let invalid_keys = vec![
        vec![0u8; 31], // Too short
        vec![0u8; 33], // Too long
        vec![0u8; 0],  // Empty
    ];

    for invalid_key in invalid_keys {
        let result = GovernanceKeypair::from_secret_key(&invalid_key);
        assert!(result.is_err());
    }
}

#[test]
fn test_invalid_public_key_handling() {
    // Test with invalid public key lengths
    let invalid_keys = vec![
        vec![0u8; 32], // Wrong length for compressed
        vec![0u8; 64], // Wrong length for uncompressed
        vec![0u8; 0],  // Empty
    ];

    for invalid_key in invalid_keys {
        let result = blvm_sdk::governance::PublicKey::from_bytes(&invalid_key);
        assert!(result.is_err());
    }
}

#[test]
fn test_keypair_consistency() {
    let keypair = GovernanceKeypair::generate().unwrap();

    // Public key should be derivable from secret key via from_secret_key
    let keypair2 = GovernanceKeypair::from_secret_key(&keypair.secret_key).unwrap();
    assert_eq!(keypair.public_key(), keypair2.public_key());
}

#[test]
fn test_keypair_display_format() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let pubkey = keypair.public_key();

    // Display should be hex-encoded
    let display_str = format!("{pubkey}");
    let expected_hex = hex::encode(pubkey.to_bytes());
    assert_eq!(display_str, expected_hex);

    // Should be 66 characters (0x + 64 hex chars)
    assert_eq!(display_str.len(), 66);
}

#[test]
fn test_keypair_equality() {
    let keypair1 = GovernanceKeypair::generate().unwrap();
    let keypair2 = GovernanceKeypair::generate().unwrap();

    // Different keypairs should not be equal
    assert_ne!(keypair1.public_key(), keypair2.public_key());

    // Same keypair should be equal to itself
    assert_eq!(keypair1.public_key(), keypair1.public_key());
}
