//! # Developer SDK
//!
//! Governance infrastructure and composition framework for Bitcoin.
//!
//! This crate provides the **institutional layer** for Bitcoin governance, offering
//! reusable governance primitives and a composition framework for building alternative
//! Bitcoin implementations.
//!
//! ## Architecture Position
//!
//! This is **Tier 5** of the 5-tier BTCDecoded architecture:
//!
//! <!--
//! bllvm-spec (Orange Paper) -> bllvm-consensus -> bllvm-protocol -> bllvm-node -> bllvm-sdk
//! -->
//!
//! ## Core Components
//!
//! ### Governance Primitives
//! - **Cryptographic key management** for governance operations
//! - **Signature creation and verification** using Bitcoin-compatible standards
//! - **Multisig threshold logic** for collective decision making
//! - **Message formats** for releases, module approvals, and budget decisions
//!
//! ### CLI Tools
//! - `bllvm-keygen` - Generate governance keypairs
//! - `bllvm-sign` - Sign governance messages
//! - `bllvm-verify` - Verify signatures and multisig thresholds
//!
//! ## Quick Start
//!
//! ```rust
//! use bllvm_sdk::{
//!     GovernanceKeypair, GovernanceMessage, Multisig, sign_message
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Generate a keypair
//! let keypair = GovernanceKeypair::generate()?;
//!
//! // Create a message to sign
//! let message = GovernanceMessage::Release {
//!     version: "v1.0.0".to_string(),
//!     commit_hash: "abc123".to_string(),
//! };
//!
//! // Sign the message
//! let signature = sign_message(&keypair.secret_key, &message.to_signing_bytes())?;
//!
//! // Verify with multisig (example with 1-of-1)
//! let maintainer_keys = vec![keypair.public_key()];
//! let multisig = Multisig::new(1, 1, maintainer_keys)?;
//! let valid = multisig.verify(&message.to_signing_bytes(), &[signature])?;
//! assert!(valid);
//! # Ok(())
//! # }
//! ```

pub mod cli;
pub mod governance;
pub mod composition;

// Re-export main types for convenience
pub use governance::{
    GovernanceError, GovernanceKeypair, GovernanceMessage, GovernanceResult, Multisig, PublicKey,
    Signature,
};

// Re-export governance functions
pub use governance::signatures::{sign_message, verify_signature};

// Re-export composition framework
pub use composition::{
    ModuleRegistry, ModuleLifecycle, NodeComposer, NodeConfig,
    ModuleInfo, ModuleSource, ModuleStatus, ModuleHealth,
    NodeSpec, ModuleSpec, ComposedNode, NetworkType,
};
