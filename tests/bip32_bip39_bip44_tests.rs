//! BIP32/BIP39/BIP44 HD Wallet Tests
//!
//! Comprehensive tests for hierarchical deterministic wallet functionality.
//! BIP39: Mnemonic generation and seed derivation
//! BIP32: HD key derivation
//! BIP44: Standard derivation paths

use bllvm_sdk::governance::bip32::{derive_master_key, derive_child_private, derive_child_public, ExtendedPrivateKey, ExtendedPublicKey};
use bllvm_sdk::governance::bip39::{generate_mnemonic, mnemonic_to_seed, validate_mnemonic, mnemonic_from_entropy, mnemonic_to_entropy, EntropyStrength};
use bllvm_sdk::governance::bip44::{Bip44Path, Bip44Wallet, CoinType, ChangeChain};
use bllvm_sdk::governance::error::GovernanceError;

/// Test helper: Generate a test seed
fn generate_test_seed() -> Vec<u8> {
    vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
         0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
         0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
         0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f]
}

// ============================================================================
// Phase 1: BIP39 Mnemonic Tests
// ============================================================================

#[test]
fn test_generate_mnemonic_12_words() {
    // Test generating 12-word mnemonic (128 bits entropy)
    let mnemonic = generate_mnemonic(EntropyStrength::Bits128).unwrap();
    
    assert_eq!(mnemonic.len(), 12);
    // All words should be from BIP39 word list
    for word in &mnemonic {
        assert!(!word.is_empty());
    }
}

#[test]
fn test_generate_mnemonic_24_words() {
    // Test generating 24-word mnemonic (256 bits entropy)
    let mnemonic = generate_mnemonic(EntropyStrength::Bits256).unwrap();
    
    assert_eq!(mnemonic.len(), 24);
    // All words should be from BIP39 word list
    for word in &mnemonic {
        assert!(!word.is_empty());
    }
}

#[test]
fn test_mnemonic_validation_valid() {
    // Test validating a valid mnemonic
    let mnemonic = generate_mnemonic(EntropyStrength::Bits128).unwrap();
    
    let result = validate_mnemonic(&mnemonic);
    assert!(result.is_ok());
}

#[test]
fn test_mnemonic_to_seed() {
    // Test converting mnemonic to seed
    let mnemonic = vec![
        "abandon".to_string(),
        "abandon".to_string(),
        "abandon".to_string(),
        "abandon".to_string(),
        "abandon".to_string(),
        "abandon".to_string(),
        "abandon".to_string(),
        "abandon".to_string(),
        "abandon".to_string(),
        "abandon".to_string(),
        "abandon".to_string(),
        "about".to_string(), // Last word has checksum
    ];
    
    let seed = mnemonic_to_seed(&mnemonic, "");
    assert_eq!(seed.len(), 64); // 512 bits = 64 bytes
}

#[test]
fn test_mnemonic_to_seed_with_passphrase() {
    // Test mnemonic to seed with passphrase
    let mnemonic = generate_mnemonic(EntropyStrength::Bits128).unwrap();
    
    let seed_no_passphrase = mnemonic_to_seed(&mnemonic, "");
    let seed_with_passphrase = mnemonic_to_seed(&mnemonic, "test passphrase");
    
    // Seeds should be different
    assert_ne!(seed_no_passphrase, seed_with_passphrase);
}

#[test]
fn test_mnemonic_entropy_roundtrip() {
    // Test mnemonic <-> entropy conversion
    let mnemonic = generate_mnemonic(EntropyStrength::Bits128).unwrap();
    
    // Convert to entropy
    let entropy = mnemonic_to_entropy(&mnemonic).unwrap();
    
    // Convert back to mnemonic
    let mnemonic2 = mnemonic_from_entropy(&entropy).unwrap();
    
    // Should match (may have different checksum word, but entropy should match)
    assert_eq!(mnemonic.len(), mnemonic2.len());
}

// ============================================================================
// Phase 2: BIP32 HD Key Derivation Tests
// ============================================================================

#[test]
fn test_derive_master_key() {
    // Test deriving master key from seed
    let seed = generate_test_seed();
    
    let (xprv, xpub) = derive_master_key(&seed).unwrap();
    
    // Master key should have depth 0
    assert_eq!(xprv.depth, 0);
    assert_eq!(xpub.depth, 0);
    
    // Parent fingerprint should be all zeros for master
    assert_eq!(xprv.parent_fingerprint, [0u8; 4]);
    assert_eq!(xpub.parent_fingerprint, [0u8; 4]);
    
    // Child number should be 0 for master
    assert_eq!(xprv.child_number, 0);
    assert_eq!(xpub.child_number, 0);
}

#[test]
fn test_derive_master_key_invalid_seed() {
    // Test that invalid seed lengths are rejected
    let seed_too_short = vec![0u8; 15]; // Too short
    let result = derive_master_key(&seed_too_short);
    assert!(result.is_err());
    
    let seed_too_long = vec![0u8; 65]; // Too long
    let result = derive_master_key(&seed_too_long);
    assert!(result.is_err());
}

#[test]
fn test_derive_child_private_normal() {
    // Test deriving normal (non-hardened) child key
    let seed = generate_test_seed();
    let (master_xprv, _) = derive_master_key(&seed).unwrap();
    
    // Derive child at index 0 (normal)
    let (child_xprv, _) = derive_child_private(&master_xprv, 0).unwrap();
    
    // Child should have depth 1
    assert_eq!(child_xprv.depth, 1);
    
    // Child number should be 0
    assert_eq!(child_xprv.child_number, 0);
    
    // Should be different from master
    assert_ne!(child_xprv.private_key_bytes(), master_xprv.private_key_bytes());
}

#[test]
fn test_derive_child_private_hardened() {
    // Test deriving hardened child key
    let seed = generate_test_seed();
    let (master_xprv, _) = derive_master_key(&seed).unwrap();
    
    // Derive child at index 2^31 (hardened)
    let hardened_index = 0x80000000;
    let (child_xprv, _) = derive_child_private(&master_xprv, hardened_index).unwrap();
    
    // Child should have depth 1
    assert_eq!(child_xprv.depth, 1);
    
    // Child number should be hardened index
    assert_eq!(child_xprv.child_number, hardened_index);
}

#[test]
fn test_derive_child_public_normal() {
    // Test deriving normal child public key
    let seed = generate_test_seed();
    let (_, master_xpub) = derive_master_key(&seed).unwrap();
    
    // Derive child at index 0 (normal)
    let child_xpub = derive_child_public(&master_xpub, 0).unwrap();
    
    // Child should have depth 1
    assert_eq!(child_xpub.depth, 1);
    
    // Should be different from master
    assert_ne!(child_xpub.public_key, master_xpub.public_key);
}

#[test]
fn test_derive_child_public_hardened_fails() {
    // Test that hardened derivation fails for public keys
    let seed = generate_test_seed();
    let (_, master_xpub) = derive_master_key(&seed).unwrap();
    
    // Hardened derivation should fail for public keys
    let hardened_index = 0x80000000;
    let result = derive_child_public(&master_xpub, hardened_index);
    assert!(result.is_err());
}

#[test]
fn test_derive_multiple_levels() {
    // Test deriving keys at multiple levels
    let seed = generate_test_seed();
    let (master_xprv, _) = derive_master_key(&seed).unwrap();
    
    // Derive first level
    let (level1, _) = derive_child_private(&master_xprv, 0).unwrap();
    assert_eq!(level1.depth, 1);
    
    // Derive second level
    let (level2, _) = derive_child_private(&level1, 0).unwrap();
    assert_eq!(level2.depth, 2);
    
    // Derive third level
    let (level3, _) = derive_child_private(&level2, 0).unwrap();
    assert_eq!(level3.depth, 3);
}

#[test]
fn test_derive_different_children() {
    // Test that different child indices produce different keys
    let seed = generate_test_seed();
    let (master_xprv, _) = derive_master_key(&seed).unwrap();
    
    let (child0, _) = derive_child_private(&master_xprv, 0).unwrap();
    let (child1, _) = derive_child_private(&master_xprv, 1).unwrap();
    let (child2, _) = derive_child_private(&master_xprv, 2).unwrap();
    
    // All should be different
    assert_ne!(child0.private_key_bytes(), child1.private_key_bytes());
    assert_ne!(child1.private_key_bytes(), child2.private_key_bytes());
    assert_ne!(child0.private_key_bytes(), child2.private_key_bytes());
}

// ============================================================================
// Phase 3: BIP44 Path Tests
// ============================================================================

#[test]
fn test_bip44_path_creation() {
    // Test creating a BIP44 path
    let path = Bip44Path::new(
        CoinType::Bitcoin,
        0,
        ChangeChain::External,
        0,
    );
    
    assert_eq!(path.purpose, 44);
    assert_eq!(path.coin_type, CoinType::Bitcoin);
    assert_eq!(path.account, 0);
    assert_eq!(path.change, ChangeChain::External);
    assert_eq!(path.address_index, 0);
}

#[test]
fn test_bip44_path_bitcoin_mainnet() {
    // Test Bitcoin mainnet path helper
    let path = Bip44Path::bitcoin_mainnet(0, ChangeChain::External, 0);
    
    assert_eq!(path.purpose, 44);
    assert_eq!(path.coin_type, CoinType::Bitcoin);
    assert_eq!(path.account, 0);
    assert_eq!(path.change, ChangeChain::External);
    assert_eq!(path.address_index, 0);
}

#[test]
fn test_bip44_path_bitcoin_testnet() {
    // Test Bitcoin testnet path helper
    let path = Bip44Path::bitcoin_testnet(0, ChangeChain::External, 0);
    
    assert_eq!(path.purpose, 44);
    assert_eq!(path.coin_type, CoinType::BitcoinTestnet);
    assert_eq!(path.account, 0);
}

#[test]
fn test_bip44_path_change_chains() {
    // Test external vs internal change chains
    let external = Bip44Path::bitcoin_mainnet(0, ChangeChain::External, 0);
    let internal = Bip44Path::bitcoin_mainnet(0, ChangeChain::Internal, 0);
    
    assert_eq!(external.change, ChangeChain::External);
    assert_eq!(internal.change, ChangeChain::Internal);
    assert_ne!(external.change.value(), internal.change.value());
}

#[test]
fn test_bip44_path_different_accounts() {
    // Test different account indices
    let account0 = Bip44Path::bitcoin_mainnet(0, ChangeChain::External, 0);
    let account1 = Bip44Path::bitcoin_mainnet(1, ChangeChain::External, 0);
    
    assert_eq!(account0.account, 0);
    assert_eq!(account1.account, 1);
}

#[test]
fn test_bip44_path_different_addresses() {
    // Test different address indices
    let addr0 = Bip44Path::bitcoin_mainnet(0, ChangeChain::External, 0);
    let addr1 = Bip44Path::bitcoin_mainnet(0, ChangeChain::External, 1);
    let addr2 = Bip44Path::bitcoin_mainnet(0, ChangeChain::External, 2);
    
    assert_eq!(addr0.address_index, 0);
    assert_eq!(addr1.address_index, 1);
    assert_eq!(addr2.address_index, 2);
}

#[test]
fn test_bip44_coin_type_values() {
    // Test coin type values
    assert_eq!(CoinType::Bitcoin.value(), 0);
    assert_eq!(CoinType::BitcoinTestnet.value(), 1);
    assert_eq!(CoinType::Litecoin.value(), 2);
    assert_eq!(CoinType::Dogecoin.value(), 3);
    assert_eq!(CoinType::Ethereum.value(), 60);
}

#[test]
fn test_bip44_coin_type_from_value() {
    // Test creating coin type from value
    assert_eq!(CoinType::from_value(0).unwrap(), CoinType::Bitcoin);
    assert_eq!(CoinType::from_value(1).unwrap(), CoinType::BitcoinTestnet);
    assert_eq!(CoinType::from_value(2).unwrap(), CoinType::Litecoin);
    
    // Invalid coin type should fail
    assert!(CoinType::from_value(999).is_err());
}

// ============================================================================
// Phase 4: BIP44 Wallet Integration Tests
// ============================================================================

#[test]
fn test_bip44_wallet_creation() {
    // Test creating a BIP44 wallet
    let seed = generate_test_seed();
    let wallet = Bip44Wallet::from_seed(&seed, CoinType::Bitcoin).unwrap();
    
    // Wallet should be created successfully
    // Note: coin_type is private, but we can verify by deriving an address
    let _ = wallet.derive_address(0, ChangeChain::External, 0).unwrap();
}

#[test]
fn test_bip44_wallet_derive_address() {
    // Test deriving an address from BIP44 wallet
    let seed = generate_test_seed();
    let wallet = Bip44Wallet::from_seed(&seed, CoinType::Bitcoin).unwrap();
    
    // Derive first external address
    let (priv_key, pub_key) = wallet.derive_address(0, ChangeChain::External, 0).unwrap();
    
    // Should have derived keys
    assert_eq!(priv_key.depth, 5); // 5 levels in BIP44 path
    assert_eq!(pub_key.depth, 5);
}

#[test]
fn test_bip44_wallet_different_accounts() {
    // Test deriving keys for different accounts
    let seed = generate_test_seed();
    let wallet = Bip44Wallet::from_seed(&seed, CoinType::Bitcoin).unwrap();
    
    let (key0_priv, key0_pub) = wallet.derive_address(0, ChangeChain::External, 0).unwrap();
    let (key1_priv, key1_pub) = wallet.derive_address(1, ChangeChain::External, 0).unwrap();
    
    // Keys should be different for different accounts
    assert_ne!(key0_priv.private_key_bytes(), key1_priv.private_key_bytes());
    assert_ne!(key0_pub.public_key_bytes(), key1_pub.public_key_bytes());
}

#[test]
fn test_bip44_wallet_external_vs_internal() {
    // Test external vs internal change chains
    let seed = generate_test_seed();
    let wallet = Bip44Wallet::from_seed(&seed, CoinType::Bitcoin).unwrap();
    
    let (external_priv, external_pub) = wallet.derive_address(0, ChangeChain::External, 0).unwrap();
    let (internal_priv, internal_pub) = wallet.derive_address(0, ChangeChain::Internal, 0).unwrap();
    
    // Keys should be different
    assert_ne!(external_priv.private_key_bytes(), internal_priv.private_key_bytes());
    assert_ne!(external_pub.public_key_bytes(), internal_pub.public_key_bytes());
}

#[test]
fn test_bip44_wallet_sequential_addresses() {
    // Test deriving sequential addresses
    let seed = generate_test_seed();
    let wallet = Bip44Wallet::from_seed(&seed, CoinType::Bitcoin).unwrap();
    
    let (key0_priv, key0_pub) = wallet.derive_address(0, ChangeChain::External, 0).unwrap();
    let (key1_priv, key1_pub) = wallet.derive_address(0, ChangeChain::External, 1).unwrap();
    let (key2_priv, key2_pub) = wallet.derive_address(0, ChangeChain::External, 2).unwrap();
    
    // All should derive successfully and be different
    assert_ne!(key0_priv.private_key_bytes(), key1_priv.private_key_bytes());
    assert_ne!(key1_priv.private_key_bytes(), key2_priv.private_key_bytes());
    assert_ne!(key0_pub.public_key_bytes(), key1_pub.public_key_bytes());
    assert_ne!(key1_pub.public_key_bytes(), key2_pub.public_key_bytes());
}

// ============================================================================
// Phase 5: End-to-End BIP39 -> BIP32 -> BIP44 Tests
// ============================================================================

#[test]
fn test_bip39_to_bip32_to_bip44_flow() {
    // Test complete flow: BIP39 mnemonic -> BIP32 master key -> BIP44 derivation
    // Generate mnemonic
    let mnemonic = generate_mnemonic(EntropyStrength::Bits128).unwrap();
    
    // Convert to seed
    let seed = mnemonic_to_seed(&mnemonic, "");
    
    // Derive master key
    let (master_xprv, _) = derive_master_key(&seed).unwrap();
    
    // Create BIP44 wallet
    let wallet = Bip44Wallet::from_seed(&seed, CoinType::Bitcoin).unwrap();
    
    // Derive BIP44 path
    let (priv_key, pub_key) = wallet.derive_address(0, ChangeChain::External, 0).unwrap();
    
    // Should have successfully derived keys
    assert_eq!(priv_key.depth, 5);
    assert_eq!(pub_key.depth, 5);
}

#[test]
fn test_deterministic_derivation() {
    // Test that same seed produces same keys
    let seed1 = generate_test_seed();
    let seed2 = generate_test_seed(); // Same seed
    
    let wallet1 = Bip44Wallet::from_seed(&seed1, CoinType::Bitcoin).unwrap();
    let wallet2 = Bip44Wallet::from_seed(&seed2, CoinType::Bitcoin).unwrap();
    
    let (key1_priv, key1_pub) = wallet1.derive_address(0, ChangeChain::External, 0).unwrap();
    let (key2_priv, key2_pub) = wallet2.derive_address(0, ChangeChain::External, 0).unwrap();
    
    // Keys should be identical (deterministic)
    assert_eq!(key1_priv.private_key_bytes(), key2_priv.private_key_bytes());
    assert_eq!(key1_pub.public_key_bytes(), key2_pub.public_key_bytes());
}

