//! Configuration for the Jupiter adapter
//!
//! This module provides configuration management for the adapter,
//! including protocol parameters needed for fee account derivation.

use solana_program::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::RwLock;
use once_cell::sync::Lazy;

/// Protocol configuration cache
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    /// Treasury pubkey from ProtocolConfig
    pub treasury: Pubkey,
    /// Known protocol token mints
    pub protocol_tokens: Vec<Pubkey>,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            // Default treasury - should be updated with actual value
            treasury: Pubkey::from_str("11111111111111111111111111111111").unwrap(),
            protocol_tokens: Vec::new(),
        }
    }
}

/// Global configuration instance with proper synchronization
pub static ADAPTER_CONFIG: Lazy<RwLock<AdapterConfig>> = Lazy::new(|| {
    RwLock::new(AdapterConfig::default())
});

/// Set the treasury pubkey
pub fn set_treasury(treasury: Pubkey) {
    if let Ok(mut config) = ADAPTER_CONFIG.write() {
        config.treasury = treasury;
    }
}

/// Add a known protocol token
pub fn add_protocol_token(mint: Pubkey) {
    if let Ok(mut config) = ADAPTER_CONFIG.write() {
        if !config.protocol_tokens.contains(&mint) {
            config.protocol_tokens.push(mint);
        }
    }
}

/// Get the protocol treasury ATA for a mint
pub fn get_treasury_ata(mint: &Pubkey) -> Pubkey {
    let config = ADAPTER_CONFIG.read().unwrap();
    spl_associated_token_account::get_associated_token_address(
        &config.treasury,
        mint
    )
}

/// Check if a mint is a protocol token
pub fn is_protocol_token(mint: &Pubkey) -> bool {
    let config = ADAPTER_CONFIG.read().unwrap();
    config.protocol_tokens.contains(mint)
}