/// Unified market management instruction for all update operations.
/// Consolidates configuration, keeper updates, and pool-derived field updates
/// into a single instruction with consistent validation and authorization.
use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use crate::error::FeelsProtocolError;
use crate::state::{
    MarketField, BufferAccount, MarketDataSource, TwapOracle, ProtocolState,
    UnifiedMarketUpdate,
};
use crate::logic::event::{MarketEvent, MarketEventType, FieldCommitmentEvent};
use crate::logic::field_verification::{verify_market_update_enhanced, FieldVerificationProof};

// ============================================================================
// Market Update Operations
// ============================================================================

/// Unified market operation enum covering all update types
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum MarketOperation {
    /// Administrative configuration of market parameters
    Configure(MarketConfigParams),
    /// External field commitment update from keeper/oracle
    UpdateCommitment(FieldCommitmentUpdate),
    /// Pool-derived field update computed from current state
    UpdateFromPool(PoolUpdateParams),
}

// ============================================================================
// Configuration Parameters
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct MarketConfigParams {
    /// Update domain weights
    pub weights: Option<WeightConfig>,
    
    /// Update risk parameters
    pub risk_params: Option<RiskConfig>,
    
    /// Update buffer configuration
    pub buffer_config: Option<BufferConfig>,
    
    /// Update freshness parameters
    pub freshness: Option<FreshnessConfig>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct WeightConfig {
    /// Spot weight (basis points)
    pub w_s: u32,
    /// Time weight (basis points)
    pub w_t: u32,
    /// Leverage weight (basis points)
    pub w_l: u32,
    /// Buffer weight (basis points)
    pub w_tau: u32,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct RiskConfig {
    /// Price volatility (basis points)
    pub sigma_price: u64,
    /// Rate volatility (basis points)
    pub sigma_rate: u64,
    /// Leverage volatility (basis points)
    pub sigma_leverage: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct BufferConfig {
    /// Participation coefficients (basis points, must sum to 10000)
    pub zeta_spot: u32,
    pub zeta_time: u32,
    pub zeta_leverage: u32,
    
    /// Fee share distribution (basis points, must sum to 10000)
    pub fee_share_spot: u32,
    pub fee_share_time: u32,
    pub fee_share_leverage: u32,
    
    /// Rebate caps
    pub rebate_cap_tx: Option<u64>,
    pub rebate_cap_epoch: Option<u64>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct FreshnessConfig {
    /// Maximum staleness before refresh required (seconds)
    pub max_staleness: i64,
    /// Update frequency requirement (seconds)
    pub update_frequency: i64,
}

// ============================================================================
// Field Commitment Parameters
// ============================================================================

#[allow(non_snake_case)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct FieldCommitmentUpdate {
    /// Market scalars computed off-chain
    pub S: u128,
    pub T: u128, 
    pub L: u128,
    
    /// Domain weights (basis points)
    pub w_s: u32,
    pub w_t: u32,
    pub w_l: u32,
    pub w_tau: u32,
    
    /// Spot value weights (basis points)
    pub omega_0: u32,
    pub omega_1: u32,
    
    /// Risk parameters (basis points)
    pub sigma_price: u64,
    pub sigma_rate: u64,
    pub sigma_leverage: u64,
    
    /// Time-weighted averages
    pub twap_0: u128,
    pub twap_1: u128,
    
    /// Commitment metadata
    pub commitment_hash: [u8; 32],
    pub sequence_number: u64,
    pub expires_at: i64,  // Renamed from validity_period for clarity
    
    /// Optional verification proof for Option B
    pub verification_proof: Option<FieldVerificationProof>,
    
    /// Optional local quadratic coefficients (Option B)
    pub local_coefficients: Option<LocalCoefficients>,
    
    /// Commitment root for verifiable fields
    pub commitment_root: Option<[u8; 32]>,
    
    /// Global bounds
    pub lipschitz_L: u64,
    pub gap_bps: u64,
}

#[allow(non_snake_case)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct LocalCoefficients {
    /// Linear coefficients
    pub c0_s: i128,
    pub c0_t: i128,
    pub c0_l: i128,
    
    /// Quadratic coefficients
    pub c1_s: i128,
    pub c1_t: i128,
    pub c1_l: i128,
    
    /// Validity window
    pub valid_until: i64,
}

// ============================================================================
// Pool Update Parameters  
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PoolUpdateParams {
    /// Update market scalars from pool state
    pub update_scalars: bool,
    /// Update TWAPs from current prices
    pub update_twaps: bool,
    /// Update risk parameters based on market conditions
    pub update_risk: bool,
    /// Force update even if within staleness window
    pub force_update: bool,
}

// ============================================================================
// Accounts Structure
// ============================================================================

#[derive(Accounts)]
#[instruction(operation: MarketOperation)]
pub struct MarketUpdate<'info> {
    /// Market field being updated
    #[account(
        mut,
        seeds = [b"market_field", pool.key().as_ref()],
        bump,
        constraint = market_field.pool == pool.key() @ FeelsProtocolError::StateError
    )]
    pub market_field: Account<'info, MarketField>,
    
    /// Buffer account for market
    #[account(
        mut,
        seeds = [b"buffer", pool.key().as_ref()],
        bump,
        constraint = buffer.pool == pool.key() @ FeelsProtocolError::StateError
    )]
    pub buffer: Account<'info, BufferAccount>,
    
    /// Market data source for authorization and validation
    #[account(
        mut,
        seeds = [b"market_data_source", pool.key().as_ref()],
        bump,
        constraint = market_data_source.load()?.market_field == market_field.key() @ FeelsProtocolError::StateError
    )]
    pub market_data_source: AccountLoader<'info, MarketDataSource>,
    
    /// TWAP oracle for price updates
    #[account(
        mut,
        seeds = [b"twap_oracle", pool.key().as_ref()],
        bump,
        constraint = twap_oracle.load()?.pool == pool.key() @ FeelsProtocolError::StateError
    )]
    pub twap_oracle: AccountLoader<'info, TwapOracle>,
    
    /// Field commitment account for verification
    #[account(
        seeds = [b"field_commitment", pool.key().as_ref()],
        bump,
    )]
    pub field_commitment: AccountLoader<'info, crate::state::FieldCommitment>,
    
    /// Pool reference (not directly modified)
    /// CHECK: Used for PDA derivation and validation only
    pub pool: UncheckedAccount<'info>,
    
    /// Protocol state for admin operations
    #[account(
        seeds = [b"protocol"],
        bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    
    /// Authority performing the update
    pub authority: Signer<'info>,
    
    /// Optional: Token 0 vault account for balance query
    /// CHECK: Optional vault account, validated if provided
    pub vault_0: Option<UncheckedAccount<'info>>,
    
    /// Optional: Token 1 vault account for balance query
    /// CHECK: Optional vault account, validated if provided
    pub vault_1: Option<UncheckedAccount<'info>>,
    
    /// Optional: Token price oracle account
    pub token_price_oracle: Option<AccountLoader<'info, crate::state::TokenPriceOracle>>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

// ============================================================================
// Hash Computation
// ============================================================================

/// Compute deterministic hash of field commitment payload
fn compute_commitment_hash(commitment: &FieldCommitmentUpdate) -> [u8; 32] {
    // Serialize all commitment fields in a deterministic order
    let mut data = Vec::new();
    
    // Market scalars
    data.extend_from_slice(&commitment.S.to_le_bytes());
    data.extend_from_slice(&commitment.T.to_le_bytes());
    data.extend_from_slice(&commitment.L.to_le_bytes());
    
    // Domain weights (in order)
    data.extend_from_slice(&commitment.w_s.to_le_bytes());
    data.extend_from_slice(&commitment.w_t.to_le_bytes());
    data.extend_from_slice(&commitment.w_l.to_le_bytes());
    data.extend_from_slice(&commitment.w_tau.to_le_bytes());
    
    // Spot value weights
    data.extend_from_slice(&commitment.omega_0.to_le_bytes());
    data.extend_from_slice(&commitment.omega_1.to_le_bytes());
    
    // Risk parameters
    data.extend_from_slice(&commitment.sigma_price.to_le_bytes());
    data.extend_from_slice(&commitment.sigma_rate.to_le_bytes());
    data.extend_from_slice(&commitment.sigma_leverage.to_le_bytes());
    
    // TWAPs
    data.extend_from_slice(&commitment.twap_0.to_le_bytes());
    data.extend_from_slice(&commitment.twap_1.to_le_bytes());
    
    // Metadata
    data.extend_from_slice(&commitment.sequence_number.to_le_bytes());
    data.extend_from_slice(&commitment.expires_at.to_le_bytes());
    
    // Optional local coefficients (serialize as 0 if None)
    if let Some(coeffs) = &commitment.local_coefficients {
        data.extend_from_slice(&coeffs.c0_s.to_le_bytes());
        data.extend_from_slice(&coeffs.c0_t.to_le_bytes());
        data.extend_from_slice(&coeffs.c0_l.to_le_bytes());
        data.extend_from_slice(&coeffs.c1_s.to_le_bytes());
        data.extend_from_slice(&coeffs.c1_t.to_le_bytes());
        data.extend_from_slice(&coeffs.c1_l.to_le_bytes());
        data.extend_from_slice(&coeffs.valid_until.to_le_bytes());
    } else {
        // Pad with zeros for consistent hashing
        data.extend_from_slice(&[0u8; 16 * 6 + 8]); // 6 i128s + 1 i64
    }
    
    // Global bounds
    data.extend_from_slice(&commitment.lipschitz_L.to_le_bytes());
    data.extend_from_slice(&commitment.gap_bps.to_le_bytes());
    
    // Compute SHA256 hash
    hash(&data).to_bytes()
}

// ============================================================================
// Validation Logic
// ============================================================================

struct MarketUpdateValidator;


impl MarketUpdateValidator {
    /// Validate weight configuration
    fn validate_weights(weights: &WeightConfig) -> Result<()> {
        let sum = weights.w_s + weights.w_t + weights.w_l + weights.w_tau;
        require!(
            sum == 10000,
            FeelsProtocolError::InvalidWeights
        );
        
        // Ensure all weights are non-zero
        require!(
            weights.w_s > 0 && weights.w_t > 0 && weights.w_l > 0 && weights.w_tau > 0,
            FeelsProtocolError::InvalidAmount
        );
        
        Ok(())
    }
    
    /// Validate buffer configuration
    fn validate_buffer_config(config: &BufferConfig) -> Result<()> {
        // Validate zeta coefficients sum to 10000
        let zeta_sum = config.zeta_spot + config.zeta_time + config.zeta_leverage;
        require!(
            zeta_sum == 10000,
            FeelsProtocolError::InvalidAmount
        );
        
        // Validate fee shares sum to 10000
        let fee_sum = config.fee_share_spot + config.fee_share_time + config.fee_share_leverage;
        require!(
            fee_sum == 10000,
            FeelsProtocolError::InvalidAmount
        );
        
        Ok(())
    }
    
    /// Validate field commitment with enhanced verification
    fn validate_field_commitment(
        commitment: &FieldCommitmentUpdate,
        current_field: &MarketField,
        current_time: i64,
        data_source: &MarketDataSource,
    ) -> Result<[u8; 32]> {
        // Compute deterministic hash of the commitment payload
        let computed_hash = compute_commitment_hash(commitment);
        
        // If a hash was provided, verify it matches
        if commitment.commitment_hash != [0u8; 32] {
            require!(
                commitment.commitment_hash == computed_hash,
                FeelsProtocolError::ValidationError
            );
            msg!("Field commitment hash verified: {:?}", computed_hash);
        } else {
            msg!("Field commitment hash computed: {:?}", computed_hash);
        }
        // Validate sequence number early to reject replays
        require!(
            commitment.sequence_number > data_source.last_sequence,
            FeelsProtocolError::InvalidSequence
        );
        
        let last_sequence = data_source.last_sequence;
        let sequence_number = commitment.sequence_number;
        msg!("Validating field commitment - current sequence: {}, new sequence: {}", 
            last_sequence, sequence_number);
        
        // Create unified update for enhanced verification
        let update = UnifiedMarketUpdate {
            source: crate::state::DATA_SOURCE_TYPE_KEEPER,
            field_commitment: Some(crate::state::FieldCommitmentData {
                S: commitment.S,
                T: commitment.T,
                L: commitment.L,
                w_s: commitment.w_s,
                w_t: commitment.w_t,
                w_l: commitment.w_l,
                w_tau: commitment.w_tau,
                omega_0: commitment.omega_0,
                omega_1: commitment.omega_1,
                twap_0: commitment.twap_0,
                twap_1: commitment.twap_1,
                max_staleness: commitment.expires_at, // expires_at is absolute timestamp
            }),
            price_data: None,
            timestamp: current_time,
            sequence: commitment.sequence_number,
        };
        
        // Use enhanced verification (includes sequence validation)
        verify_market_update_enhanced(&update, data_source, current_field, current_time)?;
        
        // Additional Option B verification if proof provided
        if let Some(proof) = &commitment.verification_proof {
            use crate::logic::field_verification::*;
            
            // Load actual field commitment from account
            let field_commitment_account = ctx.accounts.field_commitment.load()?;
            
            // Verify that the commitment matches what's stored on-chain
            require!(
                field_commitment_account.S == commitment.S &&
                field_commitment_account.T == commitment.T &&
                field_commitment_account.L == commitment.L &&
                field_commitment_account.w_s == commitment.w_s &&
                field_commitment_account.w_t == commitment.w_t &&
                field_commitment_account.w_l == commitment.w_l &&
                field_commitment_account.w_tau == commitment.w_tau,
                FeelsProtocolError::InvalidFieldCommitment
            );
            
            // Use the loaded field commitment for verification
            let field_commitment = crate::state::FieldCommitment {
                pool: field_commitment_account.pool,
                S: field_commitment_account.S,
                T: field_commitment_account.T,
                L: field_commitment_account.L,
                w_s: field_commitment_account.w_s,
                w_t: field_commitment_account.w_t,
                w_l: field_commitment_account.w_l,
                w_tau: field_commitment_account.w_tau,
                omega_0: field_commitment_account.omega_0,
                omega_1: field_commitment_account.omega_1,
                sigma_price: commitment.sigma_price,
                sigma_rate: commitment.sigma_rate,
                sigma_leverage: commitment.sigma_leverage,
                twap_0: commitment.twap_0,
                twap_1: commitment.twap_1,
                snapshot_ts: current_time,
                max_staleness: commitment.expires_at,
                lipschitz_L: commitment.lipschitz_L,
                ..Default::default()
            };
            
            // Verify convex bounds if provided
            if !proof.convex_bound_points.is_empty() {
                verify_convex_bound(proof, &field_commitment)?;
            }
            
            // Verify Lipschitz inequality if constant provided
            if !proof.lipschitz_samples.is_empty() {
                verify_lipschitz_inequality(proof, &field_commitment)?;
            }
            
            // Verify optimality gap
            {
                verify_optimality_gap(proof, commitment.gap_bps)?;
            }
            
            // Verify merkle inclusion if coefficients provided
            if let (Some(coeffs), Some(root), Some(merkle_proof)) = 
                (&commitment.local_coefficients, &commitment.commitment_root, &proof.merkle_proof) 
            {
                // Serialize coefficients for hashing
                let mut coeff_data = Vec::new();
                coeff_data.extend_from_slice(&coeffs.c0_s.to_le_bytes());
                coeff_data.extend_from_slice(&coeffs.c0_t.to_le_bytes());
                coeff_data.extend_from_slice(&coeffs.c0_l.to_le_bytes());
                coeff_data.extend_from_slice(&coeffs.c1_s.to_le_bytes());
                coeff_data.extend_from_slice(&coeffs.c1_t.to_le_bytes());
                coeff_data.extend_from_slice(&coeffs.c1_l.to_le_bytes());
                coeff_data.extend_from_slice(&coeffs.valid_until.to_le_bytes());
                
                verify_merkle_inclusion(&coeff_data, merkle_proof, root)?;
            }
        }
        
        Ok(computed_hash)
    }
    
    /// Helper to validate scalar change rate
    #[allow(dead_code)]
    fn validate_scalar_change(current: u128, new: u128, max_change_bp: u64, _field: &str) -> Result<()> {
        if current == 0 {
            return Ok(()); // Allow any change from zero
        }
        
        let change_ratio = if new > current {
            ((new - current) * 10000) / current
        } else {
            ((current - new) * 10000) / current
        };
        
        require!(
            change_ratio <= max_change_bp as u128,
            FeelsProtocolError::ValidationError
        );
        
        Ok(())
    }
    
    /// Validate authority based on operation type
    fn validate_authority(
        operation: &MarketOperation,
        authority: &Pubkey,
        protocol_state: &ProtocolState,
        market_data_source: &MarketDataSource,
    ) -> Result<()> {
        match operation {
            MarketOperation::Configure(_) => {
                // Configuration requires protocol authority
                require!(
                    *authority == protocol_state.authority,
                    FeelsProtocolError::Unauthorized
                );
            }
            MarketOperation::UpdateCommitment(_) => {
                // Commitment updates require authorized data provider
                require!(
                    *authority == market_data_source.primary_provider || 
                    *authority == market_data_source.secondary_provider,
                    FeelsProtocolError::Unauthorized
                );
            }
            MarketOperation::UpdateFromPool(_) => {
                // Pool updates can be performed by any authorized provider or admin
                require!(
                    *authority == protocol_state.authority ||
                    *authority == market_data_source.primary_provider || 
                    *authority == market_data_source.secondary_provider,
                    FeelsProtocolError::Unauthorized
                );
            }
        }
        
        Ok(())
    }
}

// ============================================================================
// Main Handler
// ============================================================================

pub fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, MarketUpdate<'info>>,
    operation: MarketOperation,
) -> Result<()> {
    let current_time = Clock::get()?.unix_timestamp;
    
    // ========== VALIDATION PHASE ==========
    // All validation happens with immutable references
    // No state mutations occur in this phase
    
    let market_field = &ctx.accounts.market_field;
    let market_data_source = ctx.accounts.market_data_source.load()?;
    
    // Validate authority based on operation type
    MarketUpdateValidator::validate_authority(
        &operation,
        &ctx.accounts.authority.key(),
        &ctx.accounts.protocol_state,
        &market_data_source,
    )?;
    
    // Operation-specific validation
    // Store computed hash for UpdateCommitment operations
    let mut computed_commitment_hash = None;
    match &operation {
        MarketOperation::Configure(config) => {
            if let Some(weights) = &config.weights {
                MarketUpdateValidator::validate_weights(weights)?;
            }
            if let Some(buffer_config) = &config.buffer_config {
                MarketUpdateValidator::validate_buffer_config(buffer_config)?;
            }
        }
        MarketOperation::UpdateCommitment(commitment) => {
            let hash = MarketUpdateValidator::validate_field_commitment(
                commitment,
                &market_field,
                current_time,
                &market_data_source,
            )?;
            computed_commitment_hash = Some(hash);
        }
        MarketOperation::UpdateFromPool(_) => {
            // Pool updates validated against staleness and data source config
            require!(
                market_data_source.is_active != 0,
                FeelsProtocolError::StateError
            );
        }
    }
    
    // Drop immutable references before moving to execution phase
    let _ = market_field;
    let _ = market_data_source;
    
    // ========== EXECUTION PHASE ==========
    // State mutations happen only after all validation passes
    // All validation errors above will prevent reaching this phase
    
    let market_field = &mut ctx.accounts.market_field;
    let mut market_data_source = ctx.accounts.market_data_source.load_mut()?;
    
    match &operation {
        MarketOperation::Configure(config) => {
            // Apply configuration changes
            if let Some(weights) = &config.weights {
                market_field.w_s = weights.w_s;
                market_field.w_t = weights.w_t;
                market_field.w_l = weights.w_l;
                market_field.w_tau = weights.w_tau;
            }
            
            if let Some(risk) = &config.risk_params {
                market_field.sigma_price = risk.sigma_price;
                market_field.sigma_rate = risk.sigma_rate;
                market_field.sigma_leverage = risk.sigma_leverage;
            }
            
            if let Some(buffer_config) = &config.buffer_config {
                ctx.accounts.buffer.zeta_spot = buffer_config.zeta_spot;
                ctx.accounts.buffer.zeta_time = buffer_config.zeta_time;
                ctx.accounts.buffer.zeta_leverage = buffer_config.zeta_leverage;
                ctx.accounts.buffer.fee_share_spot = buffer_config.fee_share_spot;
                ctx.accounts.buffer.fee_share_time = buffer_config.fee_share_time;
                ctx.accounts.buffer.fee_share_leverage = buffer_config.fee_share_leverage;
                
                if let Some(cap_tx) = buffer_config.rebate_cap_tx {
                    ctx.accounts.buffer.rebate_cap_tx = cap_tx;
                }
                if let Some(cap_epoch) = buffer_config.rebate_cap_epoch {
                    ctx.accounts.buffer.rebate_cap_epoch = cap_epoch;
                }
            }
            
            if let Some(freshness) = &config.freshness {
                market_field.max_staleness = freshness.max_staleness;
                market_data_source.update_frequency = freshness.update_frequency;
            }
        }
        
        MarketOperation::UpdateCommitment(commitment) => {
            // Apply field commitment update
            market_field.S = commitment.S;
            market_field.T = commitment.T;
            market_field.L = commitment.L;
            market_field.w_s = commitment.w_s;
            market_field.w_t = commitment.w_t;
            market_field.w_l = commitment.w_l;
            market_field.w_tau = commitment.w_tau;
            market_field.omega_0 = commitment.omega_0;
            market_field.omega_1 = commitment.omega_1;
            market_field.sigma_price = commitment.sigma_price;
            market_field.sigma_rate = commitment.sigma_rate;
            market_field.sigma_leverage = commitment.sigma_leverage;
            market_field.twap_0 = commitment.twap_0;
            market_field.twap_1 = commitment.twap_1;
            market_field.snapshot_ts = current_time;
            
            // Store the computed hash (validated during validation phase)
            market_field.commitment_hash = computed_commitment_hash.unwrap();
            
            // Update data source with sequence number and commitment root
            market_data_source.update_count = commitment.sequence_number;
            market_data_source.last_sequence = commitment.sequence_number;
            
            // Update commitment root if provided
            if let Some(root) = commitment.commitment_root {
                market_data_source.update_commitment(root, commitment.sequence_number)?;
            }
        }
        
        MarketOperation::UpdateFromPool(ref pool_params) => {
            // Update field data from current pool state
            if pool_params.force_update || 
               current_time - market_field.snapshot_ts > market_field.max_staleness {
                
                if pool_params.update_scalars {
                    // Would compute S, T, L from current pool state
                    // Implementation depends on pool state structure
                }
                
                if pool_params.update_twaps {
                    // Would update TWAPs from oracle/pool prices
                    let twap_oracle = ctx.accounts.twap_oracle.load_mut()?;
                    market_field.twap_0 = twap_oracle.price_cumulative_0 / twap_oracle.observation_count.max(1) as u128;
                    market_field.twap_1 = twap_oracle.price_cumulative_1 / twap_oracle.observation_count.max(1) as u128;
                }
                
                if pool_params.update_risk {
                    // Would adjust risk parameters based on market volatility
                    // Implementation depends on volatility calculation
                }
                
                market_field.snapshot_ts = current_time;
            }
        }
    }
    
    // Update data source metadata
    market_data_source.last_update = current_time;
    if matches!(operation, MarketOperation::UpdateCommitment(_)) {
        // Sequence number already updated above
    } else {
        market_data_source.update_count += 1;
    }
    
    // ========== EVENT EMISSION PHASE ==========
    // Emit events after successful state mutations
    
    match &operation {
        MarketOperation::Configure(_) => {
            emit!(MarketEvent {
                market: ctx.accounts.pool.key(),
                event_type: MarketEventType::ConfigUpdated,
                token_0_mint: Pubkey::default(),
                token_1_mint: Pubkey::default(),
                token_0_vault: Pubkey::default(),
                token_1_vault: Pubkey::default(),
                spot_price: market_field.S,
                weights: [market_field.w_s, market_field.w_t, market_field.w_l, market_field.w_tau],
                invariant: 0,
                update_source: 0, // 0=Keeper
                sequence: market_data_source.last_sequence,
                previous_commitment: market_field.commitment_hash,
                timestamp: current_time,
            });
        }
        MarketOperation::UpdateCommitment(commitment) => {
            // Emit both the standard market event and the specific field commitment event
            emit!(MarketEvent {
                market: ctx.accounts.pool.key(),
                event_type: MarketEventType::FieldCommitted,
                token_0_mint: Pubkey::default(),
                token_1_mint: Pubkey::default(),
                token_0_vault: Pubkey::default(),
                token_1_vault: Pubkey::default(),
                spot_price: market_field.S,
                weights: [market_field.w_s, market_field.w_t, market_field.w_l, market_field.w_tau],
                invariant: 0,
                update_source: 0, // 0=Keeper
                sequence: commitment.sequence_number,
                previous_commitment: market_field.commitment_hash,
                timestamp: current_time,
            });
            
            // Emit the field commitment event with hash
            emit!(FieldCommitmentEvent {
                pool: ctx.accounts.pool.key(),
                commitment_hash: computed_commitment_hash.unwrap(),
                sequence_number: commitment.sequence_number,
                S: commitment.S,
                T: commitment.T,
                L: commitment.L,
                gap_bps: commitment.gap_bps,
                lipschitz_L: commitment.lipschitz_L,
                expires_at: commitment.expires_at,
                weights: [commitment.w_s, commitment.w_t, commitment.w_l, commitment.w_tau],
                omega_weights: [commitment.omega_0, commitment.omega_1],
                data_source: ctx.accounts.market_data_source.key(),
                provider: ctx.accounts.authority.key(),
                authority: ctx.accounts.authority.key(),
                timestamp: current_time,
            });
            
            msg!("Field commitment hash: {:?}", computed_commitment_hash.unwrap());
        }
        MarketOperation::UpdateFromPool(_) => {
            emit!(MarketEvent {
                market: ctx.accounts.pool.key(),
                event_type: MarketEventType::FieldUpdated,
                token_0_mint: Pubkey::default(),
                token_1_mint: Pubkey::default(),
                token_0_vault: Pubkey::default(),
                token_1_vault: Pubkey::default(),
                spot_price: market_field.S,
                weights: [market_field.w_s, market_field.w_t, market_field.w_l, market_field.w_tau],
                invariant: 0,
                update_source: 2, // 2=Pool
                sequence: market_data_source.last_sequence,
                previous_commitment: market_field.commitment_hash,
                timestamp: current_time,
            });
        }
    }
    
    msg!("Market update completed successfully");
    msg!("Authority: {}", ctx.accounts.authority.key());
    msg!("Market: {}", ctx.accounts.pool.key());
    
    Ok(())
}

/*
instruction_handler!(
    handler,
    MarketUpdate<'info>,
    MarketOperation,
    (),
    {
        validate: {
            let current_time = Clock::get()?.unix_timestamp;
            let market_field = &ctx.accounts.market_field;
            let market_data_source = ctx.accounts.market_data_source.load()?;
            
            // Validate authority based on operation type
            MarketUpdateValidator::validate_authority(
                &params,
                &ctx.accounts.authority.key(),
                &ctx.accounts.protocol_state,
                &market_data_source,
            )?;
            
            // Operation-specific validation
            match &params {
                MarketOperation::Configure(config) => {
                    if let Some(weights) = &config.weights {
                        MarketUpdateValidator::validate_weights(weights)?;
                    }
                    if let Some(buffer_config) = &config.buffer_config {
                        MarketUpdateValidator::validate_buffer_config(buffer_config)?;
                    }
                }
                MarketOperation::UpdateCommitment(commitment) => {
                    MarketUpdateValidator::validate_field_commitment(
                        commitment,
                        &market_field,
                        current_time,
                    )?;
                }
                MarketOperation::UpdateFromPool(_) => {
                    // Pool updates validated against staleness and data source config
                    require!(
                        market_data_source.is_active,
                        FeelsProtocolError::StateError
                    );
                }
            }
            
            drop(market_field);
            drop(market_data_source);
        },
        
        prepare: {
            // No shared preparation needed - each operation handles its own setup
        },
        
        execute: {
            let current_time = Clock::get()?.unix_timestamp;
            let market_field = &mut ctx.accounts.market_field;
            let mut market_data_source = ctx.accounts.market_data_source.load_mut()?;
            
            // Execute based on operation type
            match params {
                MarketOperation::Configure(config) => {
                    // Apply configuration changes
                    if let Some(weights) = config.weights {
                        market_field.w_s = weights.w_s;
                        market_field.w_t = weights.w_t;
                        market_field.w_l = weights.w_l;
                        market_field.w_tau = weights.w_tau;
                    }
                    
                    if let Some(risk) = config.risk_params {
                        market_field.sigma_price = risk.sigma_price;
                        market_field.sigma_rate = risk.sigma_rate;
                        market_field.sigma_leverage = risk.sigma_leverage;
                    }
                    
                    if let Some(buffer_config) = config.buffer_config {
                        ctx.accounts.buffer.zeta_spot = buffer_config.zeta_spot;
                        ctx.accounts.buffer.zeta_time = buffer_config.zeta_time;
                        ctx.accounts.buffer.zeta_leverage = buffer_config.zeta_leverage;
                        ctx.accounts.buffer.fee_share_spot = buffer_config.fee_share_spot;
                        ctx.accounts.buffer.fee_share_time = buffer_config.fee_share_time;
                        ctx.accounts.buffer.fee_share_leverage = buffer_config.fee_share_leverage;
                        
                        if let Some(cap_tx) = buffer_config.rebate_cap_tx {
                            ctx.accounts.buffer.rebate_cap_tx = cap_tx;
                        }
                        if let Some(cap_epoch) = buffer_config.rebate_cap_epoch {
                            ctx.accounts.buffer.rebate_cap_epoch = cap_epoch;
                        }
                    }
                    
                    if let Some(freshness) = config.freshness {
                        market_field.max_staleness = freshness.max_staleness;
                        market_data_source.update_frequency = freshness.update_frequency;
                    }
                }
                
                MarketOperation::UpdateCommitment(commitment) => {
                    // Apply field commitment update
                    market_field.S = commitment.S;
                    market_field.T = commitment.T;
                    market_field.L = commitment.L;
                    market_field.w_s = commitment.w_s;
                    market_field.w_t = commitment.w_t;
                    market_field.w_l = commitment.w_l;
                    market_field.w_tau = commitment.w_tau;
                    market_field.omega_0 = commitment.omega_0;
                    market_field.omega_1 = commitment.omega_1;
                    market_field.sigma_price = commitment.sigma_price;
                    market_field.sigma_rate = commitment.sigma_rate;
                    market_field.sigma_leverage = commitment.sigma_leverage;
                    market_field.twap_0 = commitment.twap_0;
                    market_field.twap_1 = commitment.twap_1;
                    market_field.snapshot_ts = current_time;
                }
                
                MarketOperation::UpdateFromPool(ref pool_params) => {
                    // Update field data from current pool state
                    // This would involve reading current pool state and computing derived values
                    // For now, update timestamp and validate freshness
                    if pool_params.force_update || 
                       current_time - market_field.snapshot_ts > market_field.max_staleness {
                        
                        if pool_params.update_scalars {
                            // Would compute S, T, L from current pool state
                            // Implementation depends on pool state structure
                        }
                        
                        if pool_params.update_twaps {
                            // Would update TWAPs from oracle/pool prices
                            // Implementation depends on TWAP oracle structure
                        }
                        
                        if pool_params.update_risk {
                            // Would adjust risk parameters based on market volatility
                            // Implementation depends on volatility calculation
                        }
                        
                        market_field.snapshot_ts = current_time;
                    }
                }
            }
            
            // Update data source metadata
            market_data_source.last_update = current_time;
            market_data_source.update_count += 1;
            
            ()
        },
        
        events: {
            let current_time = Clock::get()?.unix_timestamp;
            let market_field = &ctx.accounts.market_field;
            
            let event_type = match &params {
                MarketOperation::Configure(_) => MarketEventType::ConfigUpdated,
                MarketOperation::UpdateCommitment(_) => MarketEventType::FieldCommitted,
                MarketOperation::UpdateFromPool(_) => MarketEventType::FieldUpdated,
            };
            
            emit!(MarketEvent {
                market: ctx.accounts.pool.key(),
                event_type,
                token_0: Pubkey::default(), // Would need pool reference for tokens
                token_1: Pubkey::default(),
                spot_price: market_field.S,
                weights: [market_field.w_s, market_field.w_t, market_field.w_l, market_field.w_tau],
                invariant: 0, // Would compute from current field state
                timestamp: current_time,
            });
        },
        
        finalize: {
            let operation_name = match &params {
                MarketOperation::Configure(_) => "Configuration",
                MarketOperation::UpdateCommitment(_) => "Field commitment",
                MarketOperation::UpdateFromPool(_) => "Pool-derived update",
            };
            
            msg!("Market {} completed successfully", operation_name.to_lowercase());
            msg!("Authority: {}", ctx.accounts.authority.key());
            msg!("Market: {}", ctx.accounts.pool.key());
        }
    }
);
*/

