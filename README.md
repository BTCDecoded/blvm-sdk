# Bitcoin Commons Developer SDK

**Governance infrastructure and composition framework for Bitcoin.**

This crate provides the **institutional layer** for Bitcoin governance, offering reusable governance primitives and a composition framework for building alternative Bitcoin implementations.

## Architecture Position

This is **Tier 5** of the 5-tier Bitcoin Commons architecture (BLLVM technology stack):

```
1. Orange Paper (mathematical foundation)
2. consensus-proof (pure math implementation)
3. protocol-engine (Bitcoin abstraction)
4. reference-node (full node implementation)
5. developer-sdk (composition + governance libraries) ← THIS CRATE
   ↓ imports governance primitives
governance-app (GitHub enforcement) ← FUTURE, separate repo
   ↓
Module Ecosystem (separate repos: Lightning, RSK, etc.)
   ↓
User-Composed Bitcoin Stacks
```

## Core Components

### Governance Primitives
- **Cryptographic key management** for governance operations
- **Signature creation and verification** using Bitcoin-compatible standards
- **Multisig threshold logic** for collective decision making
- **Message formats** for releases, module approvals, and budget decisions

### CLI Tools
- `bllvm-keygen` - Generate governance keypairs
- `bllvm-sign` - Sign governance messages
- `bllvm-verify` - Verify signatures and multisig thresholds

### Composition Framework (Future)
- **Declarative node composition** from modules
- **Module registry** and lifecycle management
- **Economic integration** through merge mining

## Quick Start

### As a Library

```rust
use developer_sdk::governance::{
    GovernanceKeypair, GovernanceMessage, Multisig
};

// Generate a keypair
let keypair = GovernanceKeypair::generate()?;

// Create a message to sign
let message = GovernanceMessage::Release {
    version: "v1.0.0".to_string(),
    commit_hash: "abc123".to_string(),
};

// Sign the message
let signature = keypair.sign(&message.to_signing_bytes())?;

// Verify with multisig
let multisig = Multisig::new(6, 7, maintainer_keys)?;
let valid = multisig.verify(&message.to_signing_bytes(), &[signature])?;
```

### CLI Usage

```bash
# Generate a keypair
bllvm-keygen --output alice.key --format pem

# Sign a release
bllvm-sign release \
  --version v1.0.0 \
  --commit abc123 \
  --key alice.key \
  --output signature.txt

# Verify signatures
bllvm-verify release \
  --version v1.0.0 \
  --commit abc123 \
  --signatures sig1.txt,sig2.txt,sig3.txt,sig4.txt,sig5.txt,sig6.txt \
  --threshold 6-of-7 \
  --pubkeys keys.json
```

## Design Principles

1. **Governance Crypto is Reusable:** Clean library API for external consumers
2. **No GitHub Logic:** SDK is pure cryptography + composition, not enforcement
3. **Bitcoin-Compatible:** Use Bitcoin message signing standards
4. **Test Everything:** Governance crypto needs 100% test coverage
5. **Document for Consumers:** governance-app developers are the customer

## What This Is NOT

- NOT a general-purpose Bitcoin library
- NOT the GitHub enforcement engine (that's governance-app)
- NOT handling wallet keys or user funds
- NOT competing with rust-bitcoin or BDK

## Security

See [SECURITY.md](SECURITY.md) for security policies and [BTCDecoded Security Policy](https://github.com/BTCDecoded/.github/blob/main/SECURITY.md) for organization-wide guidelines.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the [BTCDecoded Contribution Guide](https://github.com/BTCDecoded/.github/blob/main/CONTRIBUTING.md).

## License

MIT License - see LICENSE file for details.




