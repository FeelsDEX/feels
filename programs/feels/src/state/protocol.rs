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
    pub const SIZE: usize = 8 + // discriminator
        32 + // authority
        32 + // treasury
        2 + // default_protocol_fee_rate
        2 + // max_pool_fee_rate
        1 + // paused
        1 + // pool_creation_allowed
        8 + // total_pools
        8 + // total_fees_collected
        8 + // initialized_at
        128; // _reserved
}