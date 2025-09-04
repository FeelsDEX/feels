use crate::state::duration::Duration;
use crate::logic::order_system::HubRoute;
/// Centralized event definitions and aggregation utilities for protocol analytics and monitoring.
/// Contains all protocol event structures and aggregation logic for TWAP/VWAP calculations,
/// volume tracking, and rate analytics. Essential for off-chain indexing and analysis.
use anchor_lang::prelude::*;

// ============================================================================
// Event Data Types
// ============================================================================

/// Hook data for event emission
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct HookData {
    pub pre_swap_price: u128,
    pub post_swap_price: u128,
    pub price_impact_bps: u16,
    pub volume: u64,
}

// ============================================================================
// Core Event Infrastructure
// ============================================================================

/// Base trait for all protocol events
pub trait EventBase {
    fn pool(&self) -> Pubkey;
    fn timestamp(&self) -> i64;
    fn actor(&self) -> Pubkey;
}

// ============================================================================
// Event Type Definitions
// ============================================================================

/// Liquidity event types
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum LiquidityEventType {
    Add,
    Remove,
}

/// Market event types
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum MarketEventType {
    Initialized,
    Configured,
    Updated,
    Paused,
    Resumed,
    ConfigUpdated,
    FieldCommitted,
    FieldUpdated,
}

// ============================================================================
// Market Events
// ============================================================================

/// Emitted when market state changes  
#[event]
pub struct MarketEvent {
    #[index]
    pub market: Pubkey,
    pub event_type: MarketEventType,
    
    // Token information for audit trail
    pub token_0_mint: Pubkey,      // Actual token mint A
    pub token_1_mint: Pubkey,      // Actual token mint B
    pub token_0_vault: Pubkey,     // Token 0 vault account
    pub token_1_vault: Pubkey,     // Token 1 vault account
    
    // Market state snapshot
    pub spot_price: u128,
    pub weights: [u32; 4],
    pub invariant: u128,
    
    // Update metadata for audit chain
    pub update_source: u8,         // 0=Keeper, 1=Oracle, 2=Pool 
    pub sequence: u64,             // Update sequence number
    pub previous_commitment: [u8; 32], // Previous commitment hash
    
    pub timestamp: i64,
}

impl EventBase for MarketEvent {
    fn pool(&self) -> Pubkey {
        self.market
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.market // Market events don't have a specific actor
    }
}

// ============================================================================
// Oracle Update Events
// ============================================================================

/// Emitted when oracle updates market parameters
#[event]
pub struct OracleUpdateEvent {
    #[index]
    pub pool: Pubkey,
    pub oracle: Pubkey,
    pub timestamp: i64,
    pub spot_gradient: i64,
    pub rate_gradient: i64,
    pub leverage_gradient: i64,
    pub market_curvature: u64,
    pub risk_adjustment: u32,
    pub volatility: u32,
}

impl EventBase for OracleUpdateEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    
    fn actor(&self) -> Pubkey {
        self.oracle
    }
}

// ============================================================================
// Protocol Initialization Events
// ============================================================================

/// Emitted when a new pool is initialized
#[event]
pub struct PoolInitialized {
    #[index]
    pub pool: Pubkey,
    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub fee_rate: u16,
    pub tick_spacing: i16,
    pub initial_sqrt_price: u128,
    pub authority: Pubkey,
    pub feelssol_mint: Pubkey,
    pub timestamp: i64,
}

impl EventBase for PoolInitialized {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.authority
    }
}

/// Emitted when FeelsSOL is initialized
#[event]
pub struct FeelsSOLInitialized {
    pub feelssol: Pubkey,
    pub underlying_mint: Pubkey,
    pub feels_mint: Pubkey,
    pub vault: Pubkey,
    pub initial_exchange_rate: u64,
    pub timestamp: i64,
}

// ============================================================================
// Trading Events
// ============================================================================

/// Emitted when a swap is executed
#[event]
pub struct SwapEvent {
    #[index]
    pub pool: Pubkey,
    pub user: Pubkey,
    
    // Token information for audit trail
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub zero_for_one: bool,
    
    // Amounts and fees
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee: u64,
    pub protocol_fee: u64,
    
    // Market impact
    pub sqrt_price_before: u128,
    pub sqrt_price_after: u128,
    pub tick_before: i32,
    pub tick_after: i32,
    
    // Work calculation data
    pub work_computed: i128,
    pub field_commitment_used: [u8; 32],
    
    pub timestamp: i64,
}

impl EventBase for SwapEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.user
    }
}

/// Emitted when a cross-token swap is executed (multi-hop)
#[event]
pub struct CrossTokenSwapEvent {
    #[index]
    pub user: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub route: HubRoute,
    pub intermediate_amount: Option<u64>,    // For two-hop swaps
    pub sqrt_price_after_hop1: Option<u128>,  // Rate after first hop
    pub sqrt_price_after_final: u128,         // Final rate state
    pub tick_after_hop1: Option<i32>,        // Tick after first hop
    pub tick_after_final: i32,               // Final tick state
    pub total_fees_paid: u64,                // Sum of all fees across hops
    pub protocol_fees_collected: u64,        // Protocol fees from all hops
    pub gas_used_estimate: u64,              // Estimated compute units used
    pub timestamp: i64,
}

impl EventBase for CrossTokenSwapEvent {
    fn pool(&self) -> Pubkey {
        if self.route.pools.is_empty() {
            Pubkey::default()
        } else {
            self.route.pools[0] // Return first pool
        }
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.user
    }
}

// ============================================================================
// Liquidity Management Events
// ============================================================================

/// Event emitted when liquidity changes
#[event]
pub struct LiquidityEvent {
    #[index]
    pub pool: Pubkey,
    pub position: Pubkey,
    pub liquidity_delta: i128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub event_type: LiquidityEventType,
    pub timestamp: i64,
}

impl EventBase for LiquidityEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.position
    }
}

// ============================================================================
// Fee Collection Events
// ============================================================================

/// Emitted when pool fees are collected
#[event]
pub struct FeeCollectionEvent {
    #[index]
    pub pool: Pubkey,
    pub position: Pubkey,
    pub amount_0: u64,
    pub amount_1: u64,
    pub timestamp: i64,
}

impl EventBase for FeeCollectionEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.position
    }
}

/// Emitted when protocol fees are collected
#[event]
pub struct ProtocolFeeCollectionEvent {
    #[index]
    pub pool: Pubkey,
    pub collector: Pubkey,
    pub amount_0: u64,
    pub amount_1: u64,
    pub timestamp: i64,
}

impl EventBase for ProtocolFeeCollectionEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.collector
    }
}

// ============================================================================
// Hook System Events
// ============================================================================

/// Emitted when a hook registry is initialized
#[event]
pub struct HookRegistryInitializedEvent {
    pub pool: Pubkey,
    pub registry: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a new hook is registered
#[event]
pub struct HookRegisteredEvent {
    pub pool: Pubkey,
    pub hook_program: Pubkey,
    pub event_mask: u32,
    pub stage_mask: u8,
    pub permission: u8,
    pub index: u8,
    pub timestamp: i64,
}

/// Emitted when a hook is unregistered
#[event]
pub struct HookUnregisteredEvent {
    pub pool: Pubkey,
    pub hook_program: Pubkey,
    pub timestamp: i64,
}

/// Emitted when hooks are emergency paused
#[event]
pub struct HooksEmergencyPausedEvent {
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}

// ============================================================================
// Tick & Position Management Events
// ============================================================================

/// Emitted when tick router is updated
#[event]
pub struct RouterUpdatedEvent {
    pub pool: Pubkey,
    pub previous_router: Option<Pubkey>,
    pub new_router: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a tick array is cleaned
#[event]
pub struct TickArrayCleanedEvent {
    #[index]
    pub pool: Pubkey,
    pub tick_array: Pubkey,
    pub start_tick: i32,
    pub initialized_count: u8,
    pub timestamp: i64,
}

/// Alternative tick array cleanup event
#[event]
pub struct TickArrayCleanedUpEvent {
    #[index]
    pub pool: Pubkey,
    pub tick_array: Pubkey,
    pub start_tick: i32,
    pub ticks_cleaned: u8,
    pub gas_refund_estimate: u64,
    pub cleaner: Pubkey,
    pub timestamp: i64,
}

// ============================================================================
// Field Commitment Events
// ============================================================================

/// Emitted when a field commitment is updated with hash
#[event]
#[allow(non_snake_case)]
pub struct FieldCommitmentEvent {
    #[index]
    pub pool: Pubkey,
    pub commitment_hash: [u8; 32],
    pub sequence_number: u64,
    
    // Market scalars for audit trail  
    pub S: u128,
    pub T: u128,
    pub L: u128,
    
    // Commitment parameters for off-chain verification
    pub gap_bps: u64,              // Optimality gap in basis points
    pub lipschitz_L: u64,          // Lipschitz constant
    pub expires_at: i64,           // Commitment expiration timestamp
    
    // Domain weights for field verification
    pub weights: [u32; 4],         // [w_s, w_t, w_l, w_tau] 
    pub omega_weights: [u32; 2],   // [omega_0, omega_1]
    
    // Data source info for audit trail
    pub data_source: Pubkey,       // MarketDataSource account
    pub provider: Pubkey,          // Primary/secondary provider
    
    pub authority: Pubkey,
    pub timestamp: i64,
}

/// Emitted for detailed market update auditing
#[event]
pub struct MarketUpdateAuditEvent {
    #[index]
    pub pool: Pubkey,
    
    // Validation results
    pub validation_passed: bool,
    pub staleness_check: bool,
    pub sequence_check: bool,
    pub authority_check: bool,
    
    // Change metrics (in basis points)
    pub scalar_changes_bps: [u32; 3], // [S, T, L] changes in bps
    pub max_allowed_change_bps: u32,
    
    // Verification proof summary
    pub proof_provided: bool,
    pub convex_bound_verified: bool,
    pub lipschitz_verified: bool,
    pub merkle_verified: bool,
    
    // Timing data for monitoring
    pub update_frequency: i64,
    pub time_since_last: i64,
    pub data_age: i64,
    
    pub timestamp: i64,
}

/// Emitted when field verification is performed
#[event]
pub struct FieldVerificationEvent {
    #[index]
    pub pool: Pubkey,
    pub commitment_hash: [u8; 32],
    
    // Verification details
    pub verification_type: u8,     // 0=Basic, 1=Enhanced, 2=Full
    pub proof_hash: [u8; 32],      // Hash of verification proof
    
    // Bounds verification results
    pub convex_points_checked: u32,
    pub lipschitz_samples: u32,
    pub optimality_gap_bps: u64,
    
    // Coefficient verification (if applicable)
    pub has_local_coefficients: bool,
    pub coefficient_root: Option<[u8; 32]>,
    pub merkle_path_length: u8,
    
    // Validation outcome
    pub all_checks_passed: bool,
    pub failure_reason: Option<String>,
    
    pub verifier: Pubkey,
    pub timestamp: i64,
}

// ============================================================================
// Token Management Events
// ============================================================================

/// Emitted when a new token is created
#[event]
pub struct TokenCreated {
    pub mint: Pubkey,
    pub ticker: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub authority: Pubkey,
    pub initial_supply: u64,
}

// ============================================================================
// Analytics & Aggregation Utilities
// ============================================================================

/// Event aggregation utilities for analytics
pub struct EventAggregator;

/// Rate data point for TWAP calculation
#[derive(Clone, Debug)]
pub struct PricePoint {
    pub sqrt_price: u128,
    pub timestamp: i64,
    pub liquidity: u128,
}

/// Volume data point
#[derive(Clone, Debug)]
pub struct VolumeData {
    pub token_a_volume: u128,
    pub token_b_volume: u128,
    pub timestamp: i64,
}

/// Swap event data structure for aggregation
#[derive(Clone, Debug)]
pub struct SwapEventData {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub zero_for_one: bool,
    pub amount_in: u64,
    pub amount_out: u64,
    pub sqrt_price_before: u128,
    pub sqrt_price_after: u128,
    pub timestamp: i64,
}

impl EventAggregator {
    /// Aggregate volume from swap events
    /// Returns (token_a_volume, token_b_volume) for the given events
    pub fn aggregate_volume(swap_events: &[SwapEventData]) -> (u128, u128) {
        let mut total_volume_a = 0u128;
        let mut total_volume_b = 0u128;

        for event in swap_events {
            // For zero_for_one swaps: amount_in is token_0, amount_out is token_1
            // For one_for_zero swaps: amount_in is token_1, amount_out is token_0
            if event.zero_for_one {
                total_volume_a = total_volume_a.saturating_add(event.amount_in as u128);
                total_volume_b = total_volume_b.saturating_add(event.amount_out as u128);
            } else {
                total_volume_a = total_volume_a.saturating_add(event.amount_out as u128);
                total_volume_b = total_volume_b.saturating_add(event.amount_in as u128);
            }
        }

        (total_volume_a, total_volume_b)
    }

    /// Calculate time-weighted average rate from rate snapshots
    /// TWAP = Σ(price_i × time_weight_i) / Σ(time_weight_i)
    pub fn calculate_twap(price_points: &[PricePoint], window_seconds: i64) -> Option<u128> {
        if price_points.is_empty() || window_seconds <= 0 {
            return None;
        }

        // Find the most recent timestamp
        let latest_timestamp = price_points.iter().map(|p| p.timestamp).max()?;

        let cutoff_time = latest_timestamp.saturating_sub(window_seconds);

        // Filter points within the window and sort by timestamp
        let mut window_points: Vec<&PricePoint> = price_points
            .iter()
            .filter(|p| p.timestamp >= cutoff_time)
            .collect();

        if window_points.is_empty() {
            return None;
        }

        window_points.sort_by_key(|p| p.timestamp);

        // Calculate time-weighted average
        let mut weighted_sum = 0u128;
        let mut total_weight = 0u64;

        for i in 0..window_points.len() {
            let current_point = window_points[i];

            // Calculate time weight (duration this rate was active)
            let time_weight = if i < window_points.len() - 1 {
                // Time until next rate point
                (window_points[i + 1].timestamp - current_point.timestamp) as u64
            } else {
                // Time from last point to end of window
                (latest_timestamp - current_point.timestamp) as u64
            };

            if time_weight > 0 {
                // Weight the rate by time duration
                // Use saturating math to prevent overflow
                let weighted_price = current_point.sqrt_price.saturating_mul(time_weight as u128);
                weighted_sum = weighted_sum.saturating_add(weighted_price);
                total_weight = total_weight.saturating_add(time_weight);
            }
        }

        if total_weight > 0 {
            Some(weighted_sum / total_weight as u128)
        } else {
            None
        }
    }

    /// Calculate volume-weighted average rate (VWAR)
    pub fn calculate_vwap(trades: &[SwapEventData]) -> Option<u128> {
        if trades.is_empty() {
            return None;
        }

        let mut volume_weighted_sum = 0u128;
        let mut total_volume = 0u128;

        for trade in trades {
            // Use the geometric mean of sqrt rates as the trade rate
            let avg_sqrt_price = (trade.sqrt_price_before + trade.sqrt_price_after) / 2;
            let volume = trade.amount_in as u128;

            volume_weighted_sum =
                volume_weighted_sum.saturating_add(avg_sqrt_price.saturating_mul(volume));
            total_volume = total_volume.saturating_add(volume);
        }

        if total_volume > 0 {
            Some(volume_weighted_sum / total_volume)
        } else {
            None
        }
    }
}

// ============================================================================
// 3D Order Events
// ============================================================================

/// Event types for 3D orders
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum OrderEventType {
    Created,
    Filled,
    Modified,
    Closed,
    Redenominated,
}

/// Emitted when a 3D order is created or executed
#[event]
pub struct OrderCreatedEvent {
    #[index]
    pub pool: Pubkey,
    pub user: Pubkey,
    pub order_id: u64,
    pub amount: u64,
    pub rate: u128,
    pub duration: Duration,
    pub leverage: u64,
    pub timestamp: i64,
}

/// Emitted when an order is modified
#[event]
pub struct OrderModifiedEvent {
    #[index]
    pub pool: Pubkey,
    pub user: Pubkey,
    pub order_id: u64,
    pub new_amount: u64,
    pub new_leverage: u64,
    pub timestamp: i64,
}

impl EventBase for OrderCreatedEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.user
    }
}

impl EventBase for OrderModifiedEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.user
    }
}

/// Emitted when a 3D order is modified
#[event]
pub struct OrderModifyEvent {
    #[index]
    pub pool: Pubkey,
    pub user: Pubkey,
    pub order_id: Pubkey,
    pub modification_type: String,
    pub old_value: u64,
    pub new_value: u64,
    pub timestamp: i64,
}

impl EventBase for OrderModifyEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.user
    }
}

/// Emitted during redenomination
#[event]
pub struct RedenominationEvent {
    #[index]
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub market_loss: u128,
    pub total_distributed: u128,
    pub orders_affected: u32,
    pub timestamp: i64,
}

impl EventBase for RedenominationEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.authority
    }
}

// ============================================================================
// Configuration Events
// ============================================================================

/// Emitted when pool configuration is updated
#[event]
pub struct PoolConfigUpdatedEvent {
    #[index]
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub config_type: String,
    pub timestamp: i64,
}

impl EventBase for PoolConfigUpdatedEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.authority
    }
}

// ============================================================================
// Vault Events
// ============================================================================

/// Emitted when a user deposits into a vault
#[event]
pub struct VaultDepositEvent {
    #[index]
    pub vault: Pubkey,
    pub user: Pubkey,
    pub shares_minted: u64,
    pub amount_deposited: u64,
    pub share_class: u8,
    pub timestamp: i64,
}

impl EventBase for VaultDepositEvent {
    fn pool(&self) -> Pubkey {
        self.vault // Vault acts as pool for this context
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.user
    }
}

/// Emitted when a user withdraws from a vault
#[event]
pub struct VaultWithdrawEvent {
    #[index]
    pub vault: Pubkey,
    pub user: Pubkey,
    pub shares_burned: u64,
    pub amount_withdrawn: u64,
    pub share_class: u8,
    pub timestamp: i64,
}

impl EventBase for VaultWithdrawEvent {
    fn pool(&self) -> Pubkey {
        self.vault // Vault acts as pool for this context
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.user
    }
}

// ============================================================================
// Rebase Events
// ============================================================================

/// Rebase event types
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum RebaseEventType {
    LendingRebase,
    LeverageRebase,
    WeightRebase,
}

/// Emitted when rebase indices are updated
#[event]
pub struct RebaseEvent {
    #[index]
    pub pool: Pubkey,
    pub event_type: RebaseEventType,
    pub index_0: u128,
    pub index_1: u128,
    pub funding_index_long: u128,
    pub funding_index_short: u128,
    pub timestamp: i64,
    pub authority: Pubkey,
}

impl EventBase for RebaseEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn actor(&self) -> Pubkey {
        self.authority
    }
}

// ============================================================================
// Event Helper Functions
// ============================================================================

/// Helper functions for creating events with comprehensive audit data
pub struct EventHelpers;

impl EventHelpers {
    /// Create field commitment event with all audit data
    #[allow(non_snake_case, clippy::too_many_arguments)]
    pub fn create_field_commitment_event(
        pool: Pubkey,
        commitment_hash: [u8; 32],
        sequence_number: u64,
        S: u128,
        T: u128, 
        L: u128,
        gap_bps: u64,
        lipschitz_L: u64,
        expires_at: i64,
        weights: [u32; 4],
        omega_weights: [u32; 2],
        data_source: Pubkey,
        provider: Pubkey,
        authority: Pubkey,
    ) -> FieldCommitmentEvent {
        FieldCommitmentEvent {
            pool,
            commitment_hash,
            sequence_number,
            S,
            T,
            L,
            gap_bps,
            lipschitz_L,
            expires_at,
            weights,
            omega_weights,
            data_source,
            provider,
            authority,
            timestamp: Clock::get().map(|c| c.unix_timestamp).unwrap_or(0),
        }
    }
    
    /// Create market event with actual token information
    pub fn create_market_event(
        market: Pubkey,
        event_type: MarketEventType,
        token_0_mint: Pubkey,
        token_1_mint: Pubkey,
        token_0_vault: Pubkey,
        token_1_vault: Pubkey,
        spot_price: u128,
        weights: [u32; 4],
        invariant: u128,
        update_source: u8,
        sequence: u64,
        previous_commitment: [u8; 32],
    ) -> MarketEvent {
        MarketEvent {
            market,
            event_type,
            token_0_mint,
            token_1_mint,
            token_0_vault,
            token_1_vault,
            spot_price,
            weights,
            invariant,
            update_source,
            sequence,
            previous_commitment,
            timestamp: Clock::get().map(|c| c.unix_timestamp).unwrap_or(0),
        }
    }
    
    /// Create enhanced swap event with token mints and work data
    pub fn create_swap_event(
        pool: Pubkey,
        user: Pubkey,
        token_in_mint: Pubkey,
        token_out_mint: Pubkey,
        zero_for_one: bool,
        amount_in: u64,
        amount_out: u64,
        fee: u64,
        protocol_fee: u64,
        sqrt_price_before: u128,
        sqrt_price_after: u128,
        tick_before: i32,
        tick_after: i32,
        work_computed: i128,
        field_commitment_used: [u8; 32],
    ) -> SwapEvent {
        SwapEvent {
            pool,
            user,
            token_in_mint,
            token_out_mint,
            zero_for_one,
            amount_in,
            amount_out,
            fee,
            protocol_fee,
            sqrt_price_before,
            sqrt_price_after,
            tick_before,
            tick_after,
            work_computed,
            field_commitment_used,
            timestamp: Clock::get().map(|c| c.unix_timestamp).unwrap_or(0),
        }
    }
    
    /// Create market update audit event
    pub fn create_audit_event(
        pool: Pubkey,
        validation_passed: bool,
        staleness_check: bool,
        sequence_check: bool,
        authority_check: bool,
        scalar_changes_bps: [u32; 3],
        max_allowed_change_bps: u32,
        proof_provided: bool,
        convex_bound_verified: bool,
        lipschitz_verified: bool,
        merkle_verified: bool,
        update_frequency: i64,
        time_since_last: i64,
        data_age: i64,
    ) -> MarketUpdateAuditEvent {
        MarketUpdateAuditEvent {
            pool,
            validation_passed,
            staleness_check,
            sequence_check,
            authority_check,
            scalar_changes_bps,
            max_allowed_change_bps,
            proof_provided,
            convex_bound_verified,
            lipschitz_verified,
            merkle_verified,
            update_frequency,
            time_since_last,
            data_age,
            timestamp: Clock::get().map(|c| c.unix_timestamp).unwrap_or(0),
        }
    }
    
    /// Create field verification event
    pub fn create_verification_event(
        pool: Pubkey,
        commitment_hash: [u8; 32],
        verification_type: u8,
        proof_hash: [u8; 32],
        convex_points_checked: u32,
        lipschitz_samples: u32,
        optimality_gap_bps: u64,
        has_local_coefficients: bool,
        coefficient_root: Option<[u8; 32]>,
        merkle_path_length: u8,
        all_checks_passed: bool,
        failure_reason: Option<String>,
        verifier: Pubkey,
    ) -> FieldVerificationEvent {
        FieldVerificationEvent {
            pool,
            commitment_hash,
            verification_type,
            proof_hash,
            convex_points_checked,
            lipschitz_samples,
            optimality_gap_bps,
            has_local_coefficients,
            coefficient_root,
            merkle_path_length,
            all_checks_passed,
            failure_reason,
            verifier,
            timestamp: Clock::get().map(|c| c.unix_timestamp).unwrap_or(0),
        }
    }
    
    /// Calculate commitment digest for audit purposes
    pub fn calculate_commitment_digest(
        commitment_hash: [u8; 32],
        gap_bps: u64,
        lipschitz_L: u64,
        expires_at: i64,
    ) -> [u8; 32] {
        use anchor_lang::solana_program::hash::{hash, Hash};
        
        let mut data = Vec::new();
        data.extend_from_slice(&commitment_hash);
        data.extend_from_slice(&gap_bps.to_le_bytes());
        data.extend_from_slice(&lipschitz_L.to_le_bytes());
        data.extend_from_slice(&expires_at.to_le_bytes());
        
        hash(&data).to_bytes()
    }
}
