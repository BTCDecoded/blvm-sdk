//! Additional tests for governance keys module to improve coverage.

use blvm_sdk::governance::{GovernanceError, GovernanceKeypair, PublicKey};

#[test]
fn test_governance_keypair_debug_format() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let debug_str = format!("{keypair:?}");
    assert!(debug_str.contains("GovernanceKeypair"));
    assert!(debug_str.contains("secret_key"));
    assert!(debug_str.contains("public_key"));
}

#[test]
fn test_governance_keypair_clone() {
    let keypair1 = GovernanceKeypair::generate().unwrap();
    let keypair2 = keypair1.clone();

    // Test that clone works and properties are preserved
    assert_eq!(keypair1.public_key(), keypair2.public_key());
    assert_eq!(keypair1.secret_key_bytes(), keypair2.secret_key_bytes());
}

#[test]
fn test_governance_keypair_from_secret_key_edge_cases() {
    // Test with all zeros (might be invalid depending on implementation)
    let zero_secret = [0u8; 32];
    let result = GovernanceKeypair::from_secret_key(&zero_secret);
    // This might be invalid, so we just test that it doesn't panic
    let _ = result;

    // Test with all 0xFF (might be valid depending on implementation)
    let max_secret = [0xFFu8; 32];
    let result = GovernanceKeypair::from_secret_key(&max_secret);
    // This might be invalid, so we just test that it doesn't panic
    let _ = result;

    // Test with invalid length
    let short_secret = [0u8; 31];
    let result = GovernanceKeypair::from_secret_key(&short_secret);
    assert!(result.is_err());

    let long_secret = [0u8; 33];
    let result = GovernanceKeypair::from_secret_key(&long_secret);
    assert!(result.is_err());
}

#[test]
fn test_public_key_debug_format() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let public_key = keypair.public_key();
    let debug_str = format!("{public_key:?}");
    assert!(debug_str.contains("PublicKey"));
}

#[test]
fn test_public_key_clone() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let pubkey1 = keypair.public_key();
    let pubkey2 = pubkey1.clone();

    // Test that clone works and properties are preserved
    assert_eq!(pubkey1.to_bytes(), pubkey2.to_bytes());
}

#[test]
fn test_public_key_from_bytes_edge_cases() {
    // Test with invalid compressed public key (wrong prefix)
    let mut invalid_compressed = [0u8; 33];
    invalid_compressed[0] = 0x04; // Uncompressed format marker
    let result = PublicKey::from_bytes(&invalid_compressed);
    assert!(result.is_err());

    // Test with invalid compressed public key (invalid point)
    let mut invalid_point = [0u8; 33];
    invalid_point[0] = 0x02; // Compressed format marker
                             // All zeros is not a valid point
    let result = PublicKey::from_bytes(&invalid_point);
    assert!(result.is_err());

    // Test with wrong length
    let wrong_length = [0u8; 32];
    let result = PublicKey::from_bytes(&wrong_length);
    assert!(result.is_err());

    let wrong_length = [0u8; 34];
    let result = PublicKey::from_bytes(&wrong_length);
    assert!(result.is_err());
}

#[test]
fn test_public_key_to_bytes() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let public_key = keypair.public_key();
    let bytes = public_key.to_bytes();

    assert_eq!(bytes.len(), 33);
    assert!(bytes[0] == 0x02 || bytes[0] == 0x03); // Compressed format marker

    // Test roundtrip
    let reconstructed = PublicKey::from_bytes(&bytes).unwrap();
    assert_eq!(public_key, reconstructed);
}

#[test]
fn test_governance_error_display() {
    let error = GovernanceError::InvalidKey("test error".to_string());
    let display_str = format!("{error}");
    assert!(display_str.contains("Invalid key"));
    assert!(display_str.contains("test error"));

    let error = GovernanceError::SignatureVerification("sig error".to_string());
    let display_str = format!("{error}");
    assert!(display_str.contains("Signature verification failed"));
    assert!(display_str.contains("sig error"));

    let error = GovernanceError::InvalidThreshold {
        threshold: 2,
        total: 1,
    };
    let display_str = format!("{error}");
    assert!(display_str.contains("Invalid threshold"));
    assert!(display_str.contains("2"));
    assert!(display_str.contains("1"));
}

#[test]
fn test_governance_error_debug() {
    let error = GovernanceError::InvalidKey("test error".to_string());
    let debug_str = format!("{error:?}");
    assert!(debug_str.contains("InvalidKey"));
    assert!(debug_str.contains("test error"));
}

#[test]
fn test_governance_error_variants() {
    // Test different error variants
    let error1 = GovernanceError::InvalidKey("test".to_string());
    let error2 = GovernanceError::SignatureVerification("test".to_string());
    let error3 = GovernanceError::InvalidMultisig("test".to_string());
    let error4 = GovernanceError::MessageFormat("test".to_string());
    let error5 = GovernanceError::Cryptographic("test".to_string());
    let error6 = GovernanceError::Serialization("test".to_string());
    let error7 = GovernanceError::InvalidThreshold {
        threshold: 1,
        total: 2,
    };
    let error8 = GovernanceError::InsufficientSignatures { got: 1, need: 2 };
    let error9 = GovernanceError::InvalidSignatureFormat("test".to_string());

    // Test that all variants can be formatted
    assert!(!format!("{error1}").is_empty());
    assert!(!format!("{error2}").is_empty());
    assert!(!format!("{error3}").is_empty());
    assert!(!format!("{error4}").is_empty());
    assert!(!format!("{error5}").is_empty());
    assert!(!format!("{error6}").is_empty());
    assert!(!format!("{error7}").is_empty());
    assert!(!format!("{error8}").is_empty());
    assert!(!format!("{error9}").is_empty());
}
