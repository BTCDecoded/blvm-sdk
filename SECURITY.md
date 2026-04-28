# Security Policy

This document covers repo-specific security boundaries. See the [BTCDecoded Security Policy](https://github.com/BTCDecoded/.github/blob/main/SECURITY.md) for organization-wide policy.

## Security Boundaries

### What This Crate Handles
- **Governance cryptography**: Key generation, signing, verification
- **Multisig operations**: Threshold validation and signature collection
- **Message formats**: Standardized formats for governance decisions
- **CLI tools**: Command-line interfaces for maintainer operations

### What This Crate Does NOT Handle
- **User funds**: No wallet functionality or private key storage
- **Network enforcement**: No GitHub webhook handling or enforcement logic
- **Consensus validation**: No Bitcoin consensus rule implementation
- **User authentication**: No user-facing authentication or authorization

## Threat Model

### Primary Threats
1. **Cryptographic vulnerabilities**: Weak key generation or signature verification
2. **Message tampering**: Unauthorized modification of governance messages
3. **Multisig bypass**: Incorrect threshold validation allowing unauthorized actions
4. **Key compromise**: Exposure of governance private keys

### Mitigation Strategies
1. **Exact dependency pinning**: All crypto dependencies pinned to exact versions
2. **Comprehensive testing**: 100% test coverage for all crypto operations
3. **Bitcoin compatibility**: Use established Bitcoin cryptographic standards
4. **Clear documentation**: Explicit security boundaries and usage guidelines

## Security Considerations

### Key Management
- Keys are generated using cryptographically secure random number generation
- Private keys should be stored securely by the application using this library
- This library does not provide key storage or persistence mechanisms

### Signature Verification
- All signatures are verified using Bitcoin-compatible standards
- Message formats are designed to prevent replay attacks
- Multisig thresholds are strictly enforced

### Dependencies
- All cryptographic dependencies are pinned to exact versions
- Regular security audits of dependency versions
- Minimal dependency surface area

## Reporting Security Issues

Please report security issues to security@thebitcoincommons.org or through the [BTCDecoded Security Policy](https://github.com/BTCDecoded/.github/blob/main/SECURITY.md).

## Security Updates

Security updates will be released as patch versions (0.1.x) with clear security advisories.
