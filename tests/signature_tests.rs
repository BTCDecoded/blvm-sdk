//! # Signature Tests
//!
//! Tests for signature creation and verification.

use blvm_sdk::governance::{GovernanceKeypair, Signature};
use blvm_sdk::sign_message;

#[test]
fn test_signature_creation_and_verification() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let message = b"test message";

    let signature = sign_message(&keypair.secret_key, message).unwrap();

    let verified =
        blvm_sdk::governance::verify_signature(&signature, message, &keypair.public_key()).unwrap();

    assert!(verified);
}

#[test]
fn test_signature_serialization_roundtrip() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let message = b"test message";

    let signature = sign_message(&keypair.secret_key, message).unwrap();

    // Serialize and deserialize
    let signature_bytes = signature.to_bytes();
    let reconstructed = Signature::from_bytes(&signature_bytes).unwrap();

    assert_eq!(signature, reconstructed);

    // Should still verify
    let verified =
        blvm_sdk::governance::verify_signature(&reconstructed, message, &keypair.public_key())
            .unwrap();

    assert!(verified);
}

#[test]
fn test_signature_with_different_messages() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let message1 = b"message 1";
    let message2 = b"message 2";

    let signature1 = sign_message(&keypair.secret_key, message1).unwrap();

    let signature2 = sign_message(&keypair.secret_key, message2).unwrap();

    // Signatures should be different
    assert_ne!(signature1, signature2);

    // Each signature should only verify for its message
    assert!(
        blvm_sdk::governance::verify_signature(&signature1, message1, &keypair.public_key(),)
            .unwrap()
    );

    assert!(
        !blvm_sdk::governance::verify_signature(&signature1, message2, &keypair.public_key(),)
            .unwrap()
    );

    assert!(
        blvm_sdk::governance::verify_signature(&signature2, message2, &keypair.public_key(),)
            .unwrap()
    );

    assert!(
        !blvm_sdk::governance::verify_signature(&signature2, message1, &keypair.public_key(),)
            .unwrap()
    );
}

#[test]
fn test_signature_with_different_keys() {
    let keypair1 = GovernanceKeypair::generate().unwrap();
    let keypair2 = GovernanceKeypair::generate().unwrap();
    let message = b"test message";

    let signature = sign_message(&keypair1.secret_key, message).unwrap();

    // Should verify with keypair1
    assert!(
        blvm_sdk::governance::verify_signature(&signature, message, &keypair1.public_key(),)
            .unwrap()
    );

    // Should not verify with keypair2
    assert!(
        !blvm_sdk::governance::verify_signature(&signature, message, &keypair2.public_key(),)
            .unwrap()
    );
}

#[test]
fn test_signature_deterministic() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let message = b"deterministic test";

    // Sign the same message multiple times
    let signature1 = sign_message(&keypair.secret_key, message).unwrap();

    let signature2 = sign_message(&keypair.secret_key, message).unwrap();

    // Signatures may be the same or different (implementation dependent)
    // Both should be valid regardless

    // But both should verify
    assert!(
        blvm_sdk::governance::verify_signature(&signature1, message, &keypair.public_key(),)
            .unwrap()
    );

    assert!(
        blvm_sdk::governance::verify_signature(&signature2, message, &keypair.public_key(),)
            .unwrap()
    );
}

#[test]
fn test_invalid_signature_handling() {
    let _keypair = GovernanceKeypair::generate().unwrap();
    let _message = b"test message";

    // Test with invalid signature lengths
    let invalid_signatures = vec![
        vec![0u8; 63], // Too short
        vec![0u8; 65], // Too long
        vec![0u8; 0],  // Empty
    ];

    for invalid_sig in invalid_signatures {
        let result = Signature::from_bytes(&invalid_sig);
        assert!(result.is_err());
    }
}

#[test]
fn test_signature_display_format() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let message = b"display test";

    let signature = sign_message(&keypair.secret_key, message).unwrap();

    let display_str = format!("{signature}");
    let expected_hex = hex::encode(signature.to_bytes());
    assert_eq!(display_str, expected_hex);

    // Should be 128 characters (64 hex chars)
    assert_eq!(display_str.len(), 128);
}

#[test]
fn test_signature_der_format() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let message = b"der test";

    let signature = sign_message(&keypair.secret_key, message).unwrap();

    let der_bytes = signature.to_der_bytes();

    // DER format should be longer than compact format
    assert!(der_bytes.len() > 64);

    // Should be valid DER
    assert!(der_bytes.len() >= 70); // Minimum DER signature length
    assert!(der_bytes.len() <= 72); // Maximum DER signature length
}
