//! # Governance Signatures
//!
//! Signature creation and verification for governance operations.

use rand::rngs::OsRng;
use secp256k1::{ecdsa::Signature as Secp256k1Signature, Message, Secp256k1, SecretKey};
use sha2::Digest;
use std::fmt;

use crate::governance::error::{GovernanceError, GovernanceResult};

/// A governance signature
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub(crate) inner: Secp256k1Signature,
}

impl Signature {
    /// Create a signature from bytes
    pub fn from_bytes(bytes: &[u8]) -> GovernanceResult<Self> {
        let signature = Secp256k1Signature::from_compact(bytes).map_err(|e| {
            GovernanceError::InvalidSignatureFormat(format!("Invalid signature: {e}"))
        })?;

        Ok(Self { inner: signature })
    }

    /// Get the signature bytes
    pub fn to_bytes(&self) -> [u8; 64] {
        self.inner.serialize_compact()
    }

    /// Get the signature in DER format
    pub fn to_der_bytes(&self) -> Vec<u8> {
        self.inner.serialize_der().to_vec()
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.to_bytes()))
    }
}

/// Sign a message with a secret key
pub fn sign_message(secret_key: &SecretKey, message: &[u8]) -> GovernanceResult<Signature> {
    let secp = Secp256k1::new();
    let _rng = OsRng;

    // Hash the message using SHA256 (Bitcoin standard)
    let message_hash = sha2::Sha256::digest(message);
    let message = Message::from_digest_slice(&message_hash)
        .map_err(|e| GovernanceError::Cryptographic(format!("Invalid message hash: {e}")))?;

    let signature = secp.sign_ecdsa(&message, secret_key);

    Ok(Signature { inner: signature })
}

/// Verify a signature against a message and public key
pub fn verify_signature(
    signature: &Signature,
    message: &[u8],
    public_key: &crate::governance::PublicKey,
) -> GovernanceResult<bool> {
    let secp = Secp256k1::new();

    // Hash the message using SHA256 (Bitcoin standard)
    let message_hash = sha2::Sha256::digest(message);
    let message = Message::from_digest_slice(&message_hash)
        .map_err(|e| GovernanceError::Cryptographic(format!("Invalid message hash: {e}")))?;

    let result = secp.verify_ecdsa(&message, &signature.inner, &public_key.inner);

    Ok(result.is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::GovernanceKeypair;

    #[test]
    fn test_sign_and_verify() {
        let keypair = GovernanceKeypair::generate().unwrap();
        let message = b"test message";

        let signature = sign_message(&keypair.secret_key, message).unwrap();
        let verified = verify_signature(&signature, message, &keypair.public_key()).unwrap();

        assert!(verified);
    }

    #[test]
    fn test_signature_serialization() {
        let keypair = GovernanceKeypair::generate().unwrap();
        let message = b"test message";

        let signature = sign_message(&keypair.secret_key, message).unwrap();
        let bytes = signature.to_bytes();

        let reconstructed = Signature::from_bytes(&bytes).unwrap();
        assert_eq!(signature, reconstructed);
    }

    #[test]
    fn test_invalid_signature() {
        let keypair = GovernanceKeypair::generate().unwrap();
        let message = b"test message";

        let signature = sign_message(&keypair.secret_key, message).unwrap();
        let wrong_message = b"wrong message";

        let verified = verify_signature(&signature, wrong_message, &keypair.public_key()).unwrap();
        assert!(!verified);
    }

    #[test]
    fn test_invalid_signature_format() {
        let invalid_bytes = [0u8; 63]; // Wrong length
        let result = Signature::from_bytes(&invalid_bytes);
        assert!(result.is_err());
    }
}
