#![no_main]
//! Governance parse surfaces with strict input caps (≤64 KiB enforced by libFuzzer default; we slice anyway).
use blvm_sdk::governance::bip39::{mnemonic_from_entropy, validate_mnemonic};
use blvm_sdk::GovernanceMessage;
use libfuzzer_sys::fuzz_target;

const CAP: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    let data = if data.len() > CAP {
        &data[..CAP]
    } else {
        data
    };

    let _ = serde_json::from_slice::<GovernanceMessage>(data);

    let sizes = [16usize, 20, 24, 28, 32];
    let ent_len = sizes[data.first().copied().unwrap_or(0) as usize % sizes.len()];
    if data.len() >= ent_len {
        let _ = mnemonic_from_entropy(&data[..ent_len]);
    }

    if let Ok(s) = std::str::from_utf8(data) {
        let words: Vec<String> = s
            .split_whitespace()
            .take(24)
            .map(|w| w.to_string())
            .collect();
        if !words.is_empty() {
            let _ = validate_mnemonic(&words);
        }
    }

});
