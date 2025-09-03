/// Usage examples for the unified fee model
/// Demonstrates common integration patterns for traders, keepers, and protocols

use anchor_lang::prelude::*;
use feels::{
    state::*,
    instructions::*,
    logic::*,
    constant::*,
};

// ============================================================================
// Example 1: Trader Integration
// ============================================================================

/// Calculate fees before executing a trade
pub async fn estimate_trading_fees(
    program: &Program,
    market: Pubkey,
    amount_in: u64,
    is_buy: bool,
) -> Result<TradingFeeEstimate> {
    // 1. Get current field commitment
    let field_commitment = program
        .account::<FieldCommitment>(derive_field_commitment_address(&market))
        .await?;
    
    // 2. Check if in fallback mode
    let current_time = Clock::get()?.unix_timestamp;
    let is_stale = current_time - field_commitment.snapshot_ts > field_commitment.max_staleness;
    
    if is_stale {
        println!("Warning: Using fallback mode fees");
    }
    
    // 3. Get pool status
    let pool_status = program
        .account::<PoolStatus>(derive_pool_status_address(&market))
        .await?;
    
    if !pool_status.can_accept_orders() {
        return Err(error!("Pool is disabled"));
    }
    
    // 4. Calculate work (simplified)
    let work = if is_buy {
        1000 // Positive work for buying (simplified)
    } else {
        -500 // Negative work for selling (simplified)
    };
    
    // 5. Calculate fees
    let base_fee_bps = field_commitment.base_fee_bps;
    let fee_multiplier = pool_status.get_fee_multiplier();
    let effective_base_fee = (base_fee_bps * fee_multiplier) / BPS_DENOMINATOR;
    
    let work_fee = work_to_fee(work, amount_in, effective_base_fee)?;
    let rebate = if work < 0 {
        calculate_rebate(work.abs(), amount_in, &field_commitment)?
    } else {
        0
    };
    
    Ok(TradingFeeEstimate {
        base_fee_bps: effective_base_fee,
        work_surcharge: work_fee,
        rebate_amount: rebate,
        net_fee: work_fee.saturating_sub(rebate),
        pool_status: pool_status.status,
        is_fallback: is_stale,
    })
}

/// Execute trade with fee validation
pub async fn execute_trade_with_fees(
    program: &Program,
    market: Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    max_fee_bps: u64,
) -> Result<()> {
    // 1. Estimate fees
    let fee_estimate = estimate_trading_fees(program, market, amount_in, true).await?;
    
    // 2. Validate acceptable fee
    let fee_bps = (fee_estimate.net_fee * BPS_DENOMINATOR) / amount_in;
    if fee_bps > max_fee_bps {
        return Err(error!("Fee {} bps exceeds maximum {} bps", fee_bps, max_fee_bps));
    }
    
    // 3. Execute trade
    program
        .request()
        .accounts(Order {
            market_field: derive_market_field_address(&market),
            pool_status: derive_pool_status_address(&market),
            field_commitment: derive_field_commitment_address(&market),
            buffer: derive_buffer_address(&market),
            // ... other accounts
        })
        .args(OrderParams::Create(CreateOrderParams {
            order_type: OrderType::Immediate,
            amount: amount_in,
            rate_params: RateParams::TargetRate {
                sqrt_rate_limit: price_to_sqrt_rate(min_amount_out, amount_in),
                direction: SwapDirection::BuyExactIn,
            },
            duration: Duration::Spot,
            leverage: LEVERAGE_SCALE, // 1x
            max_slippage_bps: 100,
        }))
        .send()
        .await?;
    
    Ok(())
}

// ============================================================================
// Example 2: Keeper Integration
// ============================================================================

/// Keeper service that updates field commitments
pub struct FeeKeeperService {
    hysteresis_controller: HysteresisController,
    markets: Vec<MarketConfig>,
}

impl FeeKeeperService {
    /// Main keeper loop
    pub async fn run(&mut self) -> Result<()> {
        loop {
            for market_config in &self.markets {
                match self.update_market_fees(&market_config).await {
                    Ok(_) => println!("Updated fees for {}", market_config.market),
                    Err(e) => eprintln!("Failed to update {}: {}", market_config.market, e),
                }
            }
            
            // Wait before next update cycle
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
    
    /// Update fees for a single market
    async fn update_market_fees(&mut self, config: &MarketConfig) -> Result<()> {
        // 1. Fetch market state
        let market_state = fetch_market_state(&config.market).await?;
        
        // 2. Calculate stress components
        let stress = StressComponents {
            spot_stress: calculate_spot_stress(&market_state)?,
            time_stress: calculate_time_stress(&market_state)?,
            leverage_stress: calculate_leverage_stress(&market_state)?,
        };
        
        println!("Market {} stress: {:?}", config.market, stress);
        
        // 3. Update hysteresis controller
        let current_time = Clock::get()?.unix_timestamp;
        let new_base_fee = self.hysteresis_controller.update(&stress, current_time)?;
        
        println!("Base fee: {} -> {} bps", 
            self.hysteresis_controller.current_fee, new_base_fee);
        
        // 4. Compute field commitment
        let mut field_computer = FieldComputer::new();
        let field_commitment = field_computer.compute_field_commitment(&market_state)?;
        
        // 5. Submit update
        submit_field_update(&config.market, &field_commitment).await?;
        
        Ok(())
    }
}

/// Calculate spot stress from price deviation
fn calculate_spot_stress(market_state: &MarketState) -> Result<u64> {
    let current_price = sqrt_price_to_price(market_state.current_sqrt_price);
    let twap_price = market_state.twap_a; // Simplified
    
    let deviation = if current_price > twap_price {
        ((current_price - twap_price) * BPS_DENOMINATOR as u128) / twap_price
    } else {
        ((twap_price - current_price) * BPS_DENOMINATOR as u128) / twap_price
    };
    
    Ok(deviation.min(BPS_DENOMINATOR as u128) as u64)
}

// ============================================================================
// Example 3: Liquidator Integration
// ============================================================================

/// Monitor leveraged positions for liquidation opportunities
pub async fn monitor_leverage_safety(
    program: &Program,
    markets: Vec<Pubkey>,
) -> Result<Vec<LiquidationOpportunity>> {
    let mut opportunities = Vec::new();
    
    for market in markets {
        // 1. Get field commitment and check leverage stress
        let field = program
            .account::<FieldCommitment>(derive_field_commitment_address(&market))
            .await?;
        
        let leverage_stress = calculate_leverage_stress_from_field(&field)?;
        
        if leverage_stress > 8000 { // 80% stress threshold
            println!("High leverage stress in {}: {} bps", market, leverage_stress);
            
            // 2. Check for underwater positions
            let positions = fetch_leveraged_positions(&market).await?;
            
            for position in positions {
                if is_liquidatable(&position, &field)? {
                    opportunities.push(LiquidationOpportunity {
                        market,
                        position: position.pubkey,
                        owner: position.owner,
                        collateral_value: position.collateral_value,
                        debt_value: position.debt_value,
                        expected_profit: calculate_liquidation_profit(&position)?,
                    });
                }
            }
        }
    }
    
    Ok(opportunities)
}

// ============================================================================
// Example 4: Protocol Integration
// ============================================================================

/// Integrate unified fee model into a lending protocol
pub struct LendingProtocolFees;

impl LendingProtocolFees {
    /// Apply conservation-preserving rebase with fees
    pub async fn apply_lending_rebase(
        ctx: Context<ApplyLendingRebase>,
        proof: BufferConservationProof,
    ) -> Result<()> {
        // 1. Verify conservation with buffer participation
        let conservation_ctx = BufferConservationContext {
            buffer: &ctx.accounts.buffer.load()?,
            field_commitment: &ctx.accounts.field_commitment.load()?,
            operation_type: RebaseOperationType::Lending,
        };
        
        verify_conservation_with_buffer(&proof, &conservation_ctx)?;
        
        // 2. Calculate fee distribution
        let total_fees = calculate_lending_fees(
            ctx.accounts.lending_pool.utilization,
            ctx.accounts.field_commitment.load()?.base_fee_bps,
        )?;
        
        let domain_activity = DomainActivity {
            spot_volume: 0, // Lending is pure time dimension
            time_volume: ctx.accounts.lending_pool.total_borrowed,
            leverage_volume: 0,
            total_volume: ctx.accounts.lending_pool.total_borrowed,
        };
        
        let buffer_fee_share = calculate_buffer_fee_share(
            total_fees,
            &domain_activity,
            &ctx.accounts.field_commitment.load()?,
        )?;
        
        // 3. Apply rebase with conservation
        msg!("Applying lending rebase with conservation");
        msg!("  Lender growth: {}", proof.base_proof.growth_factors[0]);
        msg!("  Borrower growth: {}", proof.base_proof.growth_factors[1]);
        msg!("  Buffer growth: {}", proof.buffer_growth_factor);
        msg!("  Buffer fee share: {}", buffer_fee_share);
        
        // Update indices
        ctx.accounts.lending_pool.lender_index = ctx.accounts.lending_pool.lender_index
            .checked_mul(proof.base_proof.growth_factors[0])
            .ok_or(FeelsProtocolError::MathOverflow)?
            / Q64;
            
        ctx.accounts.lending_pool.borrower_index = ctx.accounts.lending_pool.borrower_index
            .checked_mul(proof.base_proof.growth_factors[1])
            .ok_or(FeelsProtocolError::MathOverflow)?
            / Q64;
        
        // Update buffer
        ctx.accounts.buffer.load_mut()?.collect_fees(buffer_fee_share as u128)?;
        
        Ok(())
    }
}

// ============================================================================
// Helper Types
// ============================================================================

#[derive(Debug)]
pub struct TradingFeeEstimate {
    pub base_fee_bps: u64,
    pub work_surcharge: u64,
    pub rebate_amount: u64,
    pub net_fee: u64,
    pub pool_status: u8, // 0=Normal, 1=Warning, 2=Disabled, 3=Cooldown
    pub is_fallback: bool,
}

#[derive(Debug)]
pub struct MarketConfig {
    pub market: Pubkey,
    pub min_update_interval: i64,
    pub stress_weights: (u32, u32, u32),
}

#[derive(Debug)]
pub struct LiquidationOpportunity {
    pub market: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub collateral_value: u64,
    pub debt_value: u64,
    pub expected_profit: u64,
}

// ============================================================================
// Utility Functions
// ============================================================================

fn derive_field_commitment_address(market: &Pubkey) -> Pubkey {
    let seeds = &[b"field_commitment", market.as_ref()];
    Pubkey::find_program_address(seeds, &feels::id()).0
}

fn derive_pool_status_address(market: &Pubkey) -> Pubkey {
    let seeds = &[b"pool_status", market.as_ref()];
    Pubkey::find_program_address(seeds, &feels::id()).0
}

fn derive_buffer_address(market: &Pubkey) -> Pubkey {
    let seeds = &[b"buffer", market.as_ref()];
    Pubkey::find_program_address(seeds, &feels::id()).0
}

fn derive_market_field_address(market: &Pubkey) -> Pubkey {
    let seeds = &[b"market_field", market.as_ref()];
    Pubkey::find_program_address(seeds, &feels::id()).0
}

fn sqrt_price_to_price(sqrt_price: u128) -> u128 {
    (sqrt_price * sqrt_price) >> 64
}

fn price_to_sqrt_rate(amount_out: u64, amount_in: u64) -> u128 {
    let price = (amount_out as u128 * Q64) / amount_in as u128;
    // Approximate sqrt (would use proper sqrt in production)
    (price * Q64) >> 32
}

fn calculate_rebate(
    work_magnitude: i128,
    amount_in: u64,
    field: &FieldCommitment,
) -> Result<u64> {
    // Simplified rebate calculation
    let rebate_bps = (work_magnitude * 10) / Q64 as i128; // 10x work scaling
    let rebate = (amount_in as u128 * rebate_bps as u128) / BPS_DENOMINATOR as u128;
    Ok(rebate.min(amount_in as u128 / 100) as u64) // Cap at 1%
}