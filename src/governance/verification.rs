//! # Governance Verification
//!
//! Verification utilities for governance operations.

use crate::governance::error::{GovernanceError, GovernanceResult};
use crate::governance::{PublicKey, Signature};

/// Verify a signature against a message and public key
pub fn verify_signature(
    signature: &Signature,
    message: &[u8],
    public_key: &PublicKey,
) -> GovernanceResult<bool> {
    crate::governance::signatures::verify_signature(signature, message, public_key)
}

/// Verify a signature against a message hash
pub fn verify_signature_hash(
    signature: &Signature,
    message_hash: &[u8],
    public_key: &PublicKey,
) -> GovernanceResult<bool> {
    use secp256k1::{Message, Secp256k1};

    let secp = Secp256k1::new();

    let message = Message::from_digest_slice(message_hash)
        .map_err(|e| GovernanceError::Cryptographic(format!("Invalid message hash: {e}")))?;

    let result = secp.verify_ecdsa(&message, &signature.inner, &public_key.inner);

    Ok(result.is_ok())
}

/// Verify multiple signatures against a message
pub fn verify_multiple_signatures(
    signatures: &[Signature],
    message: &[u8],
    public_keys: &[PublicKey],
) -> GovernanceResult<Vec<bool>> {
    let mut results = Vec::new();

    for signature in signatures {
        let mut verified = false;
        for public_key in public_keys {
            if verify_signature(signature, message, public_key)? {
                verified = true;
                break;
            }
        }
        results.push(verified);
    }

    Ok(results)
}

/// Verify a signature against a specific public key
pub fn verify_signature_with_key(
    signature: &Signature,
    message: &[u8],
    public_key: &PublicKey,
) -> GovernanceResult<bool> {
    verify_signature(signature, message, public_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::GovernanceKeypair;
    use sha2::Digest;

    #[test]
    fn test_verify_signature() {
        let keypair = GovernanceKeypair::generate().unwrap();
        let message = b"test message";

        let signature = crate::sign_message(&keypair.secret_key, message).unwrap();
        let verified = verify_signature(&signature, message, &keypair.public_key()).unwrap();

        assert!(verified);
    }

    #[test]
    fn test_verify_signature_hash() {
        let keypair = GovernanceKeypair::generate().unwrap();
        let message = b"test message";
        let message_hash = sha2::Sha256::digest(message);

        let signature = crate::sign_message(&keypair.secret_key, message).unwrap();
        let verified =
            verify_signature_hash(&signature, &message_hash, &keypair.public_key()).unwrap();

        assert!(verified);
    }

    #[test]
    fn test_verify_multiple_signatures() {
        let keypairs: Vec<_> = (0..3)
            .map(|_| GovernanceKeypair::generate().unwrap())
            .collect();
        let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();
        let message = b"test message";

        let signatures: Vec<_> = keypairs
            .iter()
            .map(|kp| crate::sign_message(&kp.secret_key, message).unwrap())
            .collect();

        let results = verify_multiple_signatures(&signatures, message, &public_keys).unwrap();

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|&verified| verified));
    }

    #[test]
    fn test_verify_signature_with_wrong_key() {
        let keypair1 = GovernanceKeypair::generate().unwrap();
        let keypair2 = GovernanceKeypair::generate().unwrap();
        let message = b"test message";

        let signature = crate::sign_message(&keypair1.secret_key, message).unwrap();
        let verified = verify_signature(&signature, message, &keypair2.public_key()).unwrap();

        assert!(!verified);
    }
}
