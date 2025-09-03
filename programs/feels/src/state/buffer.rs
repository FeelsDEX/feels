/// Buffer account (τ) for fee collection and rebate distribution in the market physics model.
/// The buffer receives all fees and provides bounded rebates while participating in conservation laws.
use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Maximum rebate per transaction (basis points of transaction value)
pub const MAX_REBATE_PER_TX_BPS: u64 = 100; // 1%

/// Maximum rebate per epoch (basis points of buffer value)
pub const MAX_REBATE_PER_EPOCH_BPS: u64 = 1000; // 10%

/// EWMA half-life for fee tracking (seconds)
pub const FEE_EWMA_HALF_LIFE: i64 = 86400; // 24 hours

/// Basis points denominator
pub const BPS_DENOMINATOR: u64 = 10_000;

// ============================================================================
// Buffer Account
// ============================================================================

/// Buffer account managing fees and rebates
#[account]
pub struct BufferAccount {
    /// Pool this buffer belongs to
    pub pool: Pubkey,
    
    // ========== Buffer Value ==========
    
    /// Current buffer value in numeraire N
    pub tau_value: u128,
    
    /// Reserved buffer value (cannot be used for rebates)
    pub tau_reserved: u128,
    
    // ========== Participation Coefficients ==========
    
    /// Spot dimension participation coefficient [0, 10000]
    pub zeta_spot: u32,
    
    /// Time dimension participation coefficient [0, 10000]
    pub zeta_time: u32,
    
    /// Leverage dimension participation coefficient [0, 10000]
    pub zeta_leverage: u32,
    
    // ========== Fee Tracking (EWMA) ==========
    
    /// Fee share from spot dimension (basis points)
    pub fee_share_spot: u32,
    
    /// Fee share from time dimension (basis points)
    pub fee_share_time: u32,
    
    /// Fee share from leverage dimension (basis points)
    pub fee_share_leverage: u32,
    
    /// Last fee share update timestamp
    pub fee_share_last_update: i64,
    
    // ========== Rebate Configuration ==========
    
    /// Maximum rebate per transaction (absolute value)
    pub rebate_cap_tx: u64,
    
    /// Maximum rebate per epoch (absolute value)
    pub rebate_cap_epoch: u64,
    
    /// Current epoch rebates paid
    pub rebate_paid_epoch: u64,
    
    /// Current epoch start timestamp
    pub epoch_start: i64,
    
    /// Epoch duration (seconds)
    pub epoch_duration: i64,
    
    /// Rebate participation rate η [0, 10000]
    pub rebate_eta: u32,
    
    /// Price improvement clamp factor κ [0, 10000]
    pub kappa: u32,
    
    // ========== Statistics ==========
    
    /// Total fees collected (all-time)
    pub total_fees_collected: u128,
    
    /// Total rebates paid (all-time)
    pub total_rebates_paid: u128,
    
    /// Last update timestamp
    pub last_update: i64,
    
    // ========== Administration ==========
    
    /// Authority that can update buffer parameters
    pub authority: Pubkey,
    
    /// Protocol fee recipient
    pub protocol_fee_recipient: Pubkey,
    
    /// Reserved for future use
    pub _reserved: [u8; 64],
}

impl Default for BufferAccount {
    fn default() -> Self {
        Self {
            pool: Pubkey::default(),
            tau_value: 0,
            tau_reserved: 0,
            zeta_spot: 3333,
            zeta_time: 3333,
            zeta_leverage: 3334,
            fee_share_spot: 0,
            fee_share_time: 0,
            fee_share_leverage: 0,
            fee_share_last_update: 0,
            rebate_cap_tx: 0,
            rebate_cap_epoch: 0,
            rebate_paid_epoch: 0,
            epoch_start: 0,
            epoch_duration: 3600,
            rebate_eta: 5000,
            kappa: 5000,
            total_fees_collected: 0,
            total_rebates_paid: 0,
            last_update: 0,
            authority: Pubkey::default(),
            protocol_fee_recipient: Pubkey::default(),
            _reserved: [0u8; 64],
        }
    }
}

impl BufferAccount {
    /// Initialize buffer with default parameters
    pub fn initialize(&mut self, pool: Pubkey, epoch_duration: i64) {
        self.pool = pool;
        self.epoch_duration = epoch_duration;
        self.epoch_start = Clock::get().unwrap().unix_timestamp;
        
        // Default participation coefficients (equal participation)
        self.zeta_spot = 3333;
        self.zeta_time = 3333;
        self.zeta_leverage = 3334;
        
        // Default rebate configuration
        self.rebate_eta = 5000; // 50% participation
        self.kappa = 1000; // 10% price improvement clamp
        self.rebate_cap_tx = u64::MAX;
        self.rebate_cap_epoch = u64::MAX;
    }
    
    /// Update fee shares using EWMA
    pub fn update_fee_shares(
        &mut self,
        spot_fees: u64,
        time_fees: u64,
        leverage_fees: u64,
        current_time: i64,
    ) -> Result<()> {
        let total_fees = spot_fees
            .checked_add(time_fees)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_add(leverage_fees)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        if total_fees == 0 {
            return Ok(());
        }
        
        // Calculate new shares
        let new_spot_share = (spot_fees as u128 * BPS_DENOMINATOR as u128 / total_fees as u128) as u32;
        let new_time_share = (time_fees as u128 * BPS_DENOMINATOR as u128 / total_fees as u128) as u32;
        let new_leverage_share = BPS_DENOMINATOR as u32 - new_spot_share - new_time_share;
        
        // Apply EWMA if not first update
        if self.fee_share_last_update > 0 {
            let time_diff = current_time - self.fee_share_last_update;
            let decay = calculate_ewma_decay(time_diff, FEE_EWMA_HALF_LIFE)?;
            
            self.fee_share_spot = apply_ewma(self.fee_share_spot, new_spot_share, decay)?;
            self.fee_share_time = apply_ewma(self.fee_share_time, new_time_share, decay)?;
            self.fee_share_leverage = apply_ewma(self.fee_share_leverage, new_leverage_share, decay)?;
        } else {
            self.fee_share_spot = new_spot_share;
            self.fee_share_time = new_time_share;
            self.fee_share_leverage = new_leverage_share;
        }
        
        self.fee_share_last_update = current_time;
        Ok(())
    }
    
    /// Calculate local weights for conservation law
    pub fn calculate_local_weights(&self, domain_weights: &crate::state::DomainWeights) -> (u32, u32, u32) {
        // w_tau^(spot) = ζ_spot * φ_spot * w_tau
        let w_tau_spot = (self.zeta_spot as u64 * self.fee_share_spot as u64 * domain_weights.w_tau as u64 
            / (BPS_DENOMINATOR * BPS_DENOMINATOR)) as u32;
        
        let w_tau_time = (self.zeta_time as u64 * self.fee_share_time as u64 * domain_weights.w_tau as u64 
            / (BPS_DENOMINATOR * BPS_DENOMINATOR)) as u32;
        
        let w_tau_leverage = (self.zeta_leverage as u64 * self.fee_share_leverage as u64 * domain_weights.w_tau as u64 
            / (BPS_DENOMINATOR * BPS_DENOMINATOR)) as u32;
        
        (w_tau_spot, w_tau_time, w_tau_leverage)
    }
    
    /// Collect fees into buffer
    pub fn collect_fees(&mut self, fee_amount: u128) -> Result<()> {
        self.tau_value = self.tau_value
            .checked_add(fee_amount)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        self.total_fees_collected = self.total_fees_collected
            .checked_add(fee_amount)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        Ok(())
    }
    
    /// Pay rebate from buffer
    pub fn pay_rebate(&mut self, rebate_amount: u64, current_time: i64) -> Result<u64> {
        // Check epoch boundary
        if current_time >= self.epoch_start + self.epoch_duration {
            self.start_new_epoch(current_time);
        }
        
        // Apply all rebate caps
        let available_tau = self.tau_value
            .saturating_sub(self.tau_reserved);
        
        let capped_rebate = rebate_amount
            .min(self.rebate_cap_tx)
            .min(self.rebate_cap_epoch.saturating_sub(self.rebate_paid_epoch))
            .min(available_tau as u64);
        
        // Update buffer state
        self.tau_value = self.tau_value
            .checked_sub(capped_rebate as u128)
            .ok_or(FeelsProtocolError::InsufficientBuffer)?;
        
        self.rebate_paid_epoch = self.rebate_paid_epoch
            .checked_add(capped_rebate)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        self.total_rebates_paid = self.total_rebates_paid
            .checked_add(capped_rebate as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        Ok(capped_rebate)
    }
    
    /// Start a new epoch
    fn start_new_epoch(&mut self, current_time: i64) {
        self.epoch_start = current_time;
        self.rebate_paid_epoch = 0;
    }
    
    /// Update rebate caps
    pub fn update_rebate_caps(
        &mut self,
        cap_tx: Option<u64>,
        cap_epoch: Option<u64>,
        eta: Option<u32>,
        kappa: Option<u32>,
    ) -> Result<()> {
        if let Some(cap) = cap_tx {
            self.rebate_cap_tx = cap;
        }
        
        if let Some(cap) = cap_epoch {
            self.rebate_cap_epoch = cap;
        }
        
        if let Some(e) = eta {
            require!(e <= BPS_DENOMINATOR as u32, FeelsProtocolError::InvalidInput);
            self.rebate_eta = e;
        }
        
        if let Some(k) = kappa {
            require!(k <= BPS_DENOMINATOR as u32, FeelsProtocolError::InvalidInput);
            self.kappa = k;
        }
        
        Ok(())
    }
    
    /// Update participation coefficients
    pub fn update_participation_coefficients(
        &mut self,
        zeta_spot: Option<u32>,
        zeta_time: Option<u32>,
        zeta_leverage: Option<u32>,
    ) -> Result<()> {
        if let Some(z) = zeta_spot {
            require!(z <= BPS_DENOMINATOR as u32, FeelsProtocolError::InvalidInput);
            self.zeta_spot = z;
        }
        
        if let Some(z) = zeta_time {
            require!(z <= BPS_DENOMINATOR as u32, FeelsProtocolError::InvalidInput);
            self.zeta_time = z;
        }
        
        if let Some(z) = zeta_leverage {
            require!(z <= BPS_DENOMINATOR as u32, FeelsProtocolError::InvalidInput);
            self.zeta_leverage = z;
        }
        
        Ok(())
    }
    
    /// Get available tau for rebates
    pub fn get_available_tau(&self) -> Result<u64> {
        let available = self.tau_value.saturating_sub(self.tau_reserved);
        Ok(available.min(u64::MAX as u128) as u64)
    }
}

// ============================================================================
// Size Constants
// ============================================================================

impl BufferAccount {
    pub const SIZE: usize = 8 +  // discriminator
        32 +                      // pool pubkey
        16 +                      // tau_value
        16 +                      // tau_reserved
        4 * 3 +                   // zeta coefficients
        4 * 3 +                   // fee shares
        8 +                       // fee_share_last_update
        8 * 2 +                   // rebate caps
        8 +                       // rebate_paid_epoch
        8 +                       // epoch_start
        8 +                       // epoch_duration
        4 +                       // rebate_eta
        4 +                       // kappa
        16 +                      // total_fees_collected
        16 +                      // total_rebates_paid
        8 +                       // last_update
        32 +                      // authority
        32 +                      // protocol_fee_recipient
        64;                       // reserved
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate EWMA decay factor
fn calculate_ewma_decay(time_diff: i64, half_life: i64) -> Result<u32> {
    if time_diff <= 0 {
        return Ok(BPS_DENOMINATOR as u32);
    }
    
    // decay = 2^(-time_diff / half_life)
    // Approximation: decay ≈ 1 - (ln(2) * time_diff / half_life)
    let ln2_bps = 6931; // ln(2) * 10000
    let decay_reduction = (ln2_bps as i64 * time_diff / half_life) as u32;
    
    Ok(BPS_DENOMINATOR as u32 - decay_reduction.min(BPS_DENOMINATOR as u32))
}

/// Apply EWMA update
fn apply_ewma(old_value: u32, new_value: u32, decay: u32) -> Result<u32> {
    // result = decay * old + (1 - decay) * new
    let weighted_old = (old_value as u64 * decay as u64) / BPS_DENOMINATOR;
    let weighted_new = (new_value as u64 * (BPS_DENOMINATOR as u32 - decay) as u64) / BPS_DENOMINATOR;
    
    Ok((weighted_old + weighted_new) as u32)
}

// ============================================================================
// Rebate Calculation
// ============================================================================

/// Price improvement data for fee/rebate calculation
#[derive(Clone, Debug, Default)]
pub struct PriceImprovement {
    /// Oracle/reference price (Q64)
    pub oracle_price: u128,
    /// Execution price (Q64)
    pub execution_price: u128,
    /// Price improvement amount (basis points)
    pub improvement_bps: u64,
    /// Is this a buy order (affects improvement calculation)
    pub is_buy: bool,
}

/// Calculate instantaneous fee with price improvement clamp
/// Returns (fee_amount, rebate_amount) where one will always be zero
pub fn calculate_instantaneous_fee(
    work: i128,
    price_improvement: &PriceImprovement,
    buffer: &BufferAccount,
) -> Result<(u64, u64)> {
    // Calculate κ * price_improvement
    let kappa_improvement = (buffer.kappa as u128)
        .checked_mul(price_improvement.improvement_bps as u128)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    // Calculate fee = max(0, W - κ * price_improvement)
    if work > 0 {
        let work_u128 = work as u128;
        let clamped_work = work_u128.saturating_sub(kappa_improvement);
        
        // Convert to token units (work is already in appropriate units from SDK)
        let fee = clamped_work.min(u64::MAX as u128) as u64;
        
        Ok((fee, 0))
    } else {
        // Negative work case - calculate rebate
        let negative_work = (-work) as u128;
        
        // Apply η participation rate
        let rebate_base = negative_work
            .checked_mul(buffer.rebate_eta as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(BPS_DENOMINATOR as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        // Add price improvement bonus
        let rebate_total = rebate_base
            .checked_add(kappa_improvement)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .min(u64::MAX as u128) as u64;
        
        Ok((0, rebate_total))
    }
}

/// Calculate price improvement for an order
pub fn calculate_price_improvement(
    oracle_price: u128,
    execution_price: u128,
    is_buy: bool,
) -> PriceImprovement {
    let improvement_bps = if is_buy {
        // For buys: improvement when execution price < oracle price
        if execution_price < oracle_price {
            let diff = oracle_price - execution_price;
            ((diff * BPS_DENOMINATOR as u128) / oracle_price) as u64
        } else {
            0
        }
    } else {
        // For sells: improvement when execution price > oracle price
        if execution_price > oracle_price {
            let diff = execution_price - oracle_price;
            ((diff * BPS_DENOMINATOR as u128) / oracle_price) as u64
        } else {
            0
        }
    };
    
    PriceImprovement {
        oracle_price,
        execution_price,
        improvement_bps,
        is_buy,
    }
}

/// Legacy rebate calculation (deprecated - use calculate_instantaneous_fee)
pub fn calculate_rebate(
    negative_work: u128,
    price_map: u128,
    buffer: &BufferAccount,
) -> Result<u64> {
    // R* = -W * Π(P) * η
    let rebate_star = negative_work
        .checked_mul(price_map).ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(crate::constant::Q64).ok_or(FeelsProtocolError::MathOverflow)?
        .checked_mul(buffer.rebate_eta as u128).ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR as u128).ok_or(FeelsProtocolError::MathOverflow)?;
    
    // Check overflow
    require!(
        rebate_star <= u64::MAX as u128,
        FeelsProtocolError::MathOverflow
    );
    
    Ok(rebate_star as u64)
}

// ============================================================================
// Instructions Context
// ============================================================================

#[derive(Accounts)]
pub struct InitializeBuffer<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<BufferAccount>(),
        seeds = [b"buffer", market_field.key().as_ref()],
        bump
    )]
    pub buffer: Account<'info, BufferAccount>,
    
    pub market_field: Account<'info, crate::state::MarketField>,
    
    #[account(mut)]
    pub payer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateBufferParams<'info> {
    #[account(mut)]
    pub buffer: Account<'info, BufferAccount>,
    
    pub market_field: Account<'info, crate::state::MarketField>,
    
    pub authority: Signer<'info>,
}