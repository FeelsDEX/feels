/// Global protocol state and configuration for the Feels Protocol.
/// Stores protocol-wide parameters including authority, fee settings,
/// and operational flags that govern all pools and operations.

use anchor_lang::prelude::*;

#[account]
pub struct ProtocolState {
    /// Protocol authority that can update parameters
    pub authority: Pubkey,
    
    /// Protocol treasury for collecting fees
    pub treasury: Pubkey,
    
    /// Default protocol fee rate (in basis points, max 10000)
    pub default_protocol_fee_rate: u16,
    
    /// Maximum allowed pool fee rate (in basis points)
    pub max_pool_fee_rate: u16,
    
    /// Whether protocol is paused
    pub paused: bool,
    
    /// Whether new pool creation is allowed
    pub pool_creation_allowed: bool,
    
    /// Total number of pools created
    pub total_pools: u64,
    
    /// Total protocol fees collected in SOL
    pub total_fees_collected: u64,
    
    /// Initialization timestamp
    pub initialized_at: i64,
    
    /// Reserved space for future upgrades
    pub _reserved: [u8; 128],
}

impl ProtocolState {
    // Size breakdown for clarity and maintainability
    const DISCRIMINATOR_SIZE: usize = 8;
    const AUTHORITY_SIZE: usize = 32 + 32;  // authority + treasury
    const FEE_CONFIG_SIZE: usize = 2 + 2;  // default_protocol_fee_rate + max_pool_fee_rate
    const FLAGS_SIZE: usize = 1 + 1;  // paused + pool_creation_allowed
    const STATISTICS_SIZE: usize = 8 + 8;  // total_pools + total_fees_collected
    const METADATA_SIZE: usize = 8;  // initialized_at
    const RESERVED_SIZE: usize = 128;  // reserved for future upgrades
    
    pub const SIZE: usize = Self::DISCRIMINATOR_SIZE +
        Self::AUTHORITY_SIZE +
        Self::FEE_CONFIG_SIZE +
        Self::FLAGS_SIZE +
        Self::STATISTICS_SIZE +
        Self::METADATA_SIZE +
        Self::RESERVED_SIZE;  // Total: 228 bytes
}