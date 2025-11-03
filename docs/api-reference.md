# API Reference

Complete API documentation for the developer-sdk governance crypto library.

## Core Types

### GovernanceKeypair

A governance keypair for signing governance messages.

```rust
pub struct GovernanceKeypair {
    // Private fields
}
```

#### Methods

- `generate() -> GovernanceResult<Self>` - Generate a new random keypair
- `from_secret_key(secret_bytes: &[u8]) -> GovernanceResult<Self>` - Create from secret key
- `public_key(&self) -> PublicKey` - Get the public key
- `secret_key_bytes(&self) -> [u8; 32]` - Get the secret key bytes
- `public_key_bytes(&self) -> [u8; 33]` - Get the public key bytes

### PublicKey

A public key for governance operations.

```rust
pub struct PublicKey {
    // Private fields
}
```

#### Methods

- `from_bytes(bytes: &[u8]) -> GovernanceResult<Self>` - Create from bytes
- `to_bytes(&self) -> [u8; 33]` - Get compressed public key bytes
- `to_compressed_bytes(&self) -> [u8; 33]` - Get compressed public key bytes
- `to_uncompressed_bytes(&self) -> [u8; 65]` - Get uncompressed public key bytes

### Signature

A governance signature.

```rust
pub struct Signature {
    // Private fields
}
```

#### Methods

- `from_bytes(bytes: &[u8]) -> GovernanceResult<Self>` - Create from bytes
- `to_bytes(&self) -> [u8; 64]` - Get signature bytes
- `to_der_bytes(&self) -> Vec<u8>` - Get signature in DER format

### GovernanceMessage

A governance message that can be signed.

```rust
pub enum GovernanceMessage {
    Release {
        version: String,
        commit_hash: String,
    },
    ModuleApproval {
        module_name: String,
        version: String,
    },
    BudgetDecision {
        amount: u64,
        purpose: String,
    },
}
```

#### Methods

- `to_signing_bytes(&self) -> Vec<u8>` - Convert to bytes for signing
- `description(&self) -> String` - Get human-readable description

### Multisig

A multisig configuration for threshold signatures.

```rust
pub struct Multisig {
    // Private fields
}
```

#### Methods

- `new(threshold: usize, total: usize, public_keys: Vec<PublicKey>) -> GovernanceResult<Self>` - Create new multisig
- `verify(&self, message: &[u8], signatures: &[Signature]) -> GovernanceResult<bool>` - Verify signatures
- `collect_valid_signatures(&self, message: &[u8], signatures: &[Signature]) -> GovernanceResult<Vec<usize>>` - Collect valid signatures
- `threshold(&self) -> usize` - Get threshold
- `total(&self) -> usize` - Get total number of keys
- `public_keys(&self) -> &[PublicKey]` - Get public keys
- `is_valid_signature(&self, signature: &Signature, message: &[u8]) -> GovernanceResult<Option<usize>>` - Check if signature is valid

## Functions

### sign_message

Sign a message with a secret key.

```rust
pub fn sign_message(secret_key: &SecretKey, message: &[u8]) -> GovernanceResult<Signature>
```

**Parameters:**
- `secret_key` - The secret key to sign with
- `message` - The message to sign

**Returns:**
- `GovernanceResult<Signature>` - The signature or an error

### verify_signature

Verify a signature against a message and public key.

```rust
pub fn verify_signature(
    signature: &Signature,
    message: &[u8],
    public_key: &PublicKey,
) -> GovernanceResult<bool>
```

**Parameters:**
- `signature` - The signature to verify
- `message` - The message that was signed
- `public_key` - The public key to verify against

**Returns:**
- `GovernanceResult<bool>` - True if signature is valid, false otherwise

## Error Types

### GovernanceError

Errors that can occur during governance operations.

```rust
pub enum GovernanceError {
    InvalidKey(String),
    SignatureVerification(String),
    InvalidMultisig(String),
    MessageFormat(String),
    Cryptographic(String),
    Serialization(String),
    InvalidThreshold { threshold: usize, total: usize },
    InsufficientSignatures { got: usize, need: usize },
    InvalidSignatureFormat(String),
}
```

### GovernanceResult

Result type for governance operations.

```rust
pub type GovernanceResult<T> = Result<T, GovernanceError>;
```

## CLI Tools

### bllvm-keygen

Generate governance keypairs.

```bash
bllvm-keygen [OPTIONS]

Options:
    -o, --output <OUTPUT>    Output file for the keypair [default: governance.key]
    -f, --format <FORMAT>    Output format (text, json) [default: text]
    --seed <SEED>            Generate deterministic keypair from seed
    --show-private          Show private key in output
```

### bllvm-sign

Sign governance messages.

```bash
bllvm-sign [OPTIONS] <COMMAND>

Options:
    -o, --output <OUTPUT>    Output file for the signature [default: signature.txt]
    -f, --format <FORMAT>    Output format (text, json) [default: text]
    -k, --key <KEY>          Private key file

Commands:
    release                 Sign a release message
    module                  Sign a module approval message
    budget                  Sign a budget decision message
```

### bllvm-verify

Verify governance signatures.

```bash
bllvm-verify [OPTIONS] <COMMAND>

Options:
    -f, --format <FORMAT>    Output format (text, json) [default: text]
    -s, --signatures <SIGNATURES>    Signature files (comma-separated)
    --threshold <THRESHOLD>          Threshold (e.g., "3-of-5")
    --pubkeys <PUBKEYS>              Public key files (comma-separated)

Commands:
    release                 Verify a release message
    module                  Verify a module approval message
    budget                  Verify a budget decision message
```

## Examples

### Basic Usage

```rust
use developer_sdk::governance::{
    GovernanceKeypair, GovernanceMessage, Multisig, sign_message, verify_signature
};

// Generate keypair
let keypair = GovernanceKeypair::generate()?;

// Create message
let message = GovernanceMessage::Release {
    version: "v1.0.0".to_string(),
    commit_hash: "abc123".to_string(),
};

// Sign message
let signature = sign_message(&keypair.secret_key, &message.to_signing_bytes())?;

// Verify signature
let verified = verify_signature(&signature, &message.to_signing_bytes(), &keypair.public_key())?;
assert!(verified);
```

### Multisig Usage

```rust
use developer_sdk::governance::{
    GovernanceKeypair, GovernanceMessage, Multisig, sign_message
};

// Generate keypairs for 3-of-5 multisig
let keypairs: Vec<_> = (0..5).map(|_| GovernanceKeypair::generate().unwrap()).collect();
let public_keys: Vec<_> = keypairs.iter().map(|kp| kp.public_key()).collect();

// Create multisig
let multisig = Multisig::new(3, 5, public_keys)?;

// Create message
let message = GovernanceMessage::Release {
    version: "v1.0.0".to_string(),
    commit_hash: "abc123".to_string(),
};

// Sign with 3 keys
let signatures: Vec<_> = keypairs[0..3]
    .iter()
    .map(|kp| sign_message(&kp.secret_key, &message.to_signing_bytes()).unwrap())
    .collect();

// Verify multisig
let verified = multisig.verify(&message.to_signing_bytes(), &signatures)?;
assert!(verified);
```

### Error Handling

```rust
use developer_sdk::governance::{GovernanceResult, GovernanceError};

fn process_governance() -> GovernanceResult<()> {
    let keypair = GovernanceKeypair::generate()?;
    
    // Handle specific errors
    match keypair.public_key().to_bytes().as_slice() {
        [0, ..] => return Err(GovernanceError::InvalidKey("Invalid public key".to_string())),
        _ => {}
    }
    
    Ok(())
}
```

## Dependencies

The library depends on the following crates:

- `secp256k1` - Cryptographic operations
- `bitcoin` - Bitcoin message signing standards
- `sha2` - Hashing
- `serde` - Serialization
- `thiserror` - Error handling
- `hex` - Hex encoding
- `base64` - Base64 encoding

## Version Compatibility

- **Rust**: 1.70+
- **secp256k1**: 0.28.2
- **bitcoin**: 0.31.2
- **serde**: 1.0.226

## Security Notes

- All cryptographic operations use Bitcoin-compatible standards
- Dependencies are pinned to exact versions for security
- Private keys are never stored by the library
- Signature verification is constant-time where possible




