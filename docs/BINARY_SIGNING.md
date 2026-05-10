# Binary Signing Tools

## Overview

The binary signing tools provide cryptographic verification of release artifacts. They support signing binaries, verification bundles, and checksums with maintainer multisig.

## Tools

### blvm-sign-binary

Signs binaries, verification bundles, and SHA256SUMS files.

**Usage**:
```bash
blvm-sign-binary --key <private-key-file> --output <signature-file> <target>
```

**Targets**:
- `binary --file <path>` - Sign a binary file
- `bundle --file <path>` - Sign a verification bundle
- `checksums --file <path>` - Sign a SHA256SUMS file

**Options**:
- `--key, -k <path>` - Private key file (required)
- `--output, -o <path>` - Output signature file (default: `signature.json`)
- `--format <text|json>` - Output format (default: `text`)
- `--binary-type <type>` - Binary type: `consensus`, `protocol`, `application` (default: `application`)
- `--version <string>` - Version string

**Example**:
```bash
# Sign a binary
blvm-sign-binary \
  --key maintainer-key.pem \
  --output signature.json \
  binary --file target/release/blvm-node \
  --binary-type application \
  --version "0.1.0"

# Sign SHA256SUMS
blvm-sign-binary \
  --key maintainer-key.pem \
  --output checksums.sig \
  checksums --file SHA256SUMS
```

---

### blvm-verify-binary

Verifies binary signatures against public keys.

**Usage**:
```bash
blvm-verify-binary <target> --signature <signature-file>
```

**Targets**:
- `binary --file <path>` - Verify a binary file
- `bundle --file <path>` - Verify a verification bundle
- `checksums --file <path>` - Verify a SHA256SUMS file

**Options**:
- `--signature, -s <path>` - Signature file (required)
- `--public-key, -p <path>` - Public key file (single key verification)
- `--public-keys, -P <path>` - Public keys file (multisig verification)
- `--threshold <n>` - Multisig threshold (default: 1)
- `--format <text|json>` - Output format (default: `text`)

**Example**:
```bash
# Verify binary signature
blvm-verify-binary \
  binary --file blvm-node \
  --signature signature.json \
  --public-keys maintainers.pub \
  --threshold 3

# Verify checksums
blvm-verify-binary \
  checksums --file SHA256SUMS \
  --signature checksums.sig \
  --public-keys maintainers.pub \
  --threshold 3
```

---

### blvm-aggregate-signatures

Aggregates multiple signatures for multisig verification.

**Usage**:
```bash
blvm-aggregate-signatures --signatures <sig1> <sig2> ... --output <aggregated-sig>
```

**Options**:
- `--signatures, -s <path>...` - Signature files to aggregate (required)
- `--output, -o <path>` - Output aggregated signature file (required)
- `--format <text|json>` - Output format (default: `text`)

**Example**:
```bash
# Aggregate 3 maintainer signatures
blvm-aggregate-signatures \
  --signatures sig1.json sig2.json sig3.json \
  --output aggregated.json
```

---

## Multisig Workflows

### Signing Workflow

1. **Each maintainer signs independently**:
   ```bash
   blvm-sign-binary --key maintainer1.pem binary --file blvm-node
   blvm-sign-binary --key maintainer2.pem binary --file blvm-node
   blvm-sign-binary --key maintainer3.pem binary --file blvm-node
   ```

2. **Aggregate signatures**:
   ```bash
   blvm-aggregate-signatures \
     --signatures sig1.json sig2.json sig3.json \
     --output aggregated.json
   ```

3. **Verify aggregated signature**:
   ```bash
   blvm-verify-binary \
     binary --file blvm-node \
     --signature aggregated.json \
     --public-keys maintainers.pub \
     --threshold 3
   ```

### Verification Workflow

1. **Download binary and signature**:
   ```bash
   wget https://releases.btcdecoded.org/blvm-node
   wget https://releases.btcdecoded.org/blvm-node.sig
   ```

2. **Verify signature**:
   ```bash
   blvm-verify-binary \
     binary --file blvm-node \
     --signature blvm-node.sig \
     --public-keys maintainers.pub \
     --threshold 3
   ```

---

## Signature Format

Signatures use JSON format:

```json
{
  "target_type": "binary",
  "target_hash": "sha256-hex-hash",
  "signer": "maintainer-pubkey-hex",
  "signature": "secp256k1-signature-hex",
  "timestamp": 1234567890,
  "metadata": {
    "binary_type": "application",
    "version": "0.1.0"
  }
}
```

Multisig signatures include multiple signers:

```json
{
  "target_type": "binary",
  "target_hash": "sha256-hex-hash",
  "signatures": [
    {
      "signer": "pubkey1-hex",
      "signature": "sig1-hex",
      "timestamp": 1234567890
    },
    {
      "signer": "pubkey2-hex",
      "signature": "sig2-hex",
      "timestamp": 1234567891
    }
  ],
  "threshold": 3,
  "metadata": {
    "binary_type": "application",
    "version": "0.1.0"
  }
}
```

---

## Security Considerations

### Key Management

- Private keys must be stored securely (HSM in production)
- Never commit private keys to version control
- Use separate keys for signing vs. governance operations
- Rotate keys regularly

### Signature Verification

- Always verify signatures before executing binaries
- Use threshold multisig for production releases
- Verify public keys against authoritative source
- Check signature timestamps for freshness

### Hash Verification

- Signatures verify SHA256 hashes of targets
- Binary hashes are computed from file contents
- Checksum files are verified before use
- Hash mismatches indicate tampering

---

## Related Documentation

- [API Reference](api-reference.md) - SDK API documentation
- [Governance Crypto Library](governance-crypto-library.md) - Cryptographic primitives
- Component-specific signing workflows in release documentation
