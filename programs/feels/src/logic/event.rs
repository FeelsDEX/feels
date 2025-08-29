use crate::logic::OrderRoute;
use crate::state::duration::Duration;
/// Centralized event definitions and aggregation utilities for protocol analytics and monitoring.
/// Contains all protocol event structures and aggregation logic for TWAP/VWAP calculations,
/// volume tracking, and rate analytics. Essential for off-chain indexing and analysis.
use anchor_lang::prelude::*;

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

// ============================================================================
// Protocol Initialization Events
// ============================================================================

/// Emitted when a new pool is initialized
#[event]
pub struct PoolInitialized {
    #[index]
    pub pool: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub fee_rate: u16,
    pub tick_spacing: i16,
    pub initial_sqrt_rate: u128,
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
    pub feels_mint: Pubkey,
    pub underlying_mint: Pubkey,
    pub authority: Pubkey,
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
    pub amount_in: u64,
    pub amount_out: u64,
    pub sqrt_rate_after: u128,
    pub tick_after: i32,
    pub fee: u64,
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
    pub route: OrderRoute,
    pub intermediate_amount: Option<u64>,    // For two-hop swaps
    pub sqrt_rate_after_hop1: Option<u128>, // Rate after first hop
    pub sqrt_rate_after_final: u128,        // Final rate state
    pub tick_after_hop1: Option<i32>,        // Tick after first hop
    pub tick_after_final: i32,               // Final tick state
    pub total_fees_paid: u64,                // Sum of all fees across hops
    pub protocol_fees_collected: u64,        // Protocol fees from all hops
    pub gas_used_estimate: u64,              // Estimated compute units used
    pub timestamp: i64,
}

impl EventBase for CrossTokenSwapEvent {
    fn pool(&self) -> Pubkey {
        match self.route {
            OrderRoute::Direct(pool) => pool,
            OrderRoute::TwoHop(pool1, _pool2) => pool1, // Return first pool
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
    pub amount_a: u64,
    pub amount_b: u64,
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
    pub amount_a: u64,
    pub amount_b: u64,
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
    pub amount_a: u64,
    pub amount_b: u64,
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
    pub sqrt_rate: u128,
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
    pub sqrt_rate_before: u128,
    pub sqrt_rate_after: u128,
    pub timestamp: i64,
}

impl EventAggregator {
    /// Aggregate volume from swap events
    /// Returns (token_a_volume, token_b_volume) for the given events
    pub fn aggregate_volume(swap_events: &[SwapEventData]) -> (u128, u128) {
        let mut total_volume_a = 0u128;
        let mut total_volume_b = 0u128;

        for event in swap_events {
            // For zero_for_one swaps: amount_in is token_a, amount_out is token_b
            // For one_for_zero swaps: amount_in is token_b, amount_out is token_a
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
                let weighted_price = current_point.sqrt_rate.saturating_mul(time_weight as u128);
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
            let avg_sqrt_rate = (trade.sqrt_rate_before + trade.sqrt_rate_after) / 2;
            let volume = trade.amount_in as u128;

            volume_weighted_sum =
                volume_weighted_sum.saturating_add(avg_sqrt_rate.saturating_mul(volume));
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
pub struct OrderEvent {
    #[index]
    pub pool: Pubkey,
    pub user: Pubkey,
    pub order_type: OrderEventType,
    pub amount_in: u64,
    pub amount_out: u64,
    pub rate_tick: i32,
    pub duration: Duration,
    pub leverage: u64,
    pub fees_paid: u64,
    pub timestamp: i64,
}

impl EventBase for OrderEvent {
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
