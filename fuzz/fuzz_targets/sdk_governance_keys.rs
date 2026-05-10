#![no_main]
//! Governance secp256k1 keys/signatures, multisig verify, BIP32 master derivation (`default-features = false`).
use blvm_sdk::governance::bip32::derive_master_key;
use blvm_sdk::governance::verification::{verify_multiple_signatures, verify_signature_hash};
use blvm_sdk::governance::{verify_signature, GovernanceKeypair, Multisig, PublicKey, Signature};
use libfuzzer_sys::fuzz_target;

const CAP: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    let data = if data.len() > CAP {
        &data[..CAP]
    } else {
        data
    };

    let _ = Signature::from_bytes(data);
    if data.len() >= 64 {
        let _ = Signature::from_bytes(&data[..64]);
    }

    let _ = PublicKey::from_bytes(data);
    for len in [33usize, 65] {
        if data.len() >= len {
            let _ = PublicKey::from_bytes(&data[..len]);
        }
    }

    let _ = GovernanceKeypair::from_secret_key(data);
    if data.len() >= 32 {
        let _ = GovernanceKeypair::from_secret_key(&data[..32]);
    }

    if data.len() >= 16 {
        let end = data.len().min(64);
        let _ = derive_master_key(&data[..end]);
    }

    if data.len() >= 33 + 64 {
        if let Ok(pk) = PublicKey::from_bytes(&data[..33]) {
            if let Ok(m) = Multisig::new(1, 1, vec![pk]) {
                if let Ok(sig) = Signature::from_bytes(&data[33..97]) {
                    let _ = m.verify(data, std::slice::from_ref(&sig));
                    let _ = m.is_valid_signature(&sig, data);
                }
            }
        }
    }

    // Layout: [64 sig][33 pk][32 msg_hash][… message]
    if data.len() >= 129 {
        if let (Ok(sig), Ok(pk)) = (
            Signature::from_bytes(&data[..64]),
            PublicKey::from_bytes(&data[64..97]),
        ) {
            let _ = verify_signature_hash(&sig, &data[97..129], &pk);
            let _ = verify_signature(&sig, &data[129..], &pk);
        }
    }

    if data.len() >= 97 {
        if let (Ok(sig), Ok(pk)) = (
            Signature::from_bytes(&data[..64]),
            PublicKey::from_bytes(&data[64..97]),
        ) {
            let _ = verify_multiple_signatures(&[sig], data, &[pk]);
        }
    }
});
