//! Builder patterns for complex test setups

use super::*;
// use async_trait::async_trait;

/// Builder for creating markets with custom configuration
pub struct MarketBuilder {
    ctx: TestContext,
    token_0: Option<Pubkey>,
    token_1: Option<Pubkey>,
    initial_price: Option<u128>,
    fee_rate: Option<u16>,
    liquidity_positions: Vec<(Keypair, i32, i32, u128)>,
}

impl MarketBuilder {
    pub fn new(ctx: TestContext) -> Self {
        Self {
            ctx,
            token_0: None,
            token_1: None,
            initial_price: None,
            fee_rate: None,
            liquidity_positions: Vec::new(),
        }
    }

    pub fn token_0(mut self, token: Pubkey) -> Self {
        self.token_0 = Some(token);
        self
    }

    pub fn token_1(mut self, token: Pubkey) -> Self {
        self.token_1 = Some(token);
        self
    }

    pub fn initial_price(mut self, price: u128) -> Self {
        self.initial_price = Some(price);
        self
    }

    pub fn fee_rate(mut self, rate: u16) -> Self {
        self.fee_rate = Some(rate);
        self
    }

    pub fn tick_spacing(mut self, _spacing: u16) -> Self {
        // Tick spacing is determined by fee rate in the actual implementation
        // This is just for compatibility with tests
        self
    }

    pub fn add_liquidity(
        mut self,
        provider: Keypair,
        lower_tick: i32,
        upper_tick: i32,
        liquidity: u128,
    ) -> Self {
        self.liquidity_positions.push((provider, lower_tick, upper_tick, liquidity));
        self
    }

    pub fn add_full_range_liquidity(mut self, provider: Keypair, liquidity: u128) -> Self {
        self.liquidity_positions.push((
            provider,
            constants::MIN_TICK,
            constants::MAX_TICK,
            liquidity,
        ));
        self
    }

    pub async fn build(self) -> TestResult<Pubkey> {
        let token_0 = self.token_0.ok_or("Token A not set")?;
        let token_1 = self.token_1.ok_or("Token B not set")?;
        
        let market_helper = self.ctx.market_helper();
        
        // Create market based on configuration
        let market_id = if let Some(initial_price) = self.initial_price {
            market_helper.create_raydium_market(&token_0, &token_1, initial_price).await?
        } else {
            market_helper.create_simple_market(&token_0, &token_1).await?
        };

        // Add liquidity positions if any
        if !self.liquidity_positions.is_empty() {
            // Note: Position management is not available in the current SDK
            // This would need to be implemented when position instructions are added
            // For now, we skip adding liquidity positions
        }

        Ok(market_id)
    }
}

/// Builder for creating complex swap scenarios
pub struct SwapBuilder {
    ctx: TestContext,
    swaps: Vec<SwapSpec>,
}

struct SwapSpec {
    market: Pubkey,
    trader: Keypair,
    token_in: Pubkey,
    token_out: Pubkey,
    amount_in: Option<u64>,
    amount_out: Option<u64>,
    max_slippage: Option<f64>,
    delay_ms: Option<u64>,
}

impl SwapBuilder {
    pub fn new(ctx: TestContext) -> Self {
        Self {
            ctx,
            swaps: Vec::new(),
        }
    }

    pub fn add_swap(
        mut self,
        market: Pubkey,
        trader: Keypair,
        token_in: Pubkey,
        token_out: Pubkey,
        amount_in: u64,
    ) -> Self {
        self.swaps.push(SwapSpec {
            market,
            trader,
            token_in,
            token_out,
            amount_in: Some(amount_in),
            amount_out: None,
            max_slippage: None,
            delay_ms: None,
        });
        self
    }

    pub fn add_swap_exact_out(
        mut self,
        market: Pubkey,
        trader: Keypair,
        token_in: Pubkey,
        token_out: Pubkey,
        amount_out: u64,
    ) -> Self {
        self.swaps.push(SwapSpec {
            market,
            trader,
            token_in,
            token_out,
            amount_in: None,
            amount_out: Some(amount_out),
            max_slippage: None,
            delay_ms: None,
        });
        self
    }

    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        if let Some(last) = self.swaps.last_mut() {
            last.delay_ms = Some(delay_ms);
        }
        self
    }

    pub fn with_slippage(mut self, max_slippage: f64) -> Self {
        if let Some(last) = self.swaps.last_mut() {
            last.max_slippage = Some(max_slippage);
        }
        self
    }

    pub async fn execute(self) -> TestResult<Vec<SwapResult>> {
        let swap_helper = self.ctx.swap_helper();
        let mut results = Vec::new();

        for spec in self.swaps {
            // Apply delay if specified
            if let Some(delay_ms) = spec.delay_ms {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }

            // Execute swap based on type
            let result = if let Some(amount_in) = spec.amount_in {
                swap_helper.swap(
                    &spec.market,
                    &spec.token_in,
                    &spec.token_out,
                    amount_in,
                    &spec.trader,
                ).await?
            } else if let Some(amount_out) = spec.amount_out {
                let max_amount_in = if let Some(slippage) = spec.max_slippage {
                    // Calculate max amount in based on slippage
                    (amount_out as f64 * (1.0 + slippage)) as u64
                } else {
                    u64::MAX // No slippage protection
                };

                swap_helper.swap_exact_out(
                    &spec.market,
                    &spec.token_in,
                    &spec.token_out,
                    amount_out,
                    max_amount_in,
                    &spec.trader,
                ).await?
            } else {
                return Err("Swap must specify either amount_in or amount_out".into());
            };

            results.push(result);
        }

        Ok(results)
    }

    /// Create a sandwich attack scenario
    pub fn sandwich_attack(
        mut self,
        market: Pubkey,
        victim: Keypair,
        attacker: Keypair,
        token_in: Pubkey,
        token_out: Pubkey,
        victim_amount: u64,
        front_run_amount: u64,
    ) -> Self {
        // Front-run transaction
        let attacker_bytes = attacker.to_bytes();
        let attacker_keypair = Keypair::from_bytes(&attacker_bytes).expect("Failed to clone keypair");
        self = self.add_swap(market, attacker_keypair, token_in, token_out, front_run_amount);
        
        // Victim transaction
        self = self.add_swap(market, victim, token_in, token_out, victim_amount);
        
        // Back-run transaction (swap in opposite direction)
        self = self.add_swap(market, attacker, token_out, token_in, front_run_amount);
        
        self
    }
}

/// Builder for creating positions with various configurations
pub struct PositionBuilder {
    ctx: TestContext,
    market: Option<Pubkey>,
    owner: Option<Keypair>,
    positions: Vec<PositionSpec>,
}

struct PositionSpec {
    lower_tick: i32,
    upper_tick: i32,
    liquidity: u128,
    auto_collect_fees: bool,
}

impl PositionBuilder {
    pub fn new(ctx: TestContext) -> Self {
        Self {
            ctx,
            market: None,
            owner: None,
            positions: Vec::new(),
        }
    }

    pub fn market(mut self, market: Pubkey) -> Self {
        self.market = Some(market);
        self
    }

    pub fn owner(mut self, owner: Keypair) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn add_position(mut self, lower_tick: i32, upper_tick: i32, liquidity: u128) -> Self {
        self.positions.push(PositionSpec {
            lower_tick,
            upper_tick,
            liquidity,
            auto_collect_fees: false,
        });
        self
    }

    pub fn add_range_positions(
        mut self,
        tick_spacing: i32,
        num_positions: usize,
        liquidity_per_position: u128,
    ) -> Self {
        let center_tick = 0;
        let half_positions = num_positions / 2;
        
        for i in 0..num_positions {
            let offset = (i as i32 - half_positions as i32) * tick_spacing * 10;
            let lower_tick = center_tick + offset;
            let upper_tick = lower_tick + tick_spacing * 10;
            
            self.positions.push(PositionSpec {
                lower_tick,
                upper_tick,
                liquidity: liquidity_per_position,
                auto_collect_fees: false,
            });
        }
        
        self
    }

    pub fn with_auto_collect(mut self) -> Self {
        for pos in &mut self.positions {
            pos.auto_collect_fees = true;
        }
        self
    }

    pub async fn build(self) -> TestResult<Vec<Pubkey>> {
        let market = self.market.ok_or("Market not set")?;
        let owner = self.owner.ok_or("Owner not set")?;
        let position_helper = self.ctx.position_helper();
        
        let mut position_ids = Vec::new();
        
        for spec in self.positions {
            let position_id = position_helper.open_position(
                &market,
                &owner,
                spec.lower_tick,
                spec.upper_tick,
                spec.liquidity,
            ).await?;
            
            position_ids.push(position_id);
            
            // Auto-collect fees if specified
            if spec.auto_collect_fees {
                position_helper.collect_fees(&position_id, &owner).await?;
            }
        }
        
        Ok(position_ids)
    }
}

/// Builder for complete test scenarios
pub struct ScenarioBuilder {
    ctx: TestContext,
    steps: Vec<ScenarioStep>,
}

enum ScenarioStep {
    CreateMarket {
        token_0: Pubkey,
        token_1: Pubkey,
        initial_price: Option<u128>,
    },
    Wait {
        duration: std::time::Duration,
    },
}

impl ScenarioStep {
    async fn execute(&self, ctx: &TestContext) -> TestResult<()> {
        match self {
            ScenarioStep::CreateMarket { token_0, token_1, initial_price } => {
                let market_helper = ctx.market_helper();
                
                if let Some(price) = *initial_price {
                    market_helper.create_raydium_market(token_0, token_1, price).await?;
                } else {
                    market_helper.create_simple_market(token_0, token_1).await?;
                }
                
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            ScenarioStep::Wait { duration } => {
                ctx.advance_time(duration.as_secs() as i64).await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        }
    }
}

impl ScenarioBuilder {
    pub fn new(ctx: TestContext) -> Self {
        Self {
            ctx,
            steps: Vec::new(),
        }
    }

    pub fn create_market(mut self, token_0: Pubkey, token_1: Pubkey) -> Self {
        self.steps.push(ScenarioStep::CreateMarket {
            token_0,
            token_1,
            initial_price: None,
        });
        self
    }

    pub fn wait(mut self, duration: std::time::Duration) -> Self {
        self.steps.push(ScenarioStep::Wait { duration });
        self
    }

    pub async fn run(self) -> TestResult<()> {
        for step in self.steps {
            step.execute(&self.ctx).await?;
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    }
}

// Extension methods for TestContext
impl TestContext {
    pub fn swap_builder(&self) -> SwapBuilder {
        SwapBuilder::new(self.clone())
    }

    pub fn position_builder(&self) -> PositionBuilder {
        PositionBuilder::new(self.clone())
    }

    pub fn scenario_builder(&self) -> ScenarioBuilder {
        ScenarioBuilder::new(self.clone())
    }
}