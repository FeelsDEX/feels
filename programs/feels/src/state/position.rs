/// Represents individual liquidity positions as NFTs with concentrated liquidity metadata.
/// Tracks position boundaries (tick range), liquidity amount, accumulated fees, and ownership.
/// Each position earns fees proportional to its share of in-range liquidity during swaps.
/// NFT representation enables positions to be transferred, composed, and integrated with DeFi.
///
/// TODO: Future optimization - compress historical position data:
/// - Keep only active position data in NFT metadata
/// - Archive closed/withdrawn positions to merkle tree
/// - Generate proofs for historical fee claims
/// - Enable gas-free position queries via RPC
use anchor_lang::prelude::*;

// ============================================================================
// Tick Position NFT Structure
// ============================================================================

#[account]
pub struct TickPositionMetadata {
    // Tick Position identification
    pub pool: Pubkey,
    pub tick_position_mint: Pubkey,
    pub owner: Pubkey,

    // Range definition
    pub tick_lower: i32,
    pub tick_upper: i32,

    // Liquidity tracking
    pub liquidity: u128,

    // Fee tracking (using [u64; 4] to represent u256)
    pub fee_growth_inside_last_a: [u64; 4],
    pub fee_growth_inside_last_b: [u64; 4],
    pub tokens_owed_a: u64,
    pub tokens_owed_b: u64,

    // Phase 2: Continuous leverage support
    pub leverage: u64,              // 6 decimals (1_000_000 = 1x, 3_000_000 = 3x)
    pub risk_profile_hash: [u8; 8], // Hash of risk profile parameters for verification
    
    // Phase 3: Duration dimension for 3D model
    pub duration: crate::state::duration::Duration, // Time commitment (Flash, Swap, Weekly, etc.)
    pub creation_slot: u64,               // When position was created
    pub maturity_slot: u64,               // When position matures (0 for perpetual)

    // Virtual rebasing checkpoint
    pub rebase_checkpoint: crate::state::rebase::RebaseCheckpoint,

    // Reserved for future extensions
    pub _reserved: [u8; 31],
}

impl TickPositionMetadata {
    // Size breakdown for clarity and maintainability
    const DISCRIMINATOR_SIZE: usize = 8;
    const IDENTIFICATION_SIZE: usize = 32 * 3; // pool, tick_position_mint, owner
    const RANGE_SIZE: usize = 4 * 2; // tick_lower, tick_upper
    const LIQUIDITY_SIZE: usize = 16; // liquidity (u128)
    const FEE_TRACKING_SIZE: usize = 32 * 2 + 8 * 2; // fee_growth_inside_last + tokens_owed
    const LEVERAGE_SIZE: usize = 8 + 8; // leverage + risk_profile_hash
    const DURATION_SIZE: usize = 1 + 8 + 8; // duration enum + creation_slot + maturity_slot
    const REBASE_CHECKPOINT_SIZE: usize = 16 + 16 + 16 + 8; // index_a + index_b + funding_index + timestamp
    const RESERVED_SIZE: usize = 31; // reserved for future upgrades

    pub const SIZE: usize = Self::DISCRIMINATOR_SIZE
        + Self::IDENTIFICATION_SIZE
        + Self::RANGE_SIZE
        + Self::LIQUIDITY_SIZE
        + Self::FEE_TRACKING_SIZE
        + Self::LEVERAGE_SIZE
        + Self::DURATION_SIZE
        + Self::REBASE_CHECKPOINT_SIZE
        + Self::RESERVED_SIZE; // Total: 318 bytes

    /// Calculate hash for risk profile verification
    pub fn calculate_risk_profile_hash(leverage: u64, protection_factor: u64) -> [u8; 8] {
        use anchor_lang::solana_program::hash::hash;
        let data = [leverage.to_le_bytes(), protection_factor.to_le_bytes()].concat();
        let full_hash = hash(&data);
        let mut hash_bytes = [0u8; 8];
        hash_bytes.copy_from_slice(&full_hash.to_bytes()[..8]);
        hash_bytes
    }

    /// Check if position has leverage enabled
    pub fn is_leveraged(&self) -> bool {
        self.leverage > 1_000_000 // Greater than 1x
    }

    /// Get effective liquidity considering leverage
    pub fn effective_liquidity(&self) -> Result<u128> {
        // Calculate leveraged liquidity safely
        let leverage_factor = self.leverage.checked_div(1_000_000).unwrap_or(1);
        self.liquidity
            .checked_mul(leverage_factor as u128)
            .ok_or(error!(crate::state::FeelsProtocolError::MathOverflow))
    }
    
    /// Check if position has matured
    pub fn is_matured(&self, current_slot: u64) -> bool {
        use crate::state::duration::Duration;
        if self.duration == Duration::Swap {
            return true; // Swap positions are always "mature" (no lock)
        }
        
        if self.maturity_slot == 0 {
            // Perpetual positions
            return true;
        }
        
        current_slot >= self.maturity_slot
    }
    
    /// Calculate redenomination priority based on 3D dimensions
    pub fn redenomination_priority(&self, current_tick: i32) -> u64 {
        // Higher leverage = higher priority for losses
        let leverage_score = self.leverage / 1_000; // Scale down to reasonable numbers
        
        // Shorter duration = higher priority (less committed)
        let duration_score = self.duration.protection_priority() as u64;
        
        // Further from current price = lower priority
        let mid_tick = (self.tick_lower + self.tick_upper) / 2;
        let tick_distance = ((current_tick - mid_tick).abs() as u64).saturating_add(1); // Avoid div by 0
        
        // Combined priority: leverage × duration / distance
        leverage_score.saturating_mul(duration_score) / tick_distance
    }
    
    /// Apply virtual rebasing to get current position value
    pub fn apply_virtual_rebase(
        &self,
        base_value_a: u64,
        base_value_b: u64,
        rebase_accumulator: &crate::state::rebase::RebaseAccumulator,
    ) -> Result<(u64, u64)> {
        crate::state::rebase::apply_position_rebase(
            base_value_a,
            base_value_b,
            &self.rebase_checkpoint,
            rebase_accumulator,
            self.is_leveraged(),
            true, // Assume long for now, would need to track this
        )
    }
    
    /// Update checkpoint after claiming yield
    pub fn update_rebase_checkpoint(
        &mut self,
        rebase_accumulator: &crate::state::rebase::RebaseAccumulator,
    ) {
        self.rebase_checkpoint = crate::state::rebase::create_checkpoint(
            rebase_accumulator,
            true, // Assume long for now
        );
    }
}
