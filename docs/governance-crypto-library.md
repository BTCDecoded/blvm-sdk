# Governance Crypto Library

This document provides a comprehensive guide for developers using the governance crypto library in the blvm-sdk.

## Overview

The governance crypto library provides cryptographic primitives for Bitcoin governance operations. It's designed to be imported by external applications like governance-app and blvm-node.

## Core Components

### Key Management

The `GovernanceKeypair` struct provides key generation and management:

```rust
use blvm_sdk::governance::GovernanceKeypair;

// Generate a random keypair
let keypair = GovernanceKeypair::generate()?;

// Create from existing secret key
let secret_bytes = [1u8; 32];
let keypair = GovernanceKeypair::from_secret_key(&secret_bytes)?;

// Get public key
let public_key = keypair.public_key();

// Get key bytes
let secret_bytes = keypair.secret_key_bytes();
let public_bytes = keypair.public_key().to_bytes();
```

### Message Signing

The `GovernanceMessage` enum provides standardized message formats:

```rust
use blvm_sdk::governance::GovernanceMessage;

// Create a release message
let message = GovernanceMessage::Release {
    version: "v1.0.0".to_string(),
    commit_hash: "abc123".to_string(),
};

// Create a module approval message
let message = GovernanceMessage::ModuleApproval {
    module_name: "lightning".to_string(),
    version: "v2.0.0".to_string(),
};

// Create a budget decision message
let message = GovernanceMessage::BudgetDecision {
    amount: 1000000,
    purpose: "development".to_string(),
};

// Get signing bytes
let signing_bytes = message.to_signing_bytes();
```

### Signature Operations

Sign and verify messages:

```rust
use blvm_sdk::governance::{sign_message, verify_signature};

// Sign a message
let signature = sign_message(&keypair.secret_key, &message.to_signing_bytes())?;

// Verify a signature
let verified = verify_signature(&signature, &message.to_signing_bytes(), &keypair.public_key())?;
```

### Multisig Operations

The `Multisig` struct provides threshold signature validation:

```rust
use blvm_sdk::governance::Multisig;

// Create a 3-of-5 multisig
let keypairs: Vec<_> = (0..5).map(|_| GovernanceKeypair::generate().unwrap()).collect();
let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

let multisig = Multisig::new(3, 5, public_keys)?;

// Sign with 3 keys
let signatures: Vec<_> = keypairs[0..3]
    .iter()
    .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
    .collect();

// Verify multisig
let verified = multisig.verify(&message.to_signing_bytes(), &signatures)?;
```

## Integration Examples

### For Governance-App

```rust
use blvm_sdk::governance::{
    GovernanceKeypair, GovernanceMessage, Multisig, sign_message, verify_signature
};

// In your webhook handler
async fn handle_release_webhook(pr: PullRequest) -> Result<(), Error> {
    // Create release message
    let message = GovernanceMessage::Release {
        version: pr.title.clone(),
        commit_hash: pr.head.sha.clone(),
    };
    
    // Extract signatures from PR comments
    let signatures = extract_signatures_from_comments(&pr.comments).await?;
    
    // Load maintainer keys
    let maintainer_keys = load_maintainer_keys().await?;
    
    // Create multisig (6-of-7)
    let multisig = Multisig::new(6, 7, maintainer_keys)?;
    
    // Verify signatures
    let verified = multisig.verify(&message.to_signing_bytes(), &signatures)?;
    
    if verified {
        // Merge the PR
        merge_pull_request(&pr).await?;
    }
    
    Ok(())
}
```

### For Reference-Node

```rust
use blvm_sdk::governance::{
    GovernanceMessage, Multisig, verify_signature
};

// In your node's governance module
pub struct GovernanceModule {
    multisig: Multisig,
    maintainer_keys: Vec<PublicKey>,
}

impl GovernanceModule {
    pub fn verify_release(&self, release: &Release) -> Result<bool, Error> {
        let message = GovernanceMessage::Release {
            version: release.version.clone(),
            commit_hash: release.commit_hash.clone(),
        };
        
        // Verify against maintainer signatures
        let verified = self.multisig.verify(
            &message.to_signing_bytes(),
            &release.signatures,
        )?;
        
        Ok(verified)
    }
}
```

## Security Considerations

### Key Management
- Store private keys securely (not in the library)
- Use hardware security modules for production
- Implement proper key rotation policies

### Signature Verification
- Always verify signatures before trusting messages
- Use multisig for critical operations
- Implement proper threshold validation

### Message Validation
- Validate message content before signing
- Use standardized message formats
- Implement replay attack prevention

## Error Handling

The library uses the `GovernanceResult<T>` type for error handling:

```rust
use blvm_sdk::governance::{GovernanceResult, GovernanceError};

fn process_governance_message() -> GovernanceResult<()> {
    let keypair = GovernanceKeypair::generate()?;
    let message = GovernanceMessage::Release {
        version: "v1.0.0".to_string(),
        commit_hash: "abc123".to_string(),
    };
    
    let signature = sign_message(&keypair.secret_key, &message.to_signing_bytes())?;
    let verified = verify_signature(&signature, &message.to_signing_bytes(), &keypair.public_key())?;
    
    if !verified {
        return Err(GovernanceError::SignatureVerification("Invalid signature".to_string()));
    }
    
    Ok(())
}
```

## Testing

The library includes comprehensive tests for all operations:

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test governance_crypto_tests
cargo test key_generation_tests
cargo test signature_tests
cargo test multisig_tests
cargo test message_format_tests
```

## Performance Considerations

- Key generation is fast but should be done sparingly
- Signature verification is optimized for batch operations
- Multisig verification scales linearly with the number of signatures
- Message serialization is designed for efficiency

## Best Practices

1. **Use multisig for critical operations** - Never rely on single signatures
2. **Validate message content** - Check message fields before signing
3. **Implement proper error handling** - Use the provided error types
4. **Test thoroughly** - Use the comprehensive test suite
5. **Document your usage** - Keep track of governance operations




