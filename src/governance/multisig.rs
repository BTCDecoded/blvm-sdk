//! # Multisig Operations
//!
//! Multisig threshold logic and signature collection.

use std::collections::HashSet;

use crate::governance::error::{GovernanceError, GovernanceResult};
use crate::governance::{PublicKey, Signature};

/// A multisig configuration
#[derive(Debug, Clone)]
pub struct Multisig {
    threshold: usize,
    total: usize,
    public_keys: Vec<PublicKey>,
}

impl Multisig {
    /// Create a new multisig configuration
    pub fn new(
        threshold: usize,
        total: usize,
        public_keys: Vec<PublicKey>,
    ) -> GovernanceResult<Self> {
        if threshold == 0 {
            return Err(GovernanceError::InvalidThreshold { threshold, total });
        }

        if threshold > total {
            return Err(GovernanceError::InvalidThreshold { threshold, total });
        }

        if public_keys.len() != total {
            return Err(GovernanceError::InvalidMultisig(format!(
                "Expected {} public keys, got {}",
                total,
                public_keys.len()
            )));
        }

        // Check for duplicate public keys
        let unique_keys: HashSet<_> = public_keys.iter().collect();
        if unique_keys.len() != public_keys.len() {
            return Err(GovernanceError::InvalidMultisig(
                "Duplicate public keys not allowed".to_string(),
            ));
        }

        Ok(Self {
            threshold,
            total,
            public_keys,
        })
    }

    /// Verify a set of signatures against a message
    pub fn verify(&self, message: &[u8], signatures: &[Signature]) -> GovernanceResult<bool> {
        if signatures.len() < self.threshold {
            return Err(GovernanceError::InsufficientSignatures {
                got: signatures.len(),
                need: self.threshold,
            });
        }

        let valid_signatures = self.collect_valid_signatures(message, signatures)?;
        Ok(valid_signatures.len() >= self.threshold)
    }

    /// Collect valid signatures and return their indices
    pub fn collect_valid_signatures(
        &self,
        message: &[u8],
        signatures: &[Signature],
    ) -> GovernanceResult<Vec<usize>> {
        let mut valid_indices = Vec::new();

        for signature in signatures.iter() {
            // Try to verify against each public key
            for (j, public_key) in self.public_keys.iter().enumerate() {
                if crate::governance::verify_signature(signature, message, public_key)? {
                    valid_indices.push(j);
                    break;
                }
            }
        }

        Ok(valid_indices)
    }

    /// Get the threshold
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// Get the total number of keys
    pub fn total(&self) -> usize {
        self.total
    }

    /// Get the public keys
    pub fn public_keys(&self) -> &[PublicKey] {
        &self.public_keys
    }

    /// Check if a signature is valid for this multisig
    pub fn is_valid_signature(
        &self,
        signature: &Signature,
        message: &[u8],
    ) -> GovernanceResult<Option<usize>> {
        for (i, public_key) in self.public_keys.iter().enumerate() {
            if crate::governance::verify_signature(signature, message, public_key)? {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::GovernanceKeypair;

    #[test]
    fn test_multisig_creation() {
        let keypairs: Vec<_> = (0..5)
            .map(|_| GovernanceKeypair::generate().unwrap())
            .collect();
        let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

        let multisig = Multisig::new(3, 5, public_keys).unwrap();
        assert_eq!(multisig.threshold(), 3);
        assert_eq!(multisig.total(), 5);
    }

    #[test]
    fn test_invalid_threshold() {
        let keypairs: Vec<_> = (0..5)
            .map(|_| GovernanceKeypair::generate().unwrap())
            .collect();
        let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

        // Threshold too high
        let result = Multisig::new(6, 5, public_keys.clone());
        assert!(result.is_err());

        // Threshold zero
        let result = Multisig::new(0, 5, public_keys);
        assert!(result.is_err());
    }

    #[test]
    fn test_multisig_verification() {
        let keypairs: Vec<_> = (0..5)
            .map(|_| GovernanceKeypair::generate().unwrap())
            .collect();
        let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

        let multisig = Multisig::new(3, 5, public_keys).unwrap();
        let message = b"test message";

        // Sign with 3 keys (meets threshold)
        let signatures: Vec<_> = keypairs[0..3]
            .iter()
            .map(|kp| crate::sign_message(&kp.secret_key, message).unwrap())
            .collect();

        let result = multisig.verify(message, &signatures).unwrap();
        assert!(result);
    }

    #[test]
    fn test_insufficient_signatures() {
        let keypairs: Vec<_> = (0..5)
            .map(|_| GovernanceKeypair::generate().unwrap())
            .collect();
        let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

        let multisig = Multisig::new(3, 5, public_keys).unwrap();
        let message = b"test message";

        // Sign with only 2 keys (below threshold)
        let signatures: Vec<_> = keypairs[0..2]
            .iter()
            .map(|kp| crate::sign_message(&kp.secret_key, message).unwrap())
            .collect();

        let result = multisig.verify(message, &signatures);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_public_keys() {
        let keypair = GovernanceKeypair::generate().unwrap();
        let public_key = keypair.public_key();

        // Create multisig with duplicate keys
        let public_keys = vec![public_key.clone(), public_key];

        let result = Multisig::new(2, 2, public_keys);
        assert!(result.is_err());
    }
}
