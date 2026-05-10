//! # Governance Error Types
//!
//! Error handling for governance operations.

use thiserror::Error;

/// Result type for governance operations
pub type GovernanceResult<T> = Result<T, GovernanceError>;

/// Errors that can occur during governance operations
#[derive(Error, Debug)]
pub enum GovernanceError {
    /// Invalid key format or key generation failure
    #[error("Invalid key: {0}")]
    InvalidKey(String),

    /// Signature verification failed
    #[error("Signature verification failed: {0}")]
    SignatureVerification(String),

    /// Invalid multisig configuration
    #[error("Invalid multisig configuration: {0}")]
    InvalidMultisig(String),

    /// Message format error
    #[error("Message format error: {0}")]
    MessageFormat(String),

    /// Cryptographic operation failed
    #[error("Cryptographic operation failed: {0}")]
    Cryptographic(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Invalid threshold configuration
    #[error("Invalid threshold: {threshold} of {total}")]
    InvalidThreshold { threshold: usize, total: usize },

    /// Insufficient signatures for multisig
    #[error("Insufficient signatures: got {got}, need {need}")]
    InsufficientSignatures { got: usize, need: usize },

    /// Invalid signature format
    #[error("Invalid signature format: {0}")]
    InvalidSignatureFormat(String),

    /// Invalid input data
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Feature not yet implemented
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}
