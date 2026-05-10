//! # Governance Signatures

use blvm_secp256k1::ecdsa::{
    ecdsa_sig_parse_compact, ecdsa_sig_verify, ecdsa_sign_compact_rfc6979, ge_from_pubkey_bytes,
};
use blvm_secp256k1::scalar::Scalar;
use sha2::Digest;
use std::fmt;

use crate::governance::error::{GovernanceError, GovernanceResult};

/// A compact (64-byte) governance signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub(crate) bytes: [u8; 64],
}

impl Signature {
    /// Create a signature from compact 64-byte encoding.
    pub fn from_bytes(bytes: &[u8]) -> GovernanceResult<Self> {
        let arr: [u8; 64] = bytes.try_into().map_err(|_| {
            GovernanceError::InvalidSignatureFormat(
                "Signature must be 64 bytes (compact r||s)".to_string(),
            )
        })?;
        // Validate parsability.
        ecdsa_sig_parse_compact(&arr).ok_or_else(|| {
            GovernanceError::InvalidSignatureFormat("Invalid compact signature".to_string())
        })?;
        Ok(Self { bytes: arr })
    }

    /// Get compact 64-byte encoding.
    pub fn to_bytes(&self) -> [u8; 64] {
        self.bytes
    }

    /// DER-encoded form (for legacy compatibility).
    pub fn to_der_bytes(&self) -> Vec<u8> {
        use blvm_secp256k1::ecdsa::ecdsa_sig_serialize_der;
        let (r, s) =
            ecdsa_sig_parse_compact(&self.bytes).expect("bytes were validated on construction");
        ecdsa_sig_serialize_der(&r, &s)
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.to_bytes()))
    }
}

/// Sign a message with a 32-byte secret key (SHA-256 hash, RFC 6979 nonce).
pub fn sign_message(secret_key: &[u8; 32], message: &[u8]) -> GovernanceResult<Signature> {
    let hash: [u8; 32] = sha2::Sha256::digest(message).into();
    let compact = ecdsa_sign_compact_rfc6979(&hash, secret_key)
        .ok_or_else(|| GovernanceError::Cryptographic("ECDSA signing failed".to_string()))?;
    Ok(Signature { bytes: compact })
}

/// Verify a signature against a message and public key.
pub fn verify_signature(
    signature: &Signature,
    message: &[u8],
    public_key: &crate::governance::PublicKey,
) -> GovernanceResult<bool> {
    let hash: [u8; 32] = sha2::Sha256::digest(message).into();
    verify_hash(signature, &hash, public_key)
}

/// Verify a signature against a pre-computed 32-byte message hash.
pub fn verify_signature_hash(
    signature: &Signature,
    message_hash: &[u8; 32],
    public_key: &crate::governance::PublicKey,
) -> GovernanceResult<bool> {
    verify_hash(signature, message_hash, public_key)
}

fn verify_hash(
    signature: &Signature,
    hash: &[u8; 32],
    public_key: &crate::governance::PublicKey,
) -> GovernanceResult<bool> {
    let (sigr, sigs) = match ecdsa_sig_parse_compact(&signature.bytes) {
        Some(p) => p,
        None => return Ok(false),
    };
    let pk = match ge_from_pubkey_bytes(&public_key.public_key_bytes) {
        Some(p) => p,
        None => {
            return Err(GovernanceError::InvalidKey(
                "Invalid public key".to_string(),
            ))
        }
    };
    let mut msg = Scalar::zero();
    let _ = msg.set_b32(hash);
    Ok(ecdsa_sig_verify(&sigr, &sigs, &pk, &msg))
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
        let verified =
            verify_signature(&signature, b"wrong message", &keypair.public_key()).unwrap();
        assert!(!verified);
    }

    #[test]
    fn test_invalid_signature_format() {
        let result = Signature::from_bytes(&[0u8; 63]);
        assert!(result.is_err());
    }
}
