# Bitcoin Commons Developer SDK

[![crates.io](https://img.shields.io/crates/v/blvm-sdk.svg)](https://crates.io/crates/blvm-sdk)
[![docs.rs](https://docs.rs/blvm-sdk/badge.svg)](https://docs.rs/blvm-sdk)
[![CI](https://github.com/BTCDecoded/blvm-sdk/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/BTCDecoded/blvm-sdk/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

Governance infrastructure and composition framework for Bitcoin implementations.

> **For verified system status**: See [SYSTEM_STATUS.md](https://github.com/BTCDecoded/.github/blob/main/SYSTEM_STATUS.md) in the BTCDecoded organization repository.

Provides the institutional layer for Bitcoin governance, offering reusable governance primitives and a composition framework for building alternative Bitcoin implementations.

## Architecture Position

Tier 5 of the 6-tier Bitcoin Commons architecture (BLVM technology stack):

```
1. blvm-spec (Orange Paper - mathematical foundation)
2. blvm-consensus (pure math implementation)
3. blvm-protocol (Bitcoin abstraction)
4. blvm-node (full node implementation)
5. blvm-sdk (composition + governance libraries)
6. blvm-commons (governance enforcement)
```

## Core Components

### Governance Primitives
- Cryptographic key management for governance operations
- Signature creation and verification using Bitcoin-compatible standards
- Multisig threshold logic for collective decision making
- Nested multisig support for team-based governance
- Message formats for releases, module approvals, and budget decisions

### CLI Tools
- `blvm-keygen` - Generate governance keypairs
- `blvm-sign` - Sign governance messages
- `blvm-verify` - Verify signatures and multisig thresholds
- `blvm-compose` - Declarative node composition from modules
- `blvm-sign-binary` - Sign binary files
- `blvm-verify-binary` - Verify binary file signatures
- `blvm-aggregate-signatures` - Aggregate multiple signatures

### Composition Framework
- Declarative node composition from modules
- Module registry and lifecycle management
- Economic integration through merge mining

## Quick Start

### As a Library

```rust
use blvm_sdk::governance::{
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
blvm-keygen --output alice.key --format pem

# Sign a release
blvm-sign release \
  --version v1.0.0 \
  --commit abc123 \
  --key alice.key \
  --output signature.txt

# Verify signatures
blvm-verify release \
  --version v1.0.0 \
  --commit abc123 \
  --signatures sig1.txt,sig2.txt,sig3.txt,sig4.txt,sig5.txt,sig6.txt \
  --threshold 6-of-7 \
  --pubkeys keys.json
```

## Features

- **Governance Primitives**: Cryptographic key management and signature verification
- **CLI Tools**: `blvm-keygen`, `blvm-sign`, `blvm-verify`, `blvm-compose`, `blvm-sign-binary`, `blvm-verify-binary`, `blvm-aggregate-signatures`
- **Multisig Support**: Threshold logic for collective decision making
- **Bitcoin-Compatible**: Uses Bitcoin message signing standards
- **Composition Framework**: Declarative node composition from modules

## Design Principles

1. **Governance Crypto is Reusable:** Clean library API for external consumers
2. **No GitHub Logic:** SDK is pure cryptography + composition, not enforcement
3. **Bitcoin-Compatible:** Use Bitcoin message signing standards
4. **Test Everything:** Governance crypto needs 100% test coverage
5. **Document for Consumers:** Teams building integrations on top of governance crypto and composition are the audience for this SDK.

## What This Is NOT

- NOT a general-purpose Bitcoin library
- NOT the GitHub enforcement engine (that is **[`blvm-commons`](https://github.com/BTCDecoded/blvm-commons)**)
- NOT handling wallet keys or user funds
- NOT competing with rust-bitcoin or BDK

## Security

See [SECURITY.md](SECURITY.md) for security policies and [BTCDecoded Security Policy](https://github.com/BTCDecoded/.github/blob/main/SECURITY.md) for organization-wide guidelines.

## crates.io

The published crate is [`blvm-sdk` on crates.io](https://crates.io/crates/blvm-sdk). In the monorepo, `Cargo.toml` uses a **path** to `blvm-node` and `blvm-sdk-macros`. For published versions only:

```toml
[dependencies]
blvm-sdk = ">=0.1, <1"
```

Use `[patch.crates-io]` in `.cargo/config.toml` if you need to override with local sibling paths.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the [BTCDecoded Contribution Guide](https://github.com/BTCDecoded/.github/blob/main/CONTRIBUTING.md).

## License

MIT License - see LICENSE file for details.




