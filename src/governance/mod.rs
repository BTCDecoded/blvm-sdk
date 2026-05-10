//! # Governance Module
//!
//! Cryptographic primitives for Bitcoin governance operations.
//!
//! This module provides the core governance functionality:
//! - Key generation and management
//! - Signature creation and verification
//! - Multisig threshold logic
//! - Message formats for governance decisions

pub mod bip32;
pub mod bip39;
pub mod bip44;
pub mod error;
pub mod keys;
pub mod messages;
pub mod multisig;
pub mod nested_multisig;
pub mod psbt;
pub mod signatures;
pub mod verification;

// Re-export main types
pub use error::{GovernanceError, GovernanceResult};
pub use keys::{GovernanceKeypair, PublicKey};
pub use messages::GovernanceMessage;
pub use multisig::Multisig;
pub use signatures::Signature;
pub use verification::verify_signature;
