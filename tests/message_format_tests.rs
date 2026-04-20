//! # Message Format Tests
//!
//! Tests for message serialization and format consistency.

use blvm_sdk::governance::GovernanceMessage;

#[test]
fn test_release_message_format() {
    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123def456".to_string(),
    };

    let signing_bytes = message.to_signing_bytes();
    assert_eq!(signing_bytes, b"RELEASE:v1.0.0:abc123def456");

    let description = message.description();
    assert_eq!(description, "Release v1.0.0 (commit: abc123def456)");
}

#[test]
fn test_module_approval_message_format() {
    let message = GovernanceMessage::ModuleApproval {
        module_name: "lightning-network".to_string(),
        version: "v2.0.0".to_string(),
    };

    let signing_bytes = message.to_signing_bytes();
    assert_eq!(signing_bytes, b"MODULE:lightning-network:v2.0.0");

    let description = message.description();
    assert_eq!(
        description,
        "Approve module lightning-network version v2.0.0"
    );
}

#[test]
fn test_budget_decision_message_format() {
    let message = GovernanceMessage::BudgetDecision {
        amount: 1000000,
        purpose: "development and maintenance".to_string(),
    };

    let signing_bytes = message.to_signing_bytes();
    assert_eq!(signing_bytes, b"BUDGET:1000000:development and maintenance");

    let description = message.description();
    assert_eq!(
        description,
        "Budget decision: 1000000 satoshis for development and maintenance"
    );
}

#[test]
fn test_message_serialization_roundtrip() {
    let messages = vec![
        GovernanceMessage::Release {
            version: "v1.0.0".to_string(),
            commit_hash: "abc123".to_string(),
        },
        GovernanceMessage::ModuleApproval {
            module_name: "lightning".to_string(),
            version: "v2.0.0".to_string(),
        },
        GovernanceMessage::BudgetDecision {
            amount: 1000000,
            purpose: "development".to_string(),
        },
    ];

    for message in messages {
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: GovernanceMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(message, deserialized);
    }
}

#[test]
fn test_message_signing_bytes_consistency() {
    let message1 = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    let message2 = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    // Same message should produce same signing bytes
    assert_eq!(message1.to_signing_bytes(), message2.to_signing_bytes());
}

#[test]
fn test_message_signing_bytes_different() {
    let message1 = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    let message2 = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "def456".to_string(),
    };

    // Different messages should produce different signing bytes
    assert_ne!(message1.to_signing_bytes(), message2.to_signing_bytes());
}

#[test]
fn test_message_display_format() {
    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    let display_str = format!("{}", message);
    assert_eq!(display_str, "Release v1.0.0 (commit: abc123)");
}

#[test]
fn test_message_equality() {
    let message1 = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    let message2 = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };

    let message3 = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "def456".to_string(),
    };

    // Same messages should be equal
    assert_eq!(message1, message2);

    // Different messages should not be equal
    assert_ne!(message1, message3);
}

#[test]
fn test_message_special_characters() {
    let message = GovernanceMessage::BudgetDecision {
        amount: 1000000,
        purpose: "development & maintenance (2024)".to_string(),
    };

    let signing_bytes = message.to_signing_bytes();
    let expected = b"BUDGET:1000000:development & maintenance (2024)";

    assert_eq!(signing_bytes, expected);
}

#[test]
fn test_message_empty_fields() {
    let message = GovernanceMessage::Release {
        version: "".to_string(),
        commit_hash: "".to_string(),
    };

    let signing_bytes = message.to_signing_bytes();
    assert_eq!(signing_bytes, b"RELEASE::");

    let description = message.description();
    assert_eq!(description, "Release  (commit: )");
}

#[test]
fn test_message_unicode_support() {
    let message = GovernanceMessage::BudgetDecision {
        amount: 1000000,
        purpose: "开发与维护".to_string(), // Chinese characters
    };

    let signing_bytes = message.to_signing_bytes();
    let expected = b"BUDGET:1000000:\xE5\xBC\x80\xE5\x8F\x91\xE4\xB8\x8E\xE7\xBB\xB4\xE6\x8A\xA4";

    assert_eq!(signing_bytes, expected);
}
