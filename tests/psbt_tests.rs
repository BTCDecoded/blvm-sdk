//! PSBT (Partially Signed Bitcoin Transaction) Tests
//!
//! Tests for BIP174 PSBT format implementation.
//! Specification: https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki

use blvm_sdk::governance::psbt::{
    Bip32Derivation, PSBT_MAGIC, PSBT_SEPARATOR, PartialSignature, PartiallySignedTransaction,
    PsbtGlobalKey, PsbtInputKey, PsbtOutputKey, SighashType,
};

/// Test helper: Create a minimal unsigned transaction (mock)
fn create_mock_unsigned_tx() -> Vec<u8> {
    let mut tx = vec![
        0x01, 0x00, 0x00, 0x00, // version
        0x01, // input count
    ];
    tx.extend_from_slice(&[0u8; 32]); // prevout hash
    tx.extend_from_slice(&[0xff, 0xff, 0xff, 0xff]); // prevout index
    tx.push(0x00); // scriptSig length
    tx.extend_from_slice(&[0xff, 0xff, 0xff, 0xff]); // sequence
    tx.push(0x01); // output count
    tx.extend_from_slice(&[0u8; 8]); // value
    tx.push(0x00); // scriptPubKey length
    tx.extend_from_slice(&[0u8; 4]); // locktime
    tx
}

/// Test helper: 0-input / 0-output unsigned transaction
fn create_empty_unsigned_tx() -> Vec<u8> {
    vec![
        0x02, 0x00, 0x00, 0x00, // version
        0x00, // input count
        0x00, // output count
        0x00, 0x00, 0x00, 0x00, // locktime
    ]
}

// ============================================================================
// Phase 1: PSBT Creation Tests
// ============================================================================

#[test]
fn test_psbt_creation() {
    // Test creating a PSBT from unsigned transaction
    let unsigned_tx = create_mock_unsigned_tx();
    let psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    // Should have version 0
    assert_eq!(psbt.version, 0);

    // Should have unsigned transaction in global map
    assert!(
        psbt.global
            .contains_key(&vec![PsbtGlobalKey::UnsignedTx as u8])
    );
}

#[test]
fn test_psbt_version() {
    // Test PSBT version is set correctly
    let unsigned_tx = create_mock_unsigned_tx();
    let psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    assert_eq!(psbt.version, 0);
    assert!(
        psbt.global
            .contains_key(&vec![PsbtGlobalKey::Version as u8])
    );
}

#[test]
fn test_psbt_magic_bytes() {
    // Test PSBT magic bytes constant
    assert_eq!(PSBT_MAGIC, [0x70, 0x73, 0x62, 0x74]); // "psbt"
}

#[test]
fn test_psbt_separator() {
    // Test PSBT separator constant
    assert_eq!(PSBT_SEPARATOR, 0xff);
}

// ============================================================================
// Phase 2: PSBT Input/Output Map Tests
// ============================================================================

#[test]
fn test_psbt_add_input() {
    // Test adding input to PSBT
    let unsigned_tx = create_mock_unsigned_tx();
    let psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    assert_eq!(psbt.inputs.len(), 1);
}

#[test]
fn test_psbt_add_output() {
    // Test adding output to PSBT
    let unsigned_tx = create_mock_unsigned_tx();
    let psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    assert_eq!(psbt.outputs.len(), 1);
}

#[test]
fn test_psbt_multiple_inputs_outputs() {
    // Test PSBT with multiple inputs and outputs
    let unsigned_tx = create_mock_unsigned_tx();
    let mut psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    psbt.inputs.push(std::collections::HashMap::new());
    psbt.outputs.push(std::collections::HashMap::new());

    assert_eq!(psbt.inputs.len(), 2);
    assert_eq!(psbt.outputs.len(), 2);
}

// ============================================================================
// Phase 3: PSBT Key Type Tests
// ============================================================================

#[test]
fn test_psbt_global_key_types() {
    // Test PSBT global key types
    assert_eq!(PsbtGlobalKey::UnsignedTx as u8, 0x00);
    assert_eq!(PsbtGlobalKey::Xpub as u8, 0x01);
    assert_eq!(PsbtGlobalKey::Version as u8, 0xfb);
    assert_eq!(PsbtGlobalKey::Proprietary as u8, 0xfc);
}

#[test]
fn test_psbt_input_key_types() {
    // Test PSBT input key types
    assert_eq!(PsbtInputKey::NonWitnessUtxo as u8, 0x00);
    assert_eq!(PsbtInputKey::WitnessUtxo as u8, 0x01);
    assert_eq!(PsbtInputKey::PartialSig as u8, 0x02);
    assert_eq!(PsbtInputKey::SighashType as u8, 0x03);
    assert_eq!(PsbtInputKey::RedeemScript as u8, 0x04);
    assert_eq!(PsbtInputKey::WitnessScript as u8, 0x05);
    assert_eq!(PsbtInputKey::Bip32Derivation as u8, 0x06);
    assert_eq!(PsbtInputKey::FinalScriptSig as u8, 0x07);
    assert_eq!(PsbtInputKey::FinalScriptWitness as u8, 0x08);
}

#[test]
fn test_psbt_output_key_types() {
    // Test PSBT output key types
    assert_eq!(PsbtOutputKey::RedeemScript as u8, 0x00);
    assert_eq!(PsbtOutputKey::WitnessScript as u8, 0x01);
    assert_eq!(PsbtOutputKey::Bip32Derivation as u8, 0x02);
}

// ============================================================================
// Phase 4: Sighash Type Tests
// ============================================================================

#[test]
fn test_sighash_type_all() {
    // Test SIGHASH_ALL
    let sighash = SighashType::All;
    assert_eq!(sighash.to_byte(), 0x01);

    let parsed = SighashType::from_byte(0x01);
    assert_eq!(parsed, Some(SighashType::All));
}

#[test]
fn test_sighash_type_none() {
    // Test SIGHASH_NONE
    let sighash = SighashType::None;
    assert_eq!(sighash.to_byte(), 0x02);

    let parsed = SighashType::from_byte(0x02);
    assert_eq!(parsed, Some(SighashType::None));
}

#[test]
fn test_sighash_type_single() {
    // Test SIGHASH_SINGLE
    let sighash = SighashType::Single;
    assert_eq!(sighash.to_byte(), 0x03);

    let parsed = SighashType::from_byte(0x03);
    assert_eq!(parsed, Some(SighashType::Single));
}

#[test]
fn test_sighash_type_anyonecanpay() {
    // Test SIGHASH_ANYONECANPAY variants
    assert_eq!(SighashType::AllAnyoneCanPay.to_byte(), 0x81);
    assert_eq!(SighashType::NoneAnyoneCanPay.to_byte(), 0x82);
    assert_eq!(SighashType::SingleAnyoneCanPay.to_byte(), 0x83);

    assert_eq!(
        SighashType::from_byte(0x81),
        Some(SighashType::AllAnyoneCanPay)
    );
    assert_eq!(
        SighashType::from_byte(0x82),
        Some(SighashType::NoneAnyoneCanPay)
    );
    assert_eq!(
        SighashType::from_byte(0x83),
        Some(SighashType::SingleAnyoneCanPay)
    );
}

#[test]
fn test_sighash_type_invalid() {
    // Test invalid sighash type
    let parsed = SighashType::from_byte(0xff);
    assert_eq!(parsed, None);
}

// ============================================================================
// Phase 5: BIP32 Derivation Tests
// ============================================================================

#[test]
fn test_bip32_derivation_creation() {
    // Test creating BIP32 derivation entry
    let derivation = Bip32Derivation {
        pubkey: vec![0x02; 33],                         // Compressed public key
        path: vec![0x80000000, 0x80000001, 0x80000002], // Hardened path
        master_fingerprint: [0x12, 0x34, 0x56, 0x78],
    };

    assert_eq!(derivation.pubkey.len(), 33);
    assert_eq!(derivation.path.len(), 3);
    assert_eq!(derivation.master_fingerprint, [0x12, 0x34, 0x56, 0x78]);
}

#[test]
fn test_bip32_derivation_path() {
    // Test BIP32 derivation path
    let derivation = Bip32Derivation {
        pubkey: vec![0x02; 33],
        path: vec![
            0x8000002c, // 44' (purpose)
            0x80000000, // 0' (coin type)
            0x80000000, // 0' (account)
            0x00000000, // 0 (change)
            0x00000000, // 0 (address index)
        ],
        master_fingerprint: [0x12, 0x34, 0x56, 0x78],
    };

    // Verify path structure
    assert_eq!(derivation.path[0], 0x8000002c); // 44' hardened
    assert_eq!(derivation.path[1], 0x80000000); // 0' hardened
}

// ============================================================================
// Phase 6: Partial Signature Tests
// ============================================================================

#[test]
fn test_partial_signature_creation() {
    // Test creating partial signature entry
    let partial_sig = PartialSignature {
        pubkey: vec![0x02; 33],                  // Public key
        signature: vec![0x30, 0x45, 0x02, 0x21], // Mock signature bytes
    };

    assert_eq!(partial_sig.pubkey.len(), 33);
    assert!(!partial_sig.signature.is_empty());
}

#[test]
fn test_partial_signature_structure() {
    // Test partial signature structure
    let pubkey = vec![0x03; 33];
    let signature = vec![0x30, 0x44, 0x02, 0x20, 0x01, 0x02, 0x03];

    let partial_sig = PartialSignature {
        pubkey: pubkey.clone(),
        signature: signature.clone(),
    };

    assert_eq!(partial_sig.pubkey, pubkey);
    assert_eq!(partial_sig.signature, signature);
}

// ============================================================================
// Phase 7: PSBT Global Map Tests
// ============================================================================

#[test]
fn test_psbt_global_map_unsigned_tx() {
    // Test unsigned transaction in global map
    let unsigned_tx = create_mock_unsigned_tx();
    let psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    let key = vec![PsbtGlobalKey::UnsignedTx as u8];
    assert!(psbt.global.contains_key(&key));

    let tx_data = psbt.global.get(&key).unwrap();
    assert_eq!(tx_data, &unsigned_tx);
}

#[test]
fn test_psbt_global_map_version() {
    // Test version in global map
    let unsigned_tx = create_mock_unsigned_tx();
    let psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    let key = vec![PsbtGlobalKey::Version as u8];
    assert!(psbt.global.contains_key(&key));

    let version_data = psbt.global.get(&key).unwrap();
    assert_eq!(version_data, &vec![0x00]); // Version 0
}

// ============================================================================
// Phase 8: PSBT Input Map Tests
// ============================================================================

#[test]
fn test_psbt_input_map_witness_utxo() {
    // Test adding witness UTXO to input map
    let unsigned_tx = create_mock_unsigned_tx();
    let mut psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    let mut input_map = std::collections::HashMap::new();
    let witness_utxo = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]; // Mock UTXO
    input_map.insert(vec![PsbtInputKey::WitnessUtxo as u8], witness_utxo.clone());

    psbt.inputs[0] = input_map;

    assert!(psbt.inputs[0].contains_key(&vec![PsbtInputKey::WitnessUtxo as u8]));
}

#[test]
fn test_psbt_input_map_partial_sig() {
    // Test adding partial signature to input map
    let unsigned_tx = create_mock_unsigned_tx();
    let mut psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    let mut input_map = std::collections::HashMap::new();
    let partial_sig_data = vec![0x30, 0x45, 0x02, 0x21]; // Mock signature
    input_map.insert(
        vec![PsbtInputKey::PartialSig as u8],
        partial_sig_data.clone(),
    );

    psbt.inputs[0] = input_map;

    assert!(psbt.inputs[0].contains_key(&vec![PsbtInputKey::PartialSig as u8]));
}

#[test]
fn test_psbt_input_map_sighash_type() {
    // Test adding sighash type to input map
    let unsigned_tx = create_mock_unsigned_tx();
    let mut psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    let mut input_map = std::collections::HashMap::new();
    let sighash_byte = vec![SighashType::All.to_byte()];
    input_map.insert(vec![PsbtInputKey::SighashType as u8], sighash_byte.clone());

    psbt.inputs[0] = input_map;

    assert!(psbt.inputs[0].contains_key(&vec![PsbtInputKey::SighashType as u8]));
}

#[test]
fn test_psbt_input_map_bip32_derivation() {
    // Test adding BIP32 derivation to input map
    let unsigned_tx = create_mock_unsigned_tx();
    let mut psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    let mut input_map = std::collections::HashMap::new();
    let derivation_data = vec![0x02; 33]; // Mock derivation data
    input_map.insert(
        vec![PsbtInputKey::Bip32Derivation as u8],
        derivation_data.clone(),
    );

    psbt.inputs[0] = input_map;

    assert!(psbt.inputs[0].contains_key(&vec![PsbtInputKey::Bip32Derivation as u8]));
}

// ============================================================================
// Phase 9: PSBT Output Map Tests
// ============================================================================

#[test]
fn test_psbt_output_map_redeem_script() {
    // Test adding redeem script to output map
    let unsigned_tx = create_mock_unsigned_tx();
    let mut psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    let mut output_map = std::collections::HashMap::new();
    let redeem_script = vec![0x76, 0xa9, 0x14]; // Mock redeem script
    output_map.insert(
        vec![PsbtOutputKey::RedeemScript as u8],
        redeem_script.clone(),
    );

    psbt.outputs[0] = output_map;

    assert!(psbt.outputs[0].contains_key(&vec![PsbtOutputKey::RedeemScript as u8]));
}

#[test]
fn test_psbt_output_map_witness_script() {
    // Test adding witness script to output map
    let unsigned_tx = create_mock_unsigned_tx();
    let mut psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    let mut output_map = std::collections::HashMap::new();
    let witness_script = vec![0x00, 0x14]; // Mock witness script
    output_map.insert(
        vec![PsbtOutputKey::WitnessScript as u8],
        witness_script.clone(),
    );

    psbt.outputs[0] = output_map;

    assert!(psbt.outputs[0].contains_key(&vec![PsbtOutputKey::WitnessScript as u8]));
}

#[test]
fn test_psbt_output_map_bip32_derivation() {
    // Test adding BIP32 derivation to output map
    let unsigned_tx = create_mock_unsigned_tx();
    let mut psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    let mut output_map = std::collections::HashMap::new();
    let derivation_data = vec![0x02; 33]; // Mock derivation data
    output_map.insert(
        vec![PsbtOutputKey::Bip32Derivation as u8],
        derivation_data.clone(),
    );

    psbt.outputs[0] = output_map;

    assert!(psbt.outputs[0].contains_key(&vec![PsbtOutputKey::Bip32Derivation as u8]));
}

// ============================================================================
// Phase 10: PSBT Validation Tests
// ============================================================================

#[test]
fn test_psbt_has_unsigned_tx() {
    // Test that PSBT has unsigned transaction
    let unsigned_tx = create_mock_unsigned_tx();
    let psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    // Should have unsigned transaction
    let key = vec![PsbtGlobalKey::UnsignedTx as u8];
    assert!(psbt.global.contains_key(&key));
}

#[test]
fn test_psbt_empty_inputs_outputs() {
    let unsigned_tx = create_empty_unsigned_tx();
    let psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    assert_eq!(psbt.inputs.len(), 0);
    assert_eq!(psbt.outputs.len(), 0);
}

#[test]
fn test_psbt_extract_merges_final_script_sig() {
    let unsigned_tx = create_mock_unsigned_tx();
    let mut psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    let script_sig = vec![0x47, 0x30, 0x44, 0x02, 0x20, 0x01];
    psbt.inputs[0].insert(vec![PsbtInputKey::FinalScriptSig as u8], script_sig.clone());

    assert!(psbt.is_finalized());
    let extracted = psbt.extract_transaction().unwrap();
    assert_ne!(extracted, unsigned_tx);
    assert!(extracted.starts_with(&unsigned_tx[0..4]));
    // script length byte at fixed offset after 36-byte prevout in single-input tx
    assert_eq!(extracted[41], script_sig.len() as u8);
}

#[test]
fn test_psbt_extract_rejects_unfinalized() {
    let unsigned_tx = create_mock_unsigned_tx();
    let psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();
    assert!(!psbt.is_finalized());
    assert!(psbt.extract_transaction().is_err());
}

#[test]
fn test_psbt_serialize_deserialize_preserves_input_output_counts() {
    let unsigned_tx = create_mock_unsigned_tx();
    let psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();

    let bytes = psbt.serialize().unwrap();
    let decoded = PartiallySignedTransaction::deserialize(&bytes).unwrap();
    assert_eq!(decoded.inputs.len(), 1);
    assert_eq!(decoded.outputs.len(), 1);
}
