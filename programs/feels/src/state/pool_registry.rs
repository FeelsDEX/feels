//! Pool Registry for discovery and tracking
//!
//! Registry that tracks all pools in the protocol,
//! enabling discovery and indexing of Feels markets.

use anchor_lang::prelude::*;

/// Pool phase for lifecycle tracking
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum PoolPhase {
    /// Initial bonding curve phase
    BondingCurve = 0,
    /// Graduated to steady state
    SteadyState = 1,
    /// Paused by governance
    Paused = 2,
    /// Deprecated/closed
    Deprecated = 3,
}

impl Default for PoolPhase {
    fn default() -> Self {
        Self::BondingCurve
    }
}

/// Registry entry for a single pool
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct PoolEntry {
    /// Market pubkey
    pub market: Pubkey,
    /// Project token mint
    pub token_mint: Pubkey,
    /// FeelsSOL mint (always token_0 in markets)
    pub feelssol_mint: Pubkey,
    /// Current phase
    pub phase: PoolPhase,
    /// Creation timestamp
    pub created_at: i64,
    /// Last update timestamp
    pub updated_at: i64,
    /// Creator/launcher
    pub creator: Pubkey,
    /// Token symbol (up to 10 chars)
    pub symbol: [u8; 10],
    /// Symbol length
    pub symbol_len: u8,
    /// Reserved for future use
    pub _reserved: [u8; 32],
}

impl PoolEntry {
    pub const LEN: usize = 32 + // market
        32 + // token_mint
        32 + // feelssol_mint
        1 + // phase
        8 + // created_at
        8 + // updated_at
        32 + // creator
        10 + // symbol
        1 + // symbol_len
        32; // _reserved

    /// Get symbol as string
    pub fn symbol(&self) -> String {
        String::from_utf8_lossy(&self.symbol[..self.symbol_len as usize]).to_string()
    }
}

/// Central pool registry
#[account]
pub struct PoolRegistry {
    /// Protocol authority
    pub authority: Pubkey,
    /// Number of pools registered
    pub pool_count: u64,
    /// Pools array (paginated access)
    pub pools: Vec<PoolEntry>,
    /// Canonical bump
    pub bump: u8,
    /// Reserved for future use
    pub _reserved: [u8; 128],
}

impl PoolRegistry {
    pub const SEED: &'static [u8] = b"pool_registry";

    /// Initial allocation size (can grow dynamically)
    pub const INITIAL_SIZE: usize = 8 + // discriminator
        32 + // authority
        8 + // pool_count
        4 + // vec length
        1 + // bump
        128; // _reserved

    /// Space for one pool entry in realloc
    pub const POOL_ENTRY_SIZE: usize = PoolEntry::LEN + 4; // +4 for vec overhead

    /// Find pool by token mint
    pub fn find_pool(&self, token_mint: &Pubkey) -> Option<&PoolEntry> {
        self.pools.iter().find(|p| p.token_mint == *token_mint)
    }

    /// Find pool by market
    pub fn find_pool_by_market(&self, market: &Pubkey) -> Option<&PoolEntry> {
        self.pools.iter().find(|p| p.market == *market)
    }

    /// Add new pool
    pub fn add_pool(&mut self, entry: PoolEntry) -> Result<()> {
        // Check for duplicates
        require!(
            self.find_pool(&entry.token_mint).is_none(),
            crate::error::FeelsError::PoolAlreadyExists
        );

        self.pools.push(entry);
        self.pool_count = self.pool_count.saturating_add(1);
        Ok(())
    }

    /// Update pool phase
    pub fn update_pool_phase(
        &mut self,
        market: &Pubkey,
        new_phase: PoolPhase,
        timestamp: i64,
    ) -> Result<()> {
        let pool = self
            .pools
            .iter_mut()
            .find(|p| p.market == *market)
            .ok_or(crate::error::FeelsError::PoolNotFound)?;

        pool.phase = new_phase;
        pool.updated_at = timestamp;
        Ok(())
    }
}
