# Contributing to blvm-sdk

Thank you for your interest in contributing to blvm-sdk! This document contains **repo-specific guidelines only**. For comprehensive contributing guidelines, see the [BLVM Documentation](https://docs.thebitcoincommons.org/development/contributing.html).

## Quick Links

- **[Complete Contributing Guide](https://docs.thebitcoincommons.org/development/contributing.html)** - Full developer workflow
- **[PR Process](https://docs.thebitcoincommons.org/development/pr-process.html)** - Governance tiers and review process
- **[SDK Documentation](https://docs.thebitcoincommons.org/sdk/overview.html)** - SDK guides

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). By participating, you agree to uphold this code.

## Repository-Specific Guidelines

### Development Setup

```bash
# Clone the repository
git clone https://github.com/BTCDecoded/blvm-sdk.git
cd blvm-sdk

# Run tests
cargo test

# Run benchmarks
cargo bench

# Check formatting
cargo fmt --check

# Check clippy
cargo clippy --all-targets --all-features
```

### Contribution Areas

#### High Priority
- **Governance crypto primitives**: Key generation, signing, verification
- **Multisig operations**: Threshold validation and signature collection
- **CLI tools**: Command-line interfaces for maintainer operations
- **Testing**: Comprehensive test coverage for all crypto operations

#### Future Areas
- **Node composition**: Declarative node building from modules
- **Module interfaces**: Standardized module trait definitions
- **Economic integration**: Merge mining revenue model

### Code Standards

#### Governance Crypto
- All cryptographic operations must have 100% test coverage
- Use Bitcoin-compatible standards for message signing
- Pin all dependencies to exact versions
- Document security boundaries clearly

#### CLI Tools
- Follow standard Unix conventions for command-line interfaces
- Provide clear error messages and usage examples
- Support both JSON and human-readable output formats

#### Testing
- Unit tests for all public APIs
- Integration tests for complete workflows
- Property-based testing for cryptographic operations
- Benchmark tests for performance-critical paths

### Security Considerations

- Never commit private keys or sensitive data
- All cryptographic changes require security review
- Follow the security boundaries defined in SECURITY.md
- Report security issues through the proper channels

## Getting Help

- **Documentation**: [docs.thebitcoincommons.org](https://docs.thebitcoincommons.org)
- **Issues**: Use GitHub issues for bugs and feature requests
- **Discussions**: Use GitHub discussions for questions
- **Security**: See [SECURITY.md](SECURITY.md)

Thank you for contributing to blvm-sdk!
