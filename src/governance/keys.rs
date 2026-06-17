//! # Governance Key Management

use crate::governance::error::{GovernanceError, GovernanceResult};
use blvm_secp256k1::ecdsa::{
    ge_from_pubkey_bytes, ge_to_compressed, ge_to_uncompressed, pubkey_from_secret,
};
use blvm_secp256k1::scalar::Scalar;
use rand::RngCore;
use rand::rngs::OsRng;
use std::fmt;

/// A governance keypair for signing governance messages.
#[derive(Debug, Clone)]
pub struct GovernanceKeypair {
    pub secret_key: [u8; 32],
    pub public_key: [u8; 33],
}

/// A public key for governance operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PublicKey {
    pub public_key_bytes: [u8; 33],
}

impl GovernanceKeypair {
    /// Generate a new random keypair.
    pub fn generate() -> GovernanceResult<Self> {
        loop {
            let mut bytes = [0u8; 32];
            OsRng.fill_bytes(&mut bytes);
            let mut sec = Scalar::zero();
            if sec.set_b32(&bytes) || sec.is_zero() {
                continue;
            }
            let ge = pubkey_from_secret(&sec);
            let public_key = ge_to_compressed(&ge);
            return Ok(Self {
                secret_key: bytes,
                public_key,
            });
        }
    }

    /// Create a keypair from a secret key (raw 32-byte scalar).
    pub fn from_secret_key(secret_bytes: &[u8]) -> GovernanceResult<Self> {
        let arr: [u8; 32] = secret_bytes
            .try_into()
            .map_err(|_| GovernanceError::InvalidKey("Secret key must be 32 bytes".to_string()))?;
        let mut sec = Scalar::zero();
        if sec.set_b32(&arr) || sec.is_zero() {
            return Err(GovernanceError::InvalidKey(
                "Invalid secret key scalar".to_string(),
            ));
        }
        let public_key = ge_to_compressed(&pubkey_from_secret(&sec));
        Ok(Self {
            secret_key: arr,
            public_key,
        })
    }

    /// Get the public key.
    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            public_key_bytes: self.public_key,
        }
    }

    /// Get the secret key bytes.
    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.secret_key
    }

    /// Get the public key bytes (compressed).
    pub fn public_key_bytes(&self) -> [u8; 33] {
        self.public_key
    }
}

impl PublicKey {
    /// Create a public key from bytes (compressed 33 B or uncompressed 65 B).
    pub fn from_bytes(bytes: &[u8]) -> GovernanceResult<Self> {
        ge_from_pubkey_bytes(bytes)
            .map(|ge| PublicKey {
                public_key_bytes: ge_to_compressed(&ge),
            })
            .ok_or_else(|| GovernanceError::InvalidKey("Invalid public key".to_string()))
    }

    /// Get the compressed public key bytes (33 bytes).
    pub fn to_bytes(&self) -> [u8; 33] {
        self.public_key_bytes
    }

    /// Get the compressed public key bytes (33 bytes).
    pub fn to_compressed_bytes(&self) -> [u8; 33] {
        self.public_key_bytes
    }

    /// Get the uncompressed public key bytes (65 bytes).
    pub fn to_uncompressed_bytes(&self) -> [u8; 65] {
        let ge =
            ge_from_pubkey_bytes(&self.public_key_bytes).expect("stored pubkey is always valid");
        ge_to_uncompressed(&ge)
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.to_bytes()))
    }
}

impl fmt::Display for GovernanceKeypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GovernanceKeypair(pubkey: {})", self.public_key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = GovernanceKeypair::generate().unwrap();
        let public_key = keypair.public_key();
        let bytes = public_key.to_bytes();
        let reconstructed = PublicKey::from_bytes(&bytes).unwrap();
        assert_eq!(public_key, reconstructed);
    }

    #[test]
    fn test_keypair_from_secret_key() {
        let keypair1 = GovernanceKeypair::generate().unwrap();
        let secret_bytes = keypair1.secret_key_bytes();
        let keypair2 = GovernanceKeypair::from_secret_key(&secret_bytes).unwrap();
        assert_eq!(keypair1.public_key(), keypair2.public_key());
    }

    #[test]
    fn test_invalid_secret_key() {
        let invalid_bytes = [0u8; 31];
        let result = GovernanceKeypair::from_secret_key(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_public_key() {
        let invalid_bytes = [0u8; 32];
        let result = PublicKey::from_bytes(&invalid_bytes);
        assert!(result.is_err());
    }
}
