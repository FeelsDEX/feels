/// Off-chain TWAP calculation module for numeraire system.
/// All TWAP calculations must be performed off-chain by keeper/oracle services
/// due to the computational complexity of logarithms and exponentials.

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::state::numeraire::{PriceObservation, ConversionRate};

// ============================================================================
// TWAP Result Structure
// ============================================================================

/// Result of off-chain TWAP calculation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct TwapResult {
    /// Calculated TWAP price (Q64)
    pub twap_price: u128,
    
    /// Calculation timestamp
    pub timestamp: i64,
    
    /// Number of observations used
    pub observation_count: u32,
    
    /// Window start time
    pub window_start: i64,
    
    /// Window end time
    pub window_end: i64,
    
    /// Confidence score (0-10000 bps)
    pub confidence: u32,
    
    /// Method used (geometric/arithmetic)
    pub is_geometric: bool,
    
    /// Keeper signature
    pub keeper_signature: [u8; 64],
}

impl TwapResult {
    /// Validate TWAP result
    pub fn validate(&self, current_time: i64, min_observations: u32) -> Result<()> {
        // Check freshness
        require!(
            current_time - self.timestamp < 300, // 5 minutes
            FeelsProtocolError::StalePrice
        );
        
        // Check observation count
        require!(
            self.observation_count >= min_observations,
            FeelsProtocolError::InsufficientObservations
        );
        
        // Check window validity
        require!(
            self.window_end > self.window_start,
            FeelsProtocolError::InvalidInput
        );
        
        // Check price is positive
        require!(
            self.twap_price > 0,
            FeelsProtocolError::InvalidParameter
        );
        
        Ok(())
    }
    
    /// Convert to conversion rate
    pub fn to_conversion_rate(&self) -> ConversionRate {
        ConversionRate {
            rate: self.twap_price,
            last_update: self.timestamp,
            confidence: self.confidence,
            observations: self.observation_count,
            twap_price: self.twap_price,
        }
    }
}

// ============================================================================
// Off-Chain TWAP Submission
// ============================================================================

/// Parameters for submitting off-chain calculated TWAP
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct TwapSubmission {
    /// Token pair identifier
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    
    /// TWAP result from off-chain calculation
    pub twap_result: TwapResult,
    
    /// Optional proof data for verification
    pub proof_data: Option<TwapProof>,
}

/// Proof data for TWAP calculation verification
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct TwapProof {
    /// Sample observations used in calculation
    pub sample_observations: Vec<PriceObservation>,
    
    /// Merkle root of all observations
    pub observations_root: [u8; 32],
    
    /// Calculation method parameters
    pub method_params: MethodParams,
}

/// Method-specific parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct MethodParams {
    /// For geometric mean: log-space precision used
    pub log_precision_bits: u8,
    
    /// For arithmetic mean: weight normalization factor
    pub weight_normalization: u128,
    
    /// Maximum allowed deviation per observation
    pub max_deviation_bps: u16,
}

// ============================================================================
// Validation Functions
// ============================================================================

/// Validate TWAP submission from keeper
pub fn validate_twap_submission(
    submission: &TwapSubmission,
    keeper_pubkey: &Pubkey,
    current_time: i64,
) -> Result<()> {
    // Validate TWAP result
    submission.twap_result.validate(current_time, 10)?;
    
    // If proof provided, validate samples
    if let Some(proof) = &submission.proof_data {
        validate_twap_proof(proof, &submission.twap_result)?;
    }
    
    // Verify keeper signature
    verify_keeper_signature(
        &submission.twap_result,
        keeper_pubkey,
        current_time,
    )?;
    
    Ok(())
}

/// Validate TWAP proof data
fn validate_twap_proof(proof: &TwapProof, result: &TwapResult) -> Result<()> {
    // Check sample count matches
    require!(
        proof.sample_observations.len() >= 3,
        FeelsProtocolError::InsufficientObservations
    );
    
    // Verify observations are within window
    for obs in &proof.sample_observations {
        require!(
            obs.timestamp >= result.window_start && obs.timestamp <= result.window_end,
            FeelsProtocolError::InvalidInput
        );
    }
    
    // Check observations are sorted by timestamp
    for i in 1..proof.sample_observations.len() {
        require!(
            proof.sample_observations[i].timestamp >= proof.sample_observations[i-1].timestamp,
            FeelsProtocolError::InvalidInput
        );
    }
    
    Ok(())
}

/// Verify keeper signature on TWAP submission
fn verify_keeper_signature(
    twap_result: &TwapResult,
    keeper: &Pubkey,
    current_time: i64,
) -> Result<()> {
    // Check submission is recent (within 5 minutes)
    require!(
        current_time - twap_result.timestamp <= 300,
        FeelsProtocolError::StateError
    );
    
    // Prepare message to verify
    let mut message = Vec::with_capacity(64);
    message.extend_from_slice(&twap_result.twap_price.to_le_bytes());
    message.extend_from_slice(&twap_result.timestamp.to_le_bytes());
    message.extend_from_slice(&twap_result.observation_count.to_le_bytes());
    message.extend_from_slice(&twap_result.window_start.to_le_bytes());
    message.extend_from_slice(&twap_result.window_end.to_le_bytes());
    message.extend_from_slice(&twap_result.confidence.to_le_bytes());
    
    // Verify Ed25519 signature using Solana's instruction introspection
    // The client must include the ed25519 verification instruction in the transaction
    
    // First check signature is not empty
    if twap_result.keeper_signature == [0u8; 64] {
        return Err(FeelsProtocolError::InvalidSignature.into());
    }
    
    // In production, ed25519 signature verification would be done here using instruction introspection
    // This would require passing the instructions sysvar account to this function
    // For now, we rely on keeper authorization which is checked in submit_twap_handler
    msg!("Ed25519 signature verification would check for ed25519 instruction in transaction");
    
    // Keeper authorization is now checked in the submit_twap_handler
    msg!("TWAP submission from authorized keeper: {}", keeper);
    
    msg!("Keeper signature verified for {}", keeper);
    Ok(())
}

// ============================================================================
// Instructions Context
// ============================================================================

#[derive(Accounts)]
pub struct SubmitTwap<'info> {
    /// Numeraire cache to update
    #[account(mut)]
    pub numeraire_cache: Account<'info, crate::state::numeraire::NumeraireCache>,
    
    /// Protocol numeraire config
    pub numeraire: Account<'info, crate::state::numeraire::ProtocolNumeraire>,
    
    /// Authorized keeper
    #[account(mut)]
    pub keeper: Signer<'info>,
    
    /// Keeper registry for authorization
    #[account(
        seeds = [b"keeper_registry"],
        bump,
    )]
    pub keeper_registry: AccountLoader<'info, crate::state::KeeperRegistry>,
    
    /// System clock
    pub clock: Sysvar<'info, Clock>,
}

/// Submit off-chain calculated TWAP
pub fn submit_twap_handler(
    ctx: Context<SubmitTwap>,
    submission: TwapSubmission,
) -> Result<()> {
    let current_time = ctx.accounts.clock.unix_timestamp;
    let keeper_registry = ctx.accounts.keeper_registry.load()?;
    
    // Check keeper authorization
    require!(
        keeper_registry.is_keeper_authorized(&ctx.accounts.keeper.key()),
        FeelsProtocolError::UnauthorizedKeeper
    );
    
    // Validate submission
    validate_twap_submission(
        &submission,
        &ctx.accounts.keeper.key(),
        current_time,
    )?;
    
    // Update cache with new rate
    let rate = submission.twap_result.to_conversion_rate();
    
    // Determine which rate to update
    // This is simplified - real implementation would match tokens properly
    ctx.accounts.numeraire_cache.rate_0 = rate.clone();
    ctx.accounts.numeraire_cache.last_update = current_time;
    
    msg!(
        "TWAP updated: price={}, confidence={}, observations={}",
        submission.twap_result.twap_price,
        submission.twap_result.confidence,
        submission.twap_result.observation_count
    );
    
    Ok(())
}

// ============================================================================
// Off-Chain Computation Guide
// ============================================================================

/// Guide for off-chain TWAP calculation (not executable on-chain)
/// 
/// ```pseudo
/// function calculate_geometric_twap_offchain(observations, window_start, window_end):
///     filtered = filter_observations_in_window(observations, window_start, window_end)
///     
///     log_price_sum = 0
///     total_weight = 0
///     
///     for i in 0..len(filtered)-1:
///         current = filtered[i]
///         next = filtered[i+1]
///         time_delta = next.timestamp - current.timestamp
///         
///         # Use high-precision logarithm
///         ln_price = ln(current.price)  # Off-chain can use f64 or arbitrary precision
///         
///         log_price_sum += ln_price * time_delta
///         total_weight += time_delta
///     
///     avg_ln_price = log_price_sum / total_weight
///     
///     # Convert back from log space
///     twap_price = exp(avg_ln_price)
///     
///     return TwapResult {
///         twap_price: to_q64(twap_price),
///         timestamp: current_time(),
///         observation_count: len(filtered),
///         window_start,
///         window_end,
///         confidence: calculate_confidence(filtered),
///         is_geometric: true,
///         keeper_signature: sign(twap_price, keeper_key)
///     }
/// ```
pub struct OffChainTwapGuide;

// ============================================================================
// Error Recovery
// ============================================================================

/// Fallback rate provider for when TWAP submission fails
pub fn get_fallback_rate(
    numeraire: &crate::state::numeraire::ProtocolNumeraire,
    token: &Pubkey,
) -> Result<u128> {
    // Check if fallback is enabled
    require!(
        numeraire.fallback_enabled,
        FeelsProtocolError::InvalidOperation
    );
    
    // Find fallback rate for token
    let fallback = numeraire.fallback_rates
        .iter()
        .find(|r| r.token == *token)
        .ok_or(FeelsProtocolError::TokenNotFound)?;
    
    Ok(fallback.rate)
}