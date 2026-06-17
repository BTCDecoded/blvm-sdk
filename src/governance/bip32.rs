//! BIP32: Hierarchical Deterministic Wallets
//!
//! Specification: https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki
//!
//! Key derivation path format: m/purpose'/coin_type'/account'/change/address_index
//! Example: m/44'/0'/0'/0/0 (BIP44 standard path for Bitcoin mainnet first address)

use crate::governance::error::{GovernanceError, GovernanceResult};
use blvm_secp256k1::ecdsa::{ge_from_compressed, ge_to_compressed, pubkey_from_secret};
use blvm_secp256k1::ecmult_gen_const;
use blvm_secp256k1::group::{Ge, Gej};
use blvm_secp256k1::scalar::Scalar;
use hmac::{Hmac, Mac};
use sha2::Sha512;

type HmacSha512 = Hmac<Sha512>;

/// Extended private key (xprv)
#[derive(Debug, Clone)]
pub struct ExtendedPrivateKey {
    pub depth: u8,
    pub parent_fingerprint: [u8; 4],
    pub child_number: u32,
    pub chain_code: [u8; 32],
    /// Raw 32-byte secret scalar.
    pub private_key: [u8; 32],
}

/// Extended public key (xpub)
#[derive(Debug, Clone)]
pub struct ExtendedPublicKey {
    pub depth: u8,
    pub parent_fingerprint: [u8; 4],
    pub child_number: u32,
    pub chain_code: [u8; 32],
    /// Compressed 33-byte public key.
    pub public_key: [u8; 33],
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn scalar_from_b32(b: &[u8; 32]) -> Option<Scalar> {
    let mut s = Scalar::zero();
    if s.set_b32(b) || s.is_zero() {
        None
    } else {
        Some(s)
    }
}

/// child_private_scalar = (IL + parent_private) mod n; None if invalid.
fn scalar_add(a: &Scalar, b: &Scalar) -> Option<Scalar> {
    let mut result = Scalar::zero();
    result.add(a, b); // sets result = a + b mod n; returns carry (ignored, mod n is always valid)
    if result.is_zero() { None } else { Some(result) }
}

/// Compute IL_scalar * G + parent_point (BIP32 public child derivation).
fn point_add_scalar_g(parent_ge: &Ge, il_scalar: &Scalar) -> Option<Ge> {
    // il_scalar * G
    let mut il_gej = Gej::default();
    ecmult_gen_const(&mut il_gej, il_scalar);
    let mut il_ge = Ge::default();
    il_ge.set_gej_var(&il_gej);
    if il_ge.infinity {
        return None;
    }
    // parent_gej + il_ge
    let mut parent_gej = Gej::default();
    parent_gej.set_ge(parent_ge);
    let mut child_gej = Gej::default();
    child_gej.add_ge(&parent_gej, &il_ge);
    let mut child_ge = Ge::default();
    child_ge.set_gej_var(&child_gej);
    if child_ge.infinity {
        None
    } else {
        Some(child_ge)
    }
}

fn calculate_fingerprint(pubkey: &[u8; 33]) -> [u8; 4] {
    use ripemd::{Digest as RipemdDigest, Ripemd160};
    use sha2::Sha256;
    let sha = Sha256::digest(pubkey);
    let ripemd: [u8; 20] = Ripemd160::digest(sha).into();
    ripemd[..4].try_into().unwrap()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Derive master key from seed (BIP32).
pub fn derive_master_key(seed: &[u8]) -> GovernanceResult<(ExtendedPrivateKey, ExtendedPublicKey)> {
    if seed.len() < 16 || seed.len() > 64 {
        return Err(GovernanceError::InvalidInput(
            "Seed must be 16-64 bytes".to_string(),
        ));
    }
    let mut hmac = HmacSha512::new_from_slice(b"Bitcoin seed")
        .map_err(|e| GovernanceError::InvalidInput(format!("HMAC key error: {e}")))?;
    hmac.update(seed);
    let bytes: [u8; 64] = hmac.finalize().into_bytes().into();

    let private_key_bytes: [u8; 32] = bytes[..32].try_into().unwrap();
    let chain_code: [u8; 32] = bytes[32..].try_into().unwrap();

    let sec = scalar_from_b32(&private_key_bytes)
        .ok_or_else(|| GovernanceError::InvalidKey("Invalid master private key".to_string()))?;
    let public_key = ge_to_compressed(&pubkey_from_secret(&sec));

    Ok((
        ExtendedPrivateKey {
            depth: 0,
            parent_fingerprint: [0u8; 4],
            child_number: 0,
            chain_code,
            private_key: private_key_bytes,
        },
        ExtendedPublicKey {
            depth: 0,
            parent_fingerprint: [0u8; 4],
            child_number: 0,
            chain_code,
            public_key,
        },
    ))
}

/// Derive child private key (BIP32). Hardened if `child_number >= 0x80000000`.
pub fn derive_child_private(
    parent: &ExtendedPrivateKey,
    child_number: u32,
) -> GovernanceResult<(ExtendedPrivateKey, ExtendedPublicKey)> {
    let is_hardened = child_number >= 0x80000000;
    let parent_sec = scalar_from_b32(&parent.private_key)
        .ok_or_else(|| GovernanceError::InvalidKey("Invalid parent private key".to_string()))?;
    let parent_pubkey = ge_to_compressed(&pubkey_from_secret(&parent_sec));
    let parent_fingerprint = calculate_fingerprint(&parent_pubkey);

    let mut data = Vec::with_capacity(37);
    if is_hardened {
        data.push(0x00);
        data.extend_from_slice(&parent.private_key);
    } else {
        data.extend_from_slice(&parent_pubkey);
    }
    data.extend_from_slice(&child_number.to_be_bytes());

    let mut hmac = HmacSha512::new_from_slice(&parent.chain_code)
        .map_err(|e| GovernanceError::InvalidInput(format!("HMAC error: {e}")))?;
    hmac.update(&data);
    let result: [u8; 64] = hmac.finalize().into_bytes().into();

    let il: [u8; 32] = result[..32].try_into().unwrap();
    let child_chain_code: [u8; 32] = result[32..].try_into().unwrap();

    let il_scalar = scalar_from_b32(&il)
        .ok_or_else(|| GovernanceError::InvalidKey("IL >= curve order, skip index".to_string()))?;
    let child_sec = scalar_add(&il_scalar, &parent_sec)
        .ok_or_else(|| GovernanceError::InvalidKey("Child key is zero, skip index".to_string()))?;

    let mut child_private = [0u8; 32];
    child_sec.get_b32(&mut child_private);
    let child_pubkey = ge_to_compressed(&pubkey_from_secret(&child_sec));

    Ok((
        ExtendedPrivateKey {
            depth: parent.depth + 1,
            parent_fingerprint,
            child_number,
            chain_code: child_chain_code,
            private_key: child_private,
        },
        ExtendedPublicKey {
            depth: parent.depth + 1,
            parent_fingerprint,
            child_number,
            chain_code: child_chain_code,
            public_key: child_pubkey,
        },
    ))
}

/// Derive child public key from parent public key (non-hardened only).
pub fn derive_child_public(
    parent: &ExtendedPublicKey,
    child_number: u32,
) -> GovernanceResult<ExtendedPublicKey> {
    if child_number >= 0x80000000 {
        return Err(GovernanceError::InvalidInput(
            "Hardened derivation requires private key".to_string(),
        ));
    }
    let parent_fingerprint = calculate_fingerprint(&parent.public_key);
    let mut data = Vec::with_capacity(37);
    data.extend_from_slice(&parent.public_key);
    data.extend_from_slice(&child_number.to_be_bytes());

    let mut hmac = HmacSha512::new_from_slice(&parent.chain_code)
        .map_err(|e| GovernanceError::InvalidInput(format!("HMAC error: {e}")))?;
    hmac.update(&data);
    let result: [u8; 64] = hmac.finalize().into_bytes().into();

    let il: [u8; 32] = result[..32].try_into().unwrap();
    let child_chain_code: [u8; 32] = result[32..].try_into().unwrap();

    let il_scalar = scalar_from_b32(&il)
        .ok_or_else(|| GovernanceError::InvalidKey("IL >= curve order, skip index".to_string()))?;

    let parent_ge = ge_from_compressed(&parent.public_key)
        .ok_or_else(|| GovernanceError::InvalidKey("Invalid parent public key".to_string()))?;

    let child_ge = point_add_scalar_g(&parent_ge, &il_scalar).ok_or_else(|| {
        GovernanceError::InvalidKey("Child pubkey is point at infinity".to_string())
    })?;

    Ok(ExtendedPublicKey {
        depth: parent.depth + 1,
        parent_fingerprint,
        child_number,
        chain_code: child_chain_code,
        public_key: ge_to_compressed(&child_ge),
    })
}

impl ExtendedPrivateKey {
    pub fn to_extended_public(&self) -> ExtendedPublicKey {
        let sec = scalar_from_b32(&self.private_key).expect("stored key is always valid");
        ExtendedPublicKey {
            depth: self.depth,
            parent_fingerprint: self.parent_fingerprint,
            child_number: self.child_number,
            chain_code: self.chain_code,
            public_key: ge_to_compressed(&pubkey_from_secret(&sec)),
        }
    }

    pub fn derive_child(
        &self,
        child_number: u32,
    ) -> GovernanceResult<(ExtendedPrivateKey, ExtendedPublicKey)> {
        derive_child_private(self, child_number)
    }

    pub fn private_key_bytes(&self) -> [u8; 32] {
        self.private_key
    }
}

impl ExtendedPublicKey {
    pub fn derive_child(&self, child_number: u32) -> GovernanceResult<ExtendedPublicKey> {
        derive_child_public(self, child_number)
    }

    pub fn public_key_bytes(&self) -> [u8; 33] {
        self.public_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_key_derivation() {
        let seed = b"Hello, Bitcoin Commons!";
        let (xprv, xpub) = derive_master_key(seed).unwrap();
        assert_eq!(xprv.depth, 0);
        assert_eq!(xpub.depth, 0);
        assert_eq!(xprv.child_number, 0);
    }

    #[test]
    fn test_child_derivation() {
        let seed = b"test seed for BIP32";
        let (master_xprv, _) = derive_master_key(seed).unwrap();
        let (child_xprv, child_xpub) = master_xprv.derive_child(0).unwrap();
        assert_eq!(child_xprv.depth, 1);
        assert_eq!(child_xpub.depth, 1);
        assert_eq!(child_xprv.child_number, 0);
        let derived_xpub = child_xprv.to_extended_public();
        assert_eq!(
            derived_xpub.public_key_bytes(),
            child_xpub.public_key_bytes()
        );
    }

    #[test]
    fn test_hardened_derivation() {
        let seed = b"test seed for hardened derivation";
        let (master_xprv, _) = derive_master_key(seed).unwrap();
        let hardened_index = 0x80000000;
        let (hardened_xprv, _) = master_xprv.derive_child(hardened_index).unwrap();
        assert_eq!(hardened_xprv.child_number, hardened_index);
        assert!(hardened_xprv.child_number >= 0x80000000);
    }
}
