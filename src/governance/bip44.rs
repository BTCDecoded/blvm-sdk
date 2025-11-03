//! BIP44: Multi-Account Hierarchy for Deterministic Wallets
//!
//! Specification: https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki
//!
//! Defines standard derivation paths for HD wallets:
//! m / purpose' / coin_type' / account' / change / address_index
//!
//! Example: m/44'/0'/0'/0/0 (Bitcoin mainnet first address)

use crate::governance::bip32::{ExtendedPrivateKey, ExtendedPublicKey, derive_master_key, derive_child_private};
use crate::governance::error::{GovernanceError, GovernanceResult};

/// BIP44 purpose (always 44 for multi-account hierarchy)
pub const BIP44_PURPOSE: u32 = 44;

/// Coin types (BIP44 registered coin types)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoinType {
    /// Bitcoin mainnet
    Bitcoin = 0,
    /// Bitcoin testnet
    BitcoinTestnet = 1,
    /// Litecoin
    Litecoin = 2,
    /// Dogecoin
    Dogecoin = 3,
    /// Ethereum (for reference)
    Ethereum = 60,
}

impl CoinType {
    /// Get coin type value
    pub fn value(&self) -> u32 {
        *self as u32
    }

    /// Create from u32
    pub fn from_value(value: u32) -> Result<Self, GovernanceError> {
        match value {
            0 => Ok(CoinType::Bitcoin),
            1 => Ok(CoinType::BitcoinTestnet),
            2 => Ok(CoinType::Litecoin),
            3 => Ok(CoinType::Dogecoin),
            60 => Ok(CoinType::Ethereum),
            _ => Err(GovernanceError::InvalidInput(
                format!("Unsupported coin type: {}", value)
            )),
        }
    }
}

/// Change chain type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeChain {
    /// External chain (receiving addresses)
    External = 0,
    /// Internal chain (change addresses)
    Internal = 1,
}

impl ChangeChain {
    pub fn value(&self) -> u32 {
        *self as u32
    }
}

/// BIP44 derivation path
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bip44Path {
    /// Purpose (always 44 for BIP44)
    pub purpose: u32,
    /// Coin type (0 = Bitcoin, 1 = Testnet, etc.)
    pub coin_type: CoinType,
    /// Account index
    pub account: u32,
    /// Change chain (0 = external, 1 = internal)
    pub change: ChangeChain,
    /// Address index
    pub address_index: u32,
}

impl Bip44Path {
    /// Create a new BIP44 path
    pub fn new(
        coin_type: CoinType,
        account: u32,
        change: ChangeChain,
        address_index: u32,
    ) -> Self {
        Bip44Path {
            purpose: BIP44_PURPOSE,
            coin_type,
            account,
            change,
            address_index,
        }
    }

    /// Create Bitcoin mainnet path
    pub fn bitcoin_mainnet(account: u32, change: ChangeChain, address_index: u32) -> Self {
        Self::new(CoinType::Bitcoin, account, change, address_index)
    }

    /// Create Bitcoin testnet path
    pub fn bitcoin_testnet(account: u32, change: ChangeChain, address_index: u32) -> Self {
        Self::new(CoinType::BitcoinTestnet, account, change, address_index)
    }

    /// Parse BIP44 path from string (e.g., "m/44'/0'/0'/0/0")
    pub fn from_string(path_str: &str) -> GovernanceResult<Self> {
        // Remove "m/" prefix if present
        let path_str = path_str.strip_prefix("m/").unwrap_or(path_str);
        
        let parts: Vec<&str> = path_str.split('/').collect();
        if parts.len() != 5 {
            return Err(GovernanceError::InvalidInput(
                "BIP44 path must have 5 components: purpose'/coin_type'/account'/change/address_index".to_string()
            ));
        }

        // Parse purpose (should be 44')
        let purpose_str = parts[0].trim_end_matches('\'');
        let purpose: u32 = purpose_str.parse()
            .map_err(|_| GovernanceError::InvalidInput("Invalid purpose".to_string()))?;
        
        if purpose != BIP44_PURPOSE {
            return Err(GovernanceError::InvalidInput(
                format!("Purpose must be {} for BIP44", BIP44_PURPOSE)
            ));
        }

        // Parse coin_type (should be hardened)
        let coin_type_str = parts[1].trim_end_matches('\'');
        let coin_type_val: u32 = coin_type_str.parse()
            .map_err(|_| GovernanceError::InvalidInput("Invalid coin type".to_string()))?;
        let coin_type = CoinType::from_value(coin_type_val)?;

        // Parse account (should be hardened)
        let account_str = parts[2].trim_end_matches('\'');
        let account: u32 = account_str.parse()
            .map_err(|_| GovernanceError::InvalidInput("Invalid account".to_string()))?;

        // Parse change (not hardened)
        let change_val: u32 = parts[3].parse()
            .map_err(|_| GovernanceError::InvalidInput("Invalid change".to_string()))?;
        let change = match change_val {
            0 => ChangeChain::External,
            1 => ChangeChain::Internal,
            _ => return Err(GovernanceError::InvalidInput(
                "Change must be 0 (external) or 1 (internal)".to_string()
            )),
        };

        // Parse address_index (not hardened)
        let address_index: u32 = parts[4].parse()
            .map_err(|_| GovernanceError::InvalidInput("Invalid address index".to_string()))?;

        Ok(Bip44Path {
            purpose,
            coin_type,
            account,
            change,
            address_index,
        })
    }

    /// Convert to string representation (e.g., "m/44'/0'/0'/0/0")
    pub fn to_string(&self) -> String {
        format!(
            "m/{}/{}'/{}'/{}/{}",
            self.purpose,
            self.coin_type.value(),
            self.account,
            self.change.value(),
            self.address_index
        )
    }

    /// Derive key from master key using this path
    pub fn derive(
        &self,
        master_private: &ExtendedPrivateKey,
    ) -> GovernanceResult<(ExtendedPrivateKey, ExtendedPublicKey)> {
        // Build derivation path indices (all hardened for purpose, coin_type, account)
        let indices = vec![
            0x80000000 | self.purpose,      // purpose' (hardened)
            0x80000000 | self.coin_type.value(), // coin_type' (hardened)
            0x80000000 | self.account,     // account' (hardened)
            self.change.value(),            // change (not hardened)
            self.address_index,             // address_index (not hardened)
        ];

        // Derive through path
        let mut current = master_private.clone();
        let mut current_pub = master_private.to_extended_public();

        for &index in &indices {
            let (new_priv, new_pub) = current.derive_child(index)?;
            current = new_priv;
            current_pub = new_pub;
        }

        Ok((current, current_pub))
    }

    /// Get derivation path as vector of indices (for use with BIP32)
    pub fn to_indices(&self) -> Vec<u32> {
        vec![
            0x80000000 | self.purpose,      // purpose' (hardened)
            0x80000000 | self.coin_type.value(), // coin_type' (hardened)
            0x80000000 | self.account,     // account' (hardened)
            self.change.value(),            // change (not hardened)
            self.address_index,             // address_index (not hardened)
        ]
    }
}

/// BIP44 wallet for managing multiple accounts and addresses
pub struct Bip44Wallet {
    /// Master extended private key
    master_private: ExtendedPrivateKey,
    /// Coin type
    coin_type: CoinType,
}

impl Bip44Wallet {
    /// Create a new BIP44 wallet from seed
    pub fn from_seed(seed: &[u8], coin_type: CoinType) -> GovernanceResult<Self> {
        let (master_private, _) = derive_master_key(seed)?;
        Ok(Bip44Wallet {
            master_private,
            coin_type,
        })
    }

    /// Create from existing master key
    pub fn from_master_key(master_private: ExtendedPrivateKey, coin_type: CoinType) -> Self {
        Bip44Wallet {
            master_private,
            coin_type,
        }
    }

    /// Derive key for a specific account, change chain, and address index
    pub fn derive_address(
        &self,
        account: u32,
        change: ChangeChain,
        address_index: u32,
    ) -> GovernanceResult<(ExtendedPrivateKey, ExtendedPublicKey)> {
        let path = Bip44Path::new(self.coin_type, account, change, address_index);
        path.derive(&self.master_private)
    }

    /// Get receiving address (external chain) for account
    pub fn receiving_address(
        &self,
        account: u32,
        address_index: u32,
    ) -> GovernanceResult<(ExtendedPrivateKey, ExtendedPublicKey)> {
        self.derive_address(account, ChangeChain::External, address_index)
    }

    /// Get change address (internal chain) for account
    pub fn change_address(
        &self,
        account: u32,
        address_index: u32,
    ) -> GovernanceResult<(ExtendedPrivateKey, ExtendedPublicKey)> {
        self.derive_address(account, ChangeChain::Internal, address_index)
    }

    /// Get account extended public key (can be shared to watch addresses)
    pub fn account_xpub(&self, account: u32) -> GovernanceResult<ExtendedPublicKey> {
        // Derive to account level: m/44'/coin'/account'
        let path_indices = vec![
            0x80000000 | BIP44_PURPOSE,
            0x80000000 | self.coin_type.value(),
            0x80000000 | account,
        ];

        let mut current = self.master_private.clone();
        for &index in &path_indices {
            let (new_priv, _) = current.derive_child(index)?;
            current = new_priv;
        }

        Ok(current.to_extended_public())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bip44_path_string() {
        let path = Bip44Path::bitcoin_mainnet(0, ChangeChain::External, 0);
        assert_eq!(path.to_string(), "m/44/0'/0'/0/0");
        
        let parsed = Bip44Path::from_string("m/44'/0'/0'/0/0").unwrap();
        assert_eq!(parsed.purpose, 44);
        assert_eq!(parsed.coin_type, CoinType::Bitcoin);
        assert_eq!(parsed.account, 0);
        assert_eq!(parsed.change, ChangeChain::External);
        assert_eq!(parsed.address_index, 0);
    }

    #[test]
    fn test_bip44_path_derivation() {
        let seed = b"test seed for BIP44 derivation";
        let (master_priv, _) = derive_master_key(seed).unwrap();
        
        let path = Bip44Path::bitcoin_mainnet(0, ChangeChain::External, 0);
        let (derived_priv, derived_pub) = path.derive(&master_priv).unwrap();
        
        assert_eq!(derived_priv.depth, 5); // 5 levels: purpose, coin, account, change, address
        assert_eq!(derived_pub.depth, 5);
    }

    #[test]
    fn test_bip44_wallet() {
        let seed = b"test seed for BIP44 wallet";
        let wallet = Bip44Wallet::from_seed(seed, CoinType::Bitcoin).unwrap();
        
        let (receiving_priv, receiving_pub) = wallet.receiving_address(0, 0).unwrap();
        let (change_priv, change_pub) = wallet.change_address(0, 0).unwrap();
        
        // Receiving and change addresses should be different
        assert_ne!(receiving_priv.private_key_bytes(), change_priv.private_key_bytes());
        assert_ne!(receiving_pub.public_key_bytes(), change_pub.public_key_bytes());
    }

    #[test]
    fn test_coin_types() {
        assert_eq!(CoinType::Bitcoin.value(), 0);
        assert_eq!(CoinType::BitcoinTestnet.value(), 1);
        
        let coin = CoinType::from_value(0).unwrap();
        assert_eq!(coin, CoinType::Bitcoin);
    }
}

