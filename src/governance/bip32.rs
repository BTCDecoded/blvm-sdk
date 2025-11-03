//! BIP32: Hierarchical Deterministic Wallets
//!
//! Specification: https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki
//!
//! Implements HD key derivation using HMAC-SHA512 for extended keys.
//!
//! Key derivation path format: m/purpose'/coin_type'/account'/change/address_index
//! Example: m/44'/0'/0'/0/0 (BIP44 standard path for Bitcoin mainnet first address)

use hmac::{Hmac, Mac};
use sha2::Sha512;
use secp256k1::{Secp256k1, SecretKey, PublicKey, Scalar};
use crate::governance::error::{GovernanceError, GovernanceResult};

type HmacSha512 = Hmac<Sha512>;

/// Extended private key (xprv)
#[derive(Debug, Clone)]
pub struct ExtendedPrivateKey {
    /// Depth in derivation tree (0 = master)
    pub depth: u8,
    /// Parent fingerprint (4 bytes)
    pub parent_fingerprint: [u8; 4],
    /// Child number (index)
    pub child_number: u32,
    /// Chain code (32 bytes)
    pub chain_code: [u8; 32],
    /// Private key (32 bytes)
    pub private_key: SecretKey,
}

/// Extended public key (xpub)
#[derive(Debug, Clone)]
pub struct ExtendedPublicKey {
    /// Depth in derivation tree (0 = master)
    pub depth: u8,
    /// Parent fingerprint (4 bytes)
    pub parent_fingerprint: [u8; 4],
    /// Child number (index)
    pub child_number: u32,
    /// Chain code (32 bytes)
    pub chain_code: [u8; 32],
    /// Public key (33 bytes compressed)
    pub public_key: PublicKey,
}

/// Derive master key from seed
///
/// BIP32: I = HMAC-SHA512(Key = "Bitcoin seed", Data = seed)
///        IL = first 32 bytes (master private key)
///        IR = last 32 bytes (master chain code)
pub fn derive_master_key(seed: &[u8]) -> GovernanceResult<(ExtendedPrivateKey, ExtendedPublicKey)> {
    if seed.len() < 16 || seed.len() > 64 {
        return Err(GovernanceError::InvalidInput(
            "Seed must be 16-64 bytes".to_string()
        ));
    }

    let mut hmac = HmacSha512::new_from_slice(b"Bitcoin seed")
        .map_err(|e| GovernanceError::InvalidInput(format!("HMAC key error: {}", e)))?;
    hmac.update(seed);
    let result = hmac.finalize();
    let bytes = result.into_bytes();

    // Split into private key (IL) and chain code (IR)
    let mut private_key_bytes = [0u8; 32];
    private_key_bytes.copy_from_slice(&bytes[..32]);
    
    let mut chain_code = [0u8; 32];
    chain_code.copy_from_slice(&bytes[32..]);

    // Create secret key
    let secp = Secp256k1::new();
    let private_key = SecretKey::from_slice(&private_key_bytes)
        .map_err(|e| GovernanceError::InvalidKey(format!("Invalid master private key: {}", e)))?;
    
    let public_key = private_key.public_key(&secp);

    let xprv = ExtendedPrivateKey {
        depth: 0,
        parent_fingerprint: [0u8; 4],
        child_number: 0,
        chain_code,
        private_key,
    };

    let xpub = ExtendedPublicKey {
        depth: 0,
        parent_fingerprint: [0u8; 4],
        child_number: 0,
        chain_code,
        public_key,
    };

    Ok((xprv, xpub))
}

/// Derive child private key (BIP32)
///
/// If child_number >= 2^31, use hardened derivation (uses private key)
/// Otherwise, use normal derivation (can use public key)
pub fn derive_child_private(
    parent: &ExtendedPrivateKey,
    child_number: u32,
) -> GovernanceResult<(ExtendedPrivateKey, ExtendedPublicKey)> {
    let secp = Secp256k1::new();
    let is_hardened = child_number >= 0x80000000;

    // Prepare data for HMAC
    let mut data = Vec::with_capacity(37);
    
    if is_hardened {
        // Hardened: 0x00 || parent_private_key || child_number (4 bytes, big-endian)
        data.push(0x00);
        data.extend_from_slice(&parent.private_key.secret_bytes());
    } else {
        // Normal: parent_public_key || child_number (4 bytes, big-endian)
        let parent_pubkey = parent.private_key.public_key(&secp);
        data.extend_from_slice(&parent_pubkey.serialize());
    }
    
    data.extend_from_slice(&child_number.to_be_bytes());

    // Calculate parent fingerprint (first 4 bytes of RIPEMD160(SHA256(parent_pubkey)))
    let parent_pubkey = parent.private_key.public_key(&secp);
    let parent_fingerprint = calculate_fingerprint(&parent_pubkey.serialize());

    // HMAC-SHA512(chain_code, data)
    let mut hmac = HmacSha512::new_from_slice(&parent.chain_code)
        .map_err(|e| GovernanceError::InvalidInput(format!("HMAC error: {}", e)))?;
    hmac.update(&data);
    let result = hmac.finalize();
    let bytes = result.into_bytes();

    // Split result: IL (left 32 bytes) = child private key contribution
    //                IR (right 32 bytes) = child chain code
    let mut il = [0u8; 32];
    il.copy_from_slice(&bytes[..32]);
    
    let mut child_chain_code = [0u8; 32];
    child_chain_code.copy_from_slice(&bytes[32..]);

    // Add IL to parent private key (mod secp256k1 order)
    // BIP32: child_key = (IL + parent_key) mod n
    // IL is interpreted as a 256-bit integer (may be >= curve order, will be reduced mod n)
    // Convert IL to Scalar (this handles modulo curve order automatically)
    let il_scalar = Scalar::from_be_bytes(il)
        .map_err(|_| GovernanceError::InvalidKey("IL cannot be converted to scalar".to_string()))?;
    
    // Add IL scalar to parent private key using add_tweak
    let child_private = parent.private_key.add_tweak(&il_scalar)
        .map_err(|_| GovernanceError::InvalidKey("Key addition resulted in zero or invalid key".to_string()))?;
    
    let child_public = child_private.public_key(&secp);

    let child_xprv = ExtendedPrivateKey {
        depth: parent.depth + 1,
        parent_fingerprint,
        child_number,
        chain_code: child_chain_code,
        private_key: child_private,
    };

    let child_xpub = ExtendedPublicKey {
        depth: parent.depth + 1,
        parent_fingerprint,
        child_number,
        chain_code: child_chain_code,
        public_key: child_public,
    };

    Ok((child_xprv, child_xpub))
}

/// Derive child public key from parent public key (non-hardened only)
///
/// Note: Hardened derivation requires the private key and cannot be done from public key alone
pub fn derive_child_public(
    parent: &ExtendedPublicKey,
    child_number: u32,
) -> GovernanceResult<ExtendedPublicKey> {
    if child_number >= 0x80000000 {
        return Err(GovernanceError::InvalidInput(
            "Hardened derivation requires private key".to_string()
        ));
    }

    // Prepare data: parent_public_key || child_number (4 bytes, big-endian)
    let mut data = Vec::with_capacity(37);
    data.extend_from_slice(&parent.public_key.serialize());
    data.extend_from_slice(&child_number.to_be_bytes());

    // Calculate parent fingerprint
    let parent_fingerprint = calculate_fingerprint(&parent.public_key.serialize());
    
    // HMAC-SHA512(chain_code, data)
    let mut hmac = HmacSha512::new_from_slice(&parent.chain_code)
        .map_err(|e| GovernanceError::InvalidInput(format!("HMAC error: {}", e)))?;
    hmac.update(&data);
    let result = hmac.finalize();
    let bytes = result.into_bytes();

    // Split result
    let mut il = [0u8; 32];
    il.copy_from_slice(&bytes[..32]);
    
    let mut child_chain_code = [0u8; 32];
    child_chain_code.copy_from_slice(&bytes[32..]);

    // Add IL to parent public key (elliptic curve point addition)
    // BIP32: child_pubkey = parent_pubkey + IL * G (where G is generator)
    // Convert IL to scalar
    let il_scalar = Scalar::from_be_bytes(il)
        .map_err(|_| GovernanceError::InvalidKey("Invalid scalar".to_string()))?;
    
    // Add il_scalar * G to parent public key using add_exp_tweak
    // This computes: parent_pubkey + (il_scalar * G)
    let secp = Secp256k1::new();
    let child_public = parent.public_key.add_exp_tweak(&secp, &il_scalar)
        .map_err(|_| GovernanceError::InvalidKey("Point addition failed".to_string()))?;
    
    let parent_fingerprint = calculate_fingerprint(&parent.public_key.serialize());
    
    Ok(ExtendedPublicKey {
        depth: parent.depth + 1,
        parent_fingerprint,
        child_number,
        chain_code: child_chain_code,
        public_key: child_public,
    })
}

/// Calculate key fingerprint (first 4 bytes of RIPEMD160(SHA256(pubkey)))
fn calculate_fingerprint(pubkey: &[u8]) -> [u8; 4] {
    use sha2::{Sha256, Digest};
    use ripemd::{Ripemd160, Digest as RipemdDigest};
    
    // SHA256(pubkey)
    let mut sha256 = Sha256::new();
    sha256.update(pubkey);
    let sha256_hash = sha256.finalize();
    
    // RIPEMD160(SHA256(pubkey))
    let mut ripemd = Ripemd160::new();
    ripemd.update(&sha256_hash);
    let ripemd_hash = ripemd.finalize();
    
    // First 4 bytes
    let mut fingerprint = [0u8; 4];
    fingerprint.copy_from_slice(&ripemd_hash[..4]);
    fingerprint
}

impl ExtendedPrivateKey {
    /// Get the corresponding extended public key
    pub fn to_extended_public(&self) -> ExtendedPublicKey {
        let secp = Secp256k1::new();
        ExtendedPublicKey {
            depth: self.depth,
            parent_fingerprint: self.parent_fingerprint,
            child_number: self.child_number,
            chain_code: self.chain_code,
            public_key: self.private_key.public_key(&secp),
        }
    }

    /// Derive a child key
    pub fn derive_child(&self, child_number: u32) -> GovernanceResult<(ExtendedPrivateKey, ExtendedPublicKey)> {
        derive_child_private(self, child_number)
    }

    /// Get private key bytes
    pub fn private_key_bytes(&self) -> [u8; 32] {
        self.private_key.secret_bytes()
    }
}

impl ExtendedPublicKey {
    /// Derive a non-hardened child public key
    pub fn derive_child(&self, child_number: u32) -> GovernanceResult<ExtendedPublicKey> {
        derive_child_public(self, child_number)
    }

    /// Get public key bytes (compressed)
    pub fn public_key_bytes(&self) -> [u8; 33] {
        self.public_key.serialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_key_derivation() {
        // Test with a known seed
        let seed = b"Hello, Bitcoin Commons!";
        let (xprv, xpub) = derive_master_key(seed).unwrap();
        
        assert_eq!(xprv.depth, 0);
        assert_eq!(xpub.depth, 0);
        assert_eq!(xprv.child_number, 0);
        assert_eq!(xpub.child_number, 0);
    }

    #[test]
    fn test_child_derivation() {
        let seed = b"test seed for BIP32";
        let (master_xprv, master_xpub) = derive_master_key(seed).unwrap();
        
        // Derive first child
        let (child_xprv, child_xpub) = master_xprv.derive_child(0).unwrap();
        
        assert_eq!(child_xprv.depth, 1);
        assert_eq!(child_xpub.depth, 1);
        assert_eq!(child_xprv.child_number, 0);
        
        // Verify public key matches
        let derived_xpub = child_xprv.to_extended_public();
        assert_eq!(derived_xpub.public_key_bytes(), child_xpub.public_key_bytes());
    }

    #[test]
    fn test_hardened_derivation() {
        let seed = b"test seed for hardened derivation";
        let (master_xprv, _) = derive_master_key(seed).unwrap();
        
        // Hardened child (0x80000000 = 2147483648)
        let hardened_index = 0x80000000;
        let (hardened_xprv, _) = master_xprv.derive_child(hardened_index).unwrap();
        
        assert_eq!(hardened_xprv.child_number, hardened_index);
        assert!(hardened_xprv.child_number >= 0x80000000);
    }
}

