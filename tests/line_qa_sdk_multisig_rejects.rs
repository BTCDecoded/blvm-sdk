//! Rejects `Multisig::new` already returns before any signature is checked.

use blvm_sdk::{GovernanceError, Multisig, PublicKey};

fn key(byte: u8) -> PublicKey {
    PublicKey {
        public_key_bytes: [byte; 33],
    }
}

#[test]
fn multisig_new_rejects_a_zero_threshold() {
    match Multisig::new(0, 1, vec![key(1)]) {
        Err(GovernanceError::InvalidThreshold { threshold, total }) => {
            assert_eq!(threshold, 0);
            assert_eq!(total, 1);
        }
        Ok(_) => panic!("zero threshold was accepted"),
        Err(other) => panic!("zero threshold returned {other:?}"),
    }
}

#[test]
fn multisig_new_rejects_a_threshold_above_the_key_count() {
    match Multisig::new(3, 2, vec![key(1), key(2)]) {
        Err(GovernanceError::InvalidThreshold { threshold, total }) => {
            assert_eq!(threshold, 3);
            assert_eq!(total, 2);
        }
        Ok(_) => panic!("3-of-2 was accepted"),
        Err(other) => panic!("3-of-2 returned {other:?}"),
    }
}

#[test]
fn multisig_new_rejects_a_key_count_that_does_not_match_total() {
    match Multisig::new(1, 2, vec![key(1)]) {
        Err(GovernanceError::InvalidMultisig(message)) => {
            assert!(
                message.contains("Expected 2 public keys, got 1"),
                "key count returned {message}"
            );
        }
        Ok(_) => panic!("one key for a 2-key set was accepted"),
        Err(other) => panic!("key count returned {other:?}"),
    }
}

#[test]
fn multisig_new_rejects_a_duplicate_public_key() {
    match Multisig::new(1, 2, vec![key(1), key(1)]) {
        Err(GovernanceError::InvalidMultisig(message)) => {
            assert!(
                message.contains("Duplicate public keys not allowed"),
                "duplicate key returned {message}"
            );
        }
        Ok(_) => panic!("duplicate public key was accepted"),
        Err(other) => panic!("duplicate key returned {other:?}"),
    }
}
