//! # Governance Messages
//!
//! Message formats for governance operations.

use serde::{Deserialize, Serialize};
use std::fmt;

// No error types needed for this module

/// A governance message that can be signed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernanceMessage {
    /// A release message
    Release {
        version: String,
        commit_hash: String,
    },
    /// A module approval message
    ModuleApproval {
        module_name: String,
        version: String,
    },
    /// A budget decision message
    BudgetDecision { amount: u64, purpose: String },
}

impl GovernanceMessage {
    /// Convert the message to bytes for signing
    pub fn to_signing_bytes(&self) -> Vec<u8> {
        // Use a standardized format for signing
        match self {
            GovernanceMessage::Release {
                version,
                commit_hash,
            } => format!("RELEASE:{version}:{commit_hash}").into_bytes(),
            GovernanceMessage::ModuleApproval {
                module_name,
                version,
            } => format!("MODULE:{module_name}:{version}").into_bytes(),
            GovernanceMessage::BudgetDecision { amount, purpose } => {
                format!("BUDGET:{amount}:{purpose}").into_bytes()
            }
        }
    }

    /// Get a human-readable description of the message
    pub fn description(&self) -> String {
        match self {
            GovernanceMessage::Release {
                version,
                commit_hash,
            } => {
                format!("Release {version} (commit: {commit_hash})")
            }
            GovernanceMessage::ModuleApproval {
                module_name,
                version,
            } => {
                format!("Approve module {module_name} version {version}")
            }
            GovernanceMessage::BudgetDecision { amount, purpose } => {
                format!("Budget decision: {amount} satoshis for {purpose}")
            }
        }
    }
}

impl fmt::Display for GovernanceMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_release_message() {
        let message = GovernanceMessage::Release {
            version: "v1.0.0".to_string(),
            commit_hash: "abc123".to_string(),
        };

        let bytes = message.to_signing_bytes();
        assert_eq!(bytes, b"RELEASE:v1.0.0:abc123");
        assert_eq!(message.description(), "Release v1.0.0 (commit: abc123)");
    }

    #[test]
    fn test_module_approval_message() {
        let message = GovernanceMessage::ModuleApproval {
            module_name: "lightning".to_string(),
            version: "v2.0.0".to_string(),
        };

        let bytes = message.to_signing_bytes();
        assert_eq!(bytes, b"MODULE:lightning:v2.0.0");
        assert_eq!(
            message.description(),
            "Approve module lightning version v2.0.0"
        );
    }

    #[test]
    fn test_budget_decision_message() {
        let message = GovernanceMessage::BudgetDecision {
            amount: 1000000,
            purpose: "development".to_string(),
        };

        let bytes = message.to_signing_bytes();
        assert_eq!(bytes, b"BUDGET:1000000:development");
        assert_eq!(
            message.description(),
            "Budget decision: 1000000 satoshis for development"
        );
    }

    #[test]
    fn test_message_serialization() {
        let message = GovernanceMessage::Release {
            version: "v1.0.0".to_string(),
            commit_hash: "abc123".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: GovernanceMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, deserialized);
    }
}
