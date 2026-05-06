//! # Governance Key Management
//!
//! Key generation and management for governance operations.

use rand::rngs::OsRng;
use secp256k1::{PublicKey as Secp256k1PublicKey, Secp256k1, SecretKey};
use std::fmt;

use crate::governance::error::{GovernanceError, GovernanceResult};

/// A governance keypair for signing governance messages
#[derive(Debug, Clone)]
pub struct GovernanceKeypair {
    pub secret_key: SecretKey,
    pub public_key: Secp256k1PublicKey,
}

/// A public key for governance operations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PublicKey {
    pub inner: Secp256k1PublicKey,
}

impl GovernanceKeypair {
    /// Generate a new random keypair
    pub fn generate() -> GovernanceResult<Self> {
        let secp = Secp256k1::new();
        let mut rng = OsRng;

        let secret_key = SecretKey::new(&mut rng);
        let public_key = secret_key.public_key(&secp);

        Ok(Self {
            secret_key,
            public_key,
        })
    }

    /// Create a keypair from a secret key
    pub fn from_secret_key(secret_bytes: &[u8]) -> GovernanceResult<Self> {
        let secp = Secp256k1::new();

        let secret_key = SecretKey::from_slice(secret_bytes)
            .map_err(|e| GovernanceError::InvalidKey(format!("Invalid secret key: {e}")))?;

        let public_key = secret_key.public_key(&secp);

        Ok(Self {
            secret_key,
            public_key,
        })
    }

    /// Get the public key
    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            inner: self.public_key,
        }
    }

    /// Get the secret key bytes
    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.secret_key.secret_bytes()
    }

    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> [u8; 33] {
        self.public_key.serialize()
    }
}

impl PublicKey {
    /// Create a public key from bytes
    pub fn from_bytes(bytes: &[u8]) -> GovernanceResult<Self> {
        let public_key = Secp256k1PublicKey::from_slice(bytes)
            .map_err(|e| GovernanceError::InvalidKey(format!("Invalid public key: {e}")))?;

        Ok(Self { inner: public_key })
    }

    /// Get the public key bytes
    pub fn to_bytes(&self) -> [u8; 33] {
        self.inner.serialize()
    }

    /// Get the compressed public key bytes
    pub fn to_compressed_bytes(&self) -> [u8; 33] {
        self.inner.serialize()
    }

    /// Get the uncompressed public key bytes
    pub fn to_uncompressed_bytes(&self) -> [u8; 65] {
        self.inner.serialize_uncompressed()
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

        // Verify the public key can be serialized and deserialized
        let bytes = public_key.to_bytes();
        let reconstructed = PublicKey::from_bytes(&bytes).unwrap();
        assert_eq!(public_key, reconstructed);
    }

    #[test]
    fn test_keypair_from_secret_key() {
        let keypair1 = GovernanceKeypair::generate().unwrap();
        let secret_bytes = keypair1.secret_key_bytes();

        let keypair2 = GovernanceKeypair::from_secret_key(&secret_bytes).unwrap();

        // Both keypairs should have the same public key
        assert_eq!(keypair1.public_key(), keypair2.public_key());
    }

    #[test]
    fn test_invalid_secret_key() {
        let invalid_bytes = [0u8; 31]; // Too short
        let result = GovernanceKeypair::from_secret_key(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_public_key() {
        let invalid_bytes = [0u8; 32]; // Wrong length for public key
        let result = PublicKey::from_bytes(&invalid_bytes);
        assert!(result.is_err());
    }
}
