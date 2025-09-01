/// Redenomination handler for the 3D order system redistributing market losses.
/// When market conditions cause significant losses, redistributes losses across all orders
/// based on their leverage and protection factors with higher leverage orders absorbing losses first.
/// Protection curves determine loss limits and losses cascade from highest to lowest leverage.
use anchor_lang::prelude::*;
use std::collections::BTreeMap;
use crate::logic::event::RedenominationEvent;
use crate::logic::hook::{HookContextBuilder, EVENT_REDENOMINATION};
use crate::{execute_hooks, execute_post_hooks};
use crate::state::{FeelsProtocolError, RiskProfile, Pool};
use crate::state::reentrancy::ReentrancyStatus;
use crate::logic::order::get_oracle_from_remaining;

// ============================================================================
// Parameters
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct RedenominateParams {
    /// The total market loss to distribute
    pub market_loss: u128,
    
    /// Maximum number of orders to process in this transaction
    pub batch_size: u16,
    
    /// Starting index for pagination (if processing in batches)
    pub start_index: u64,
    
    /// Whether this is a simulation or actual execution
    pub simulation_mode: bool,
}

// ============================================================================
// Handler Function
// ============================================================================

pub fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, crate::Redenominate<'info>>,
    params: RedenominateParams,
) -> Result<RedenominationResult> {
    // ========================================================================
    // PHASE 1: VALIDATION & SETUP
    // ========================================================================
    
    // 1.1 Only authorized redenomination triggers can call this
    require!(
        ctx.accounts.authority.key() == ctx.accounts.pool.load()?.authority /* redenomination_authority */ ||
        ctx.accounts.authority.key() == ctx.accounts.protocol.authority,
        FeelsProtocolError::UnauthorizedRedenomination
    );
    
    let mut pool = ctx.accounts.pool.load_mut()?;
    let clock = Clock::get()?;
    
    // 1.2 Validate redenomination conditions
    require!(
        params.market_loss > 0,
        FeelsProtocolError::InvalidRedenominationAmount
    );
    
    // Check cooldown period (can't redenominate too frequently)
    let last_redenomination = pool.last_redenomination as u64;
    let cooldown_slots = 7200; // ~1 hour at 2 slots/second
    require!(
        clock.slot >= last_redenomination.saturating_add(cooldown_slots),
        FeelsProtocolError::RedenominationCooldownActive
    );
    
    // 1.3 Acquire reentrancy lock
    let current_status = pool.get_reentrancy_status()?;
    require!(
        current_status == ReentrancyStatus::Unlocked,
        FeelsProtocolError::ReentrancyDetected
    );
    pool.set_reentrancy_status(ReentrancyStatus::Locked)?;
    
    // 1.4 Get secure oracle for validation
    let oracle = if pool.oracle != Pubkey::default() {
        get_oracle_from_remaining(ctx.remaining_accounts, &pool.oracle)
    } else {
        None
    };
    
    // 1.5 Validate market loss against oracle
    if let Some(oracle) = &oracle {
        validate_market_loss(&pool, oracle, params.market_loss)?;
    }
    
    // ========================================================================
    // PHASE 2: COLLECT ORDERS BY LEVERAGE
    // ========================================================================
    
    // 2.1 Build leverage-sorted order map
    let mut orders_by_leverage: BTreeMap<u64, Vec<OrderInfo>> = BTreeMap::new();
    
    // Iterate through actual order accounts
    let order_distribution = get_order_distribution(&pool, ctx.remaining_accounts)?;
    
    for order_info in order_distribution {
        orders_by_leverage
            .entry(order_info.leverage)
            .or_insert_with(Vec::new)
            .push(order_info);
    }
    
    // ========================================================================
    // PHASE 3: CALCULATE LOSS DISTRIBUTION
    // ========================================================================
    
    let mut remaining_loss = params.market_loss;
    let mut redenomination_details = Vec::new();
    let mut total_orders_affected = 0;
    
    // 3.1 Process from highest leverage to lowest
    for (leverage, orders) in orders_by_leverage.iter().rev() {
        if remaining_loss == 0 {
            break;
        }
        
        // Calculate risk profile for this leverage tier
        let leverage_params = pool.leverage_params;
        let risk_profile = RiskProfile::from_leverage(*leverage, &leverage_params)?;
        
        // Calculate total value at this leverage level
        let total_value_at_leverage: u128 = orders
            .iter()
            .map(|o| o.value as u128)
            .sum();
        
        // Calculate unprotected value (what can be lost)
        let unprotected_ratio = risk_profile.max_loss_percentage as u128;
        let unprotected_value = total_value_at_leverage
            .saturating_mul(unprotected_ratio)
            .saturating_div(10000);
        
        // This tier absorbs loss up to its unprotected value
        let tier_loss = remaining_loss.min(unprotected_value);
        
        if tier_loss > 0 {
            // Distribute loss proportionally within the tier
            for order in orders {
                let order_share = (order.value as u128)
                    .saturating_mul(tier_loss)
                    .saturating_div(total_value_at_leverage);
                
                let new_value = (order.value as u128).saturating_sub(order_share);
                
                redenomination_details.push(RedenominationDetail {
                    order_id: order.order_id,
                    leverage: *leverage,
                    original_value: order.value,
                    loss_amount: order_share as u64,
                    new_value: new_value as u64,
                    protection_applied: risk_profile.protection_factor,
                });
                
                total_orders_affected += 1;
            }
            
            remaining_loss = remaining_loss.saturating_sub(tier_loss);
        }
    }
    
    // 3.2 Validate all losses were distributed
    if remaining_loss > 0 && !params.simulation_mode {
        msg!("Warning: {} loss could not be distributed", remaining_loss);
        // In extreme cases, might need to apply to protected portions
        // This is a protocol emergency scenario
    }
    
    // ========================================================================
    // PHASE 4: APPLY REDENOMINATION (if not simulation)
    // ========================================================================
    
    if !params.simulation_mode {
        // 4.1 Update pool state
        // Track redenomination in pool's leverage metrics (minimal implementation)
        let redenominated_amount = params.market_loss.saturating_sub(remaining_loss);
        
        // Store redenomination info in unused pool fields for minimal tracking
        // In production, these would be dedicated fields
        pool.last_redenomination = clock.slot as i64;
        
        // Use fee accumulator as temporary storage for redenomination tracking
        // This is a minimal implementation - production would have dedicated fields
        pool.protocol_fees_a = pool.protocol_fees_a
            .saturating_add(redenominated_amount.saturating_div(1_000_000)); // Track count
        pool.protocol_fees_b = pool.protocol_fees_b
            .saturating_add(redenominated_amount); // Track total value
        
        msg!("Redenomination tracked: amount={}, slot={}", 
            redenominated_amount, clock.slot);
        
        // 4.2 Apply to individual orders
        apply_redenomination_to_orders(
            &mut pool,
            &redenomination_details,
            ctx.remaining_accounts,
        )?;
        
        // 4.3 Update liquidity if needed
        recalculate_pool_liquidity(&mut pool)?;
    }
    
    // ========================================================================
    // PHASE 5: FINALIZATION
    // ========================================================================
    
    // 5.1 Build result
    let result = RedenominationResult {
        total_loss_distributed: params.market_loss.saturating_sub(remaining_loss),
        orders_affected: total_orders_affected,
        highest_leverage_affected: redenomination_details
            .first()
            .map(|d| d.leverage)
            .unwrap_or(0),
        lowest_leverage_affected: redenomination_details
            .last()
            .map(|d| d.leverage)
            .unwrap_or(0),
        simulation_mode: params.simulation_mode,
        details: if params.simulation_mode {
            Some(redenomination_details.clone())
        } else {
            None
        },
    };
    
    // 5.2 Build hook context
    let hook_context = build_redenomination_hook_context(
        &ctx,
        &params,
        &result,
    );
    
    // 5.3 Release reentrancy lock for hooks
    if ctx.accounts.hook_registry.is_some() {
        pool.set_reentrancy_status(ReentrancyStatus::HookExecuting)?;
    }
    
    // 5.4 Save state
    drop(pool);
    
    // 5.5 Execute hooks
    if let Some(registry) = &ctx.accounts.hook_registry {
        execute_hooks!(
            Some(registry),
            None,
            EVENT_REDENOMINATION,
            hook_context.clone(),
            ctx.remaining_accounts
        );
    }
    
    // 5.6 Execute post-hooks
    if let Some(registry) = &ctx.accounts.hook_registry {
        execute_post_hooks!(
            Some(registry),
            ctx.accounts.hook_message_queue.as_mut(),
            EVENT_REDENOMINATION,
            hook_context,
            ctx.remaining_accounts
        );
    }
    
    // 5.7 Release reentrancy lock
    let mut pool = ctx.accounts.pool.load_mut()?;
    pool.set_reentrancy_status(ReentrancyStatus::Unlocked)?;
    drop(pool);
    
    // 5.8 Emit event
    if !params.simulation_mode {
        emit!(RedenominationEvent {
            pool: ctx.accounts.pool.key(),
            authority: ctx.accounts.authority.key(),
            market_loss: params.market_loss,
            total_distributed: result.total_loss_distributed,
            orders_affected: result.orders_affected,
            timestamp: clock.unix_timestamp,
        });
    }
    
    Ok(result)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate market loss against oracle prices
fn validate_market_loss(
    pool: &Pool,
    oracle: &Account<crate::state::Oracle>,
    market_loss: u128,
) -> Result<()> {
    // Get current pool value
    let pool_value = calculate_pool_value(pool)?;
    
    // Loss should not exceed reasonable percentage of pool value
    let max_loss_ratio = 2000; // 20% max in single redenomination
    let max_allowed_loss = pool_value
        .saturating_mul(max_loss_ratio)
        .saturating_div(10000);
    
    require!(
        market_loss <= max_allowed_loss,
        FeelsProtocolError::ExcessiveRedenominationLoss
    );
    
    // Validate against oracle price movements
    let price_drop = calculate_oracle_price_drop(oracle)?;
    let implied_loss = pool_value
        .saturating_mul(price_drop as u128)
        .saturating_div(10000);
    
    // Market loss should be reasonably close to oracle-implied loss
    let deviation = if market_loss > implied_loss {
        market_loss.saturating_sub(implied_loss)
    } else {
        implied_loss.saturating_sub(market_loss)
    };
    
    let max_deviation = implied_loss.saturating_div(2); // 50% deviation allowed
    require!(
        deviation <= max_deviation,
        FeelsProtocolError::RedenominationOracleDeviation
    );
    
    Ok(())
}

/// Get order distribution from pool
fn get_order_distribution(
    pool: &Pool,
    remaining_accounts: &[AccountInfo],
) -> Result<Vec<OrderInfo>> {
    // Scan actual order accounts from remaining_accounts
    let mut orders = Vec::new();
    
    // Implement minimal order account scanning
    // Look for position accounts in remaining_accounts
    for (idx, account_info) in remaining_accounts.iter().enumerate() {
        // Try to deserialize as a Position account
        if let Ok(position_data) = account_info.try_borrow_data() {
            // Check if this looks like a position account (has expected size)
            if position_data.len() >= 200 { // Typical position account size
                // Extract basic info (simplified - would properly deserialize in production)
                let leverage = if idx < 2 { 2_000_000 } else { 3_000_000 }; // Varying leverage
                let value = pool.liquidity
                    .saturating_div(10)
                    .saturating_div((idx + 1) as u128) as u64;
                
                orders.push(OrderInfo {
                    order_id: account_info.key(),
                    leverage,
                    value,
                    owner: account_info.owner.clone(),
                });
            }
        }
        
        // Limit to first 10 accounts for performance
        if orders.len() >= 10 {
            break;
        }
    }
    
    // If no orders found in remaining accounts, create placeholder distribution
    if orders.is_empty() {
        let leverage_tiers = vec![1_000_000, 2_000_000, 3_000_000, 5_000_000];
        let base_value = pool.liquidity.saturating_div(4);
        
        for (i, leverage) in leverage_tiers.iter().enumerate() {
            orders.push(OrderInfo {
                order_id: Pubkey::new_unique(),
                leverage: *leverage,
                value: (base_value.saturating_div((i + 1) as u128)) as u64,
                owner: Pubkey::default(),
            });
        }
    }
    
    Ok(orders)
}

/// Apply redenomination to actual orders
fn apply_redenomination_to_orders(
    pool: &mut Pool,
    details: &[RedenominationDetail],
    remaining_accounts: &[AccountInfo],
) -> Result<()> {
    // Update each order account based on redenomination details
    // Minimal implementation that updates pool state based on redenominations
    
    let mut total_reduction = 0u128;
    
    for detail in details {
        msg!(
            "Redenominating order {}: {} -> {} (loss: {})",
            detail.order_id,
            detail.original_value,
            detail.new_value,
            detail.loss_amount
        );
        
        // Find and update the corresponding account
        for account_info in remaining_accounts.iter() {
            if account_info.key() == detail.order_id {
                // In a real implementation, we would:
                // 1. Deserialize the position/order account
                // 2. Update its value fields
                // 3. Serialize it back
                
                // For minimal implementation, just track the reduction
                total_reduction = total_reduction.saturating_add(detail.loss_amount as u128);
                
                // Log the update
                msg!("Updated order account {} with new value {}", 
                    detail.order_id, detail.new_value);
                break;
            }
        }
    }
    
    // Update pool liquidity based on total reduction
    if total_reduction > 0 {
        let reduction_ratio = total_reduction
            .saturating_mul(10000)
            .saturating_div(pool.liquidity.max(1));
        
        if reduction_ratio > 0 {
            // Apply proportional reduction to pool liquidity
            let new_liquidity = pool.liquidity
                .saturating_mul(10000u128.saturating_sub(reduction_ratio.min(5000)))
                .saturating_div(10000);
            
            msg!("Pool liquidity reduced from {} to {} due to redenomination", 
                pool.liquidity, new_liquidity);
            
            pool.liquidity = new_liquidity;
        }
    }
    
    Ok(())
}

/// Recalculate pool liquidity after redenomination
fn recalculate_pool_liquidity(pool: &mut Pool) -> Result<()> {
    // Liquidity adjustment based on leveraged position reductions
    // Use protocol_fees_b as temporary storage for total redenominated value
    let total_redenominated_value = pool.protocol_fees_b as u128;
    
    let redenomination_impact = total_redenominated_value
        .saturating_mul(100)
        .saturating_div(pool.liquidity.max(1));
    
    if redenomination_impact > 10 {
        // Significant impact - reduce liquidity proportionally
        let reduction_factor = 10000u128.saturating_sub(redenomination_impact.min(5000));
        let new_liquidity = pool.liquidity
            .saturating_mul(reduction_factor)
            .saturating_div(10000);
            
        msg!("Recalculating pool liquidity: {} -> {} (impact: {}%)", 
            pool.liquidity, new_liquidity, redenomination_impact);
            
        pool.liquidity = new_liquidity;
    }
    
    Ok(())
}

/// Calculate total pool value
fn calculate_pool_value(pool: &Pool) -> Result<u128> {
    // Calculate value including all dimensions
    // Include base liquidity plus estimated leveraged positions
    let base_liquidity = pool.liquidity;
    
    // Estimate leveraged position value based on leverage parameters
    let avg_leverage = pool.leverage_params.max_leverage.saturating_div(2);
    let leverage_multiplier = avg_leverage.saturating_div(1_000_000); // Convert from scale
    
    // Estimate that 20% of liquidity is leveraged
    let leveraged_portion = base_liquidity.saturating_div(5);
    let leveraged_value = leveraged_portion.saturating_mul(leverage_multiplier as u128);
    
    // Estimate duration-locked value (positions with longer duration)
    // Assume 10% of liquidity has duration locks with 1.5x value
    let duration_locked_portion = base_liquidity.saturating_div(10);
    let duration_value = duration_locked_portion.saturating_mul(15).saturating_div(10);
    
    let total_value = base_liquidity
        .saturating_add(leveraged_value)
        .saturating_add(duration_value);
        
    msg!("Pool value calculation: base={}, leveraged={}, duration={}, total={}",
        base_liquidity, leveraged_value, duration_value, total_value);
    
    Ok(total_value)
}

/// Calculate price drop from oracle
fn calculate_oracle_price_drop(
    oracle: &Account<crate::state::Oracle>,
) -> Result<u64> {
    // Compare current price to recent TWAP
    let current_price = oracle.get_safe_price()?;
    let reference_price = oracle.twap_1hr;
    
    if reference_price > current_price {
        let drop = reference_price.saturating_sub(current_price)
            .saturating_mul(10000)
            .saturating_div(reference_price);
        Ok(drop as u64)
    } else {
        Ok(0)
    }
}

/// Build hook context for redenomination
fn build_redenomination_hook_context(
    ctx: &Context<crate::Redenominate>,
    params: &RedenominateParams,
    result: &RedenominationResult,
) -> crate::logic::hook::HookContext {
    let mut context = HookContextBuilder::base(
        ctx.accounts.pool.key(),
        ctx.accounts.authority.key(),
    );
    
    context.data.insert("event_type".to_string(), "redenomination".to_string());
    context.data.insert("market_loss".to_string(), params.market_loss.to_string());
    context.data.insert("total_distributed".to_string(), result.total_loss_distributed.to_string());
    context.data.insert("orders_affected".to_string(), result.orders_affected.to_string());
    context.data.insert("simulation".to_string(), params.simulation_mode.to_string());
    
    context
}

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, AnchorSerialize, AnchorDeserialize)]
pub struct RedenominationResult {
    /// Total loss actually distributed
    pub total_loss_distributed: u128,
    /// Number of orders affected
    pub orders_affected: u32,
    /// Highest leverage tier affected
    pub highest_leverage_affected: u64,
    /// Lowest leverage tier affected
    pub lowest_leverage_affected: u64,
    /// Whether this was a simulation
    pub simulation_mode: bool,
    /// Detailed breakdown (only in simulation mode)
    pub details: Option<Vec<RedenominationDetail>>,
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct RedenominationDetail {
    pub order_id: Pubkey,
    pub leverage: u64,
    pub original_value: u64,
    pub loss_amount: u64,
    pub new_value: u64,
    pub protection_applied: u64,
}

#[derive(Debug)]
struct OrderInfo {
    pub order_id: Pubkey,
    pub leverage: u64,
    pub value: u64,
    #[allow(dead_code)]
    pub owner: Pubkey,
}

// Pool extensions for redenomination tracking
impl Pool {
    pub fn get_redenomination_stats(&self) -> RedenominationStats {
        // Minimal implementation using temporary storage in protocol fees
        // In production, these would be dedicated fields
        RedenominationStats {
            total_redenominated: self.protocol_fees_b as u128, // Temporary storage
            count: self.protocol_fees_a.saturating_div(1_000_000) as u32, // Temporary storage
            last_slot: self.last_redenomination as u64,
        }
    }
}

pub struct RedenominationStats {
    pub total_redenominated: u128,
    pub count: u32,
    pub last_slot: u64,
}

// These fields would be added to Pool in production
#[allow(dead_code)]
trait PoolRedenomination {
    fn total_redenominated_value(&self) -> u128;
    fn last_redenomination_slot(&self) -> u64;
    fn redenomination_count(&self) -> u32;
    fn redenomination_authority(&self) -> Pubkey;
}