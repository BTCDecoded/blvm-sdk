//! # Coverage Tests
//!
//! Additional tests to reach 90%+ coverage on governance crypto code.

use blvm_sdk::cli::input::{parse_base64, parse_comma_separated, parse_hex, parse_threshold};
use blvm_sdk::cli::output::{OutputFormat, OutputFormatter};
use blvm_sdk::governance::{GovernanceKeypair, GovernanceMessage, Multisig, PublicKey, Signature};
use blvm_sdk::{sign_message, verify_signature};

#[test]
fn test_governance_keypair_display() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let display = format!("{}", keypair.public_key());
    assert!(!display.is_empty());
    assert_eq!(display.len(), 66); // 33 bytes * 2 hex chars
}

#[test]
fn test_governance_keypair_serialization_edge_cases() {
    // Test with edge case secret key (all zeros except last byte)
    let mut secret_bytes = [0u8; 32];
    secret_bytes[31] = 1;
    let keypair = GovernanceKeypair::from_secret_key(&secret_bytes).unwrap();

    // Test public key serialization
    let pubkey_bytes = keypair.public_key().to_bytes();
    assert_eq!(pubkey_bytes.len(), 33);

    // Test hex representation
    let hex_str = format!("{}", keypair.public_key());
    assert_eq!(hex_str.len(), 66);
}

#[test]
fn test_signature_der_serialization() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let message = b"der serialization test";
    let signature = sign_message(&keypair.secret_key, message).unwrap();

    let der_bytes = signature.to_der_bytes();
    assert!(!der_bytes.is_empty());
    assert!(der_bytes.len() >= 70); // Minimum DER signature length
    assert!(der_bytes.len() <= 72); // Maximum DER signature length
}

#[test]
fn test_multisig_edge_cases() {
    // Test 1-of-1 multisig
    let keypair = GovernanceKeypair::generate().unwrap();
    let multisig = Multisig::new(1, 1, vec![keypair.public_key()]).unwrap();

    let message = b"1-of-1 test";
    let signature = sign_message(&keypair.secret_key, message).unwrap();

    assert!(multisig.verify(message, &[signature]).unwrap());
}

#[test]
fn test_multisig_all_signatures() {
    // Test with all possible signatures
    let keypairs: Vec<_> = (0..3)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();
    let multisig = Multisig::new(2, 3, public_keys).unwrap();

    let message = b"all signatures test";
    let signatures: Vec<_> = keypairs
        .iter()
        .map(|kp| sign_message(&kp.secret_key, message).unwrap())
        .collect();

    assert!(multisig.verify(message, &signatures).unwrap());
}

#[test]
fn test_governance_message_edge_cases() {
    // Test with empty strings
    let message = GovernanceMessage::Release {
        version: "".to_string(),
        commit_hash: "".to_string(),
    };
    let signing_bytes = message.to_signing_bytes();
    assert_eq!(signing_bytes, b"RELEASE::");

    // Test with unicode characters
    let message = GovernanceMessage::BudgetDecision {
        amount: 0,
        purpose: "测试".to_string(),
    };
    let signing_bytes = message.to_signing_bytes();
    assert!(!signing_bytes.is_empty());
}

#[test]
fn test_cli_input_edge_cases() {
    // Test parse_hex with empty string
    let result = parse_hex("");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), vec![] as Vec<u8>);

    // Test parse_base64 with empty string
    let result = parse_base64("");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), vec![] as Vec<u8>);

    // Test parse_threshold with edge cases
    let result = parse_threshold("1-of-1");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), (1, 1));

    // Test parse_comma_separated with empty string
    let result = parse_comma_separated("");
    assert_eq!(result, vec![] as Vec<String>);

    // Test parse_comma_separated with whitespace
    let result = parse_comma_separated("  a  ,  b  ,  c  ");
    assert_eq!(result, vec!["a", "b", "c"]);
}

#[test]
fn test_cli_output_edge_cases() {
    let formatter = OutputFormatter::new(OutputFormat::Text);

    // Test with empty string
    let empty = "";
    let result = formatter.format(&empty);
    assert_eq!(result, Ok("".to_string()));

    // Test JSON formatter with complex data
    let json_formatter = OutputFormatter::new(OutputFormat::Json);
    let data = serde_json::json!({
        "nested": {
            "array": [1, 2, 3],
            "string": "test"
        }
    });
    let result = json_formatter.format(&data);
    assert!(result.unwrap().contains("nested"));
}

#[test]
fn test_verification_edge_cases() {
    let keypair = GovernanceKeypair::generate().unwrap();
    let message = b"verification edge case";
    let _signature = sign_message(&keypair.secret_key, message).unwrap();

    // Test with empty message
    let empty_message = b"";
    let empty_signature = sign_message(&keypair.secret_key, empty_message).unwrap();
    assert!(verify_signature(&empty_signature, empty_message, &keypair.public_key()).unwrap());

    // Test with very long message
    let long_message = vec![0u8; 10000];
    let long_signature = sign_message(&keypair.secret_key, &long_message).unwrap();
    assert!(verify_signature(&long_signature, &long_message, &keypair.public_key()).unwrap());
}

#[test]
fn test_multisig_collect_valid_signatures_edge_cases() {
    let keypairs: Vec<_> = (0..5)
        .map(|_| GovernanceKeypair::generate().unwrap())
        .collect();
    let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();
    let multisig = Multisig::new(3, 5, public_keys).unwrap();

    let message = b"collect valid signatures test";

    // Test with no signatures
    let valid_indices = multisig.collect_valid_signatures(message, &[]).unwrap();
    assert_eq!(valid_indices, vec![] as Vec<usize>);

    // Test with all valid signatures
    let signatures: Vec<_> = keypairs
        .iter()
        .map(|kp| sign_message(&kp.secret_key, message).unwrap())
        .collect();
    let valid_indices = multisig
        .collect_valid_signatures(message, &signatures)
        .unwrap();
    assert_eq!(valid_indices.len(), 5);
}

#[test]
fn test_signature_from_bytes_edge_cases() {
    // Test with invalid signature bytes
    let invalid_bytes = [0u8; 64];
    let result = Signature::from_bytes(&invalid_bytes);
    // This should succeed (all zeros is technically a valid signature format)
    assert!(result.is_ok());

    // Test with wrong length
    let wrong_length = [0u8; 32];
    let result = Signature::from_bytes(&wrong_length);
    assert!(result.is_err());
}

#[test]
fn test_public_key_from_bytes_edge_cases() {
    // Test with invalid public key bytes
    let invalid_bytes = [0u8; 32];
    let result = PublicKey::from_bytes(&invalid_bytes);
    assert!(result.is_err());

    // Test with wrong length
    let wrong_length = [0u8; 33];
    let _result = PublicKey::from_bytes(&wrong_length);
    // This might succeed if it's a valid compressed public key
    // Let's test with a known invalid one
    let mut invalid_pubkey = [0u8; 33];
    invalid_pubkey[0] = 0x04; // Uncompressed format marker
    let result = PublicKey::from_bytes(&invalid_pubkey);
    assert!(result.is_err());
}
