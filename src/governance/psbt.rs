//! BIP174: Partially Signed Bitcoin Transaction (PSBT)
//!
//! Specification: https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki
//!
//! PSBT format enables multi-party transaction signing without exposing private keys.
//! Critical for hardware wallet support and transaction coordination.

use crate::governance::error::{GovernanceError, GovernanceResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PSBT magic bytes: 0x70736274 ("psbt")
pub const PSBT_MAGIC: [u8; 4] = [0x70, 0x73, 0x62, 0x74];

/// PSBT separator: 0xff
pub const PSBT_SEPARATOR: u8 = 0xff;

/// PSBT global map key types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PsbtGlobalKey {
    /// Unsigned transaction (required)
    UnsignedTx = 0x00,
    /// Extended public key (BIP32)
    Xpub = 0x01,
    /// Version number
    Version = 0xfb,
    /// Proprietary data
    Proprietary = 0xfc,
}

/// PSBT input map key types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PsbtInputKey {
    /// Non-witness UTXO
    NonWitnessUtxo = 0x00,
    /// Witness UTXO
    WitnessUtxo = 0x01,
    /// Partial signature
    PartialSig = 0x02,
    /// Sighash type
    SighashType = 0x03,
    /// Redeem script
    RedeemScript = 0x04,
    /// Witness script
    WitnessScript = 0x05,
    /// BIP32 derivation path
    Bip32Derivation = 0x06,
    /// Final script sig
    FinalScriptSig = 0x07,
    /// Final script witness
    FinalScriptWitness = 0x08,
    /// Proprietary data
    Proprietary = 0xfc,
}

/// PSBT output map key types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PsbtOutputKey {
    /// Redeem script
    RedeemScript = 0x00,
    /// Witness script
    WitnessScript = 0x01,
    /// BIP32 derivation path
    Bip32Derivation = 0x02,
    /// Proprietary data
    Proprietary = 0xfc,
}

/// BIP32 derivation path entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bip32Derivation {
    /// Public key (33 bytes compressed or 65 bytes uncompressed)
    pub pubkey: Vec<u8>,
    /// Derivation path
    pub path: Vec<u32>,
    /// Master key fingerprint (4 bytes)
    pub master_fingerprint: [u8; 4],
}

/// Partial signature entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialSignature {
    /// Public key
    pub pubkey: Vec<u8>,
    /// Signature
    pub signature: Vec<u8>,
}

/// Sighash type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SighashType {
    /// SIGHASH_ALL
    All = 0x01,
    /// SIGHASH_NONE
    None = 0x02,
    /// SIGHASH_SINGLE
    Single = 0x03,
    /// SIGHASH_ALL | SIGHASH_ANYONECANPAY
    AllAnyoneCanPay = 0x81,
    /// SIGHASH_NONE | SIGHASH_ANYONECANPAY
    NoneAnyoneCanPay = 0x82,
    /// SIGHASH_SINGLE | SIGHASH_ANYONECANPAY
    SingleAnyoneCanPay = 0x83,
}

impl SighashType {
    /// Parse sighash type from byte
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(SighashType::All),
            0x02 => Some(SighashType::None),
            0x03 => Some(SighashType::Single),
            0x81 => Some(SighashType::AllAnyoneCanPay),
            0x82 => Some(SighashType::NoneAnyoneCanPay),
            0x83 => Some(SighashType::SingleAnyoneCanPay),
            _ => None,
        }
    }

    /// Get byte representation
    pub fn to_byte(self) -> u8 {
        self as u8
    }
}

/// Partially Signed Bitcoin Transaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartiallySignedTransaction {
    /// Global map (unsigned transaction, xpubs, etc.)
    pub global: HashMap<Vec<u8>, Vec<u8>>,
    /// Input maps (one per input)
    pub inputs: Vec<HashMap<Vec<u8>, Vec<u8>>>,
    /// Output maps (one per output)
    pub outputs: Vec<HashMap<Vec<u8>, Vec<u8>>>,
    /// Version (default: 0)
    pub version: u8,
}

impl PartiallySignedTransaction {
    /// Create a new PSBT from an unsigned transaction
    pub fn new(unsigned_tx: &[u8]) -> GovernanceResult<Self> {
        let mut global = HashMap::new();
        global.insert(vec![PsbtGlobalKey::UnsignedTx as u8], unsigned_tx.to_vec());
        global.insert(vec![PsbtGlobalKey::Version as u8], vec![0x00]); // Version 0

        Ok(PartiallySignedTransaction {
            global,
            inputs: Vec::new(),
            outputs: Vec::new(),
            version: 0,
        })
    }

    /// Add input data
    pub fn add_input_data(&mut self, input_index: usize, key: Vec<u8>, value: Vec<u8>) -> GovernanceResult<()> {
        if input_index >= self.inputs.len() {
            // Extend inputs vector if needed
            while self.inputs.len() <= input_index {
                self.inputs.push(HashMap::new());
            }
        }
        self.inputs[input_index].insert(key, value);
        Ok(())
    }

    /// Add output data
    pub fn add_output_data(&mut self, output_index: usize, key: Vec<u8>, value: Vec<u8>) -> GovernanceResult<()> {
        if output_index >= self.outputs.len() {
            // Extend outputs vector if needed
            while self.outputs.len() <= output_index {
                self.outputs.push(HashMap::new());
            }
        }
        self.outputs[output_index].insert(key, value);
        Ok(())
    }

    /// Add partial signature to an input
    pub fn add_partial_signature(
        &mut self,
        input_index: usize,
        pubkey: Vec<u8>,
        signature: Vec<u8>,
    ) -> GovernanceResult<()> {
        // Format: <pubkey_len><pubkey><sig_len><signature>
        let mut key = vec![PsbtInputKey::PartialSig as u8];
        key.extend_from_slice(&pubkey);

        let mut value = Vec::with_capacity(1 + signature.len());
        value.push(signature.len() as u8);
        value.extend_from_slice(&signature);

        self.add_input_data(input_index, key, value)
    }

    /// Add BIP32 derivation path to an input
    pub fn add_bip32_derivation(
        &mut self,
        input_index: usize,
        pubkey: Vec<u8>,
        derivation: Bip32Derivation,
    ) -> GovernanceResult<()> {
        let mut key = vec![PsbtInputKey::Bip32Derivation as u8];
        key.extend_from_slice(&pubkey);

        // Serialize derivation: <master_fp(4)><path_len><path>
        let mut value = Vec::new();
        value.extend_from_slice(&derivation.master_fingerprint);
        value.push(derivation.path.len() as u8);
        for &index in &derivation.path {
            value.extend_from_slice(&index.to_be_bytes());
        }

        self.add_input_data(input_index, key, value)
    }

    /// Set sighash type for an input
    pub fn set_sighash_type(&mut self, input_index: usize, sighash_type: SighashType) -> GovernanceResult<()> {
        let key = vec![PsbtInputKey::SighashType as u8];
        let value = vec![sighash_type.to_byte()];
        self.add_input_data(input_index, key, value)
    }

    /// Check if PSBT is finalized (all inputs have final script sig/witness)
    pub fn is_finalized(&self) -> bool {
        for input_map in &self.inputs {
            let has_final_sig = input_map.contains_key(&vec![PsbtInputKey::FinalScriptSig as u8]);
            let has_final_witness = input_map.contains_key(&vec![PsbtInputKey::FinalScriptWitness as u8]);
            
            if !has_final_sig && !has_final_witness {
                return false;
            }
        }
        true
    }

    /// Extract final transaction (throws error if not finalized)
    pub fn extract_transaction(&self) -> GovernanceResult<Vec<u8>> {
        if !self.is_finalized() {
            return Err(GovernanceError::InvalidInput(
                "PSBT is not finalized".to_string()
            ));
        }

        // Get unsigned transaction from global map
        let unsigned_tx_key = vec![PsbtGlobalKey::UnsignedTx as u8];
        let unsigned_tx = self.global.get(&unsigned_tx_key)
            .ok_or_else(|| GovernanceError::InvalidInput("Missing unsigned transaction".to_string()))?;

        // Build final transaction by combining unsigned tx with final scripts
        // This is a simplified version - full implementation would parse transaction
        // and insert final script sig/witness data
        
        Ok(unsigned_tx.clone())
    }

    /// Serialize PSBT to bytes
    pub fn serialize(&self) -> GovernanceResult<Vec<u8>> {
        let mut result = Vec::new();
        
        // Magic bytes
        result.extend_from_slice(&PSBT_MAGIC);
        result.push(PSBT_SEPARATOR);
        
        // Global map
        serialize_map(&mut result, &self.global)?;
        
        // Separator between global and inputs
        result.push(PSBT_SEPARATOR);
        
        // Input maps
        for input_map in &self.inputs {
            serialize_map(&mut result, input_map)?;
            result.push(PSBT_SEPARATOR);
        }
        
        // Output maps
        for output_map in &self.outputs {
            serialize_map(&mut result, output_map)?;
            result.push(PSBT_SEPARATOR);
        }
        
        Ok(result)
    }

    /// Deserialize PSBT from bytes
    pub fn deserialize(data: &[u8]) -> GovernanceResult<Self> {
        if data.len() < 5 || &data[..4] != &PSBT_MAGIC || data[4] != PSBT_SEPARATOR {
            return Err(GovernanceError::InvalidInput(
                "Invalid PSBT magic bytes".to_string()
            ));
        }

        let mut offset = 5;
        
        // Parse global map
        let (global, new_offset) = deserialize_map(&data[offset..])?;
        offset += new_offset;
        
        // Skip separator
        if offset >= data.len() || data[offset] != PSBT_SEPARATOR {
            return Err(GovernanceError::InvalidInput("Missing separator after global map".to_string()));
        }
        offset += 1;
        
        // Parse input maps
        let mut inputs = Vec::new();
        // Determine number of inputs from unsigned transaction
        // For now, parse until we hit output separator or end
        while offset < data.len() && data[offset] != PSBT_SEPARATOR {
            let (input_map, new_offset) = deserialize_map(&data[offset..])?;
            inputs.push(input_map);
            offset += new_offset;
            
            // Skip separator
            if offset < data.len() && data[offset] == PSBT_SEPARATOR {
                offset += 1;
                break; // Separator indicates start of outputs
            }
        }
        
        // Parse output maps
        let mut outputs = Vec::new();
        while offset < data.len() {
            if data[offset] == PSBT_SEPARATOR && offset + 1 >= data.len() {
                break; // Final separator
            }
            let (output_map, new_offset) = deserialize_map(&data[offset..])?;
            outputs.push(output_map);
            offset += new_offset;
            
            if offset < data.len() && data[offset] == PSBT_SEPARATOR {
                offset += 1;
            }
        }
        
        // Extract version
        let version_key = vec![PsbtGlobalKey::Version as u8];
        let version = global.get(&version_key)
            .and_then(|v| v.first().copied())
            .unwrap_or(0);

        Ok(PartiallySignedTransaction {
            global,
            inputs,
            outputs,
            version,
        })
    }
}

/// Serialize a key-value map (CompactSize encoding)
fn serialize_map(result: &mut Vec<u8>, map: &HashMap<Vec<u8>, Vec<u8>>) -> GovernanceResult<()> {
    for (key, value) in map {
        // Key length (compact size)
        write_compact_size(result, key.len())?;
        result.extend_from_slice(key);
        
        // Value length (compact size)
        write_compact_size(result, value.len())?;
        result.extend_from_slice(value);
    }
    
    // End marker: 0x00
    result.push(0x00);
    
    Ok(())
}

/// Deserialize a key-value map
fn deserialize_map(data: &[u8]) -> GovernanceResult<(HashMap<Vec<u8>, Vec<u8>>, usize)> {
    let mut map = HashMap::new();
    let mut offset = 0;
    
    while offset < data.len() {
        // Check for end marker
        if data[offset] == 0x00 {
            offset += 1;
            break;
        }
        
        // Read key
        let (key_len, len_offset) = read_compact_size(&data[offset..])?;
        offset += len_offset;
        
        if offset + key_len > data.len() {
            return Err(GovernanceError::InvalidInput("Invalid key length".to_string()));
        }
        let key = data[offset..offset + key_len].to_vec();
        offset += key_len;
        
        // Read value
        let (value_len, len_offset) = read_compact_size(&data[offset..])?;
        offset += len_offset;
        
        if offset + value_len > data.len() {
            return Err(GovernanceError::InvalidInput("Invalid value length".to_string()));
        }
        let value = data[offset..offset + value_len].to_vec();
        offset += value_len;
        
        map.insert(key, value);
    }
    
    Ok((map, offset))
}

/// Write compact size (VarInt encoding)
fn write_compact_size(result: &mut Vec<u8>, size: usize) -> GovernanceResult<()> {
    if size < 0xfd {
        result.push(size as u8);
    } else if size <= 0xffff {
        result.push(0xfd);
        result.extend_from_slice(&(size as u16).to_le_bytes());
    } else if size <= 0xffffffff {
        result.push(0xfe);
        result.extend_from_slice(&(size as u32).to_le_bytes());
    } else {
        result.push(0xff);
        result.extend_from_slice(&(size as u64).to_le_bytes());
    }
    Ok(())
}

/// Read compact size (VarInt decoding)
fn read_compact_size(data: &[u8]) -> GovernanceResult<(usize, usize)> {
    if data.is_empty() {
        return Err(GovernanceError::InvalidInput("Unexpected end of data".to_string()));
    }
    
    match data[0] {
        n if n < 0xfd => Ok((n as usize, 1)),
        0xfd => {
            if data.len() < 3 {
                return Err(GovernanceError::InvalidInput("Invalid compact size".to_string()));
            }
            let value = u16::from_le_bytes([data[1], data[2]]) as usize;
            Ok((value, 3))
        }
        0xfe => {
            if data.len() < 5 {
                return Err(GovernanceError::InvalidInput("Invalid compact size".to_string()));
            }
            let value = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;
            Ok((value, 5))
        }
        0xff => {
            if data.len() < 9 {
                return Err(GovernanceError::InvalidInput("Invalid compact size".to_string()));
            }
            let value = u64::from_le_bytes([
                data[1], data[2], data[3], data[4],
                data[5], data[6], data[7], data[8],
            ]) as usize;
            Ok((value, 9))
        }
        _ => Err(GovernanceError::InvalidInput("Invalid compact size marker".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psbt_creation() {
        let unsigned_tx = vec![0x01, 0x00, 0x00, 0x00]; // Dummy transaction
        let psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();
        
        assert_eq!(psbt.version, 0);
        assert!(psbt.global.contains_key(&vec![PsbtGlobalKey::UnsignedTx as u8]));
    }

    #[test]
    fn test_serialize_deserialize() {
        let unsigned_tx = vec![0x01, 0x00, 0x00, 0x00];
        let mut psbt = PartiallySignedTransaction::new(&unsigned_tx).unwrap();
        
        // Add some data
        psbt.add_partial_signature(0, vec![0x02; 33], vec![0x30; 72]).unwrap();
        
        let serialized = psbt.serialize().unwrap();
        let deserialized = PartiallySignedTransaction::deserialize(&serialized).unwrap();
        
        assert_eq!(psbt.global, deserialized.global);
    }

    #[test]
    fn test_compact_size_encoding() {
        let mut result = Vec::new();
        write_compact_size(&mut result, 253).unwrap();
        assert_eq!(result[0], 0xfd);
        
        let (value, offset) = read_compact_size(&result).unwrap();
        assert_eq!(value, 253);
        assert_eq!(offset, 3);
    }
}

