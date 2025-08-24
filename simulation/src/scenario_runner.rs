use anchor_lang::prelude::*;
use crate::{
    TestEnvironment, AccountFactory, TokenFactory, PoolFactory,
    LiquiditySimulator, SwapSimulator, TestAccount, TestToken, TestPool,
    LiquidityPosition, SwapExecution,
};

/// High-level scenario runner for complex test scenarios
pub struct ScenarioRunner {
    pub env: TestEnvironment,
}

impl ScenarioRunner {
    /// Create a new scenario runner
    pub async fn new() -> Result<Self> {
        let mut env = TestEnvironment::new().await?;
        
        // Initialize protocol
        env.initialize_protocol().await?;
        
        // Initialize FeelsSOL
        let feelssol_mint = solana_sdk::signature::Keypair::new().pubkey();
        let sol_mint = spl_token::native_mint::id(); // Using native SOL
        env.initialize_feelssol(&feelssol_mint, &sol_mint).await?;
        
        Ok(Self { env })
    }
    
    /// Run a basic AMM scenario
    pub async fn run_basic_amm_scenario(&mut self) -> Result<BasicAmmScenarioResult> {
        // Create accounts
        let mut account_factory = AccountFactory::new(&mut self.env);
        let lp = account_factory.create_liquidity_provider().await?;
        let trader1 = account_factory.create_trader().await?;
        let trader2 = account_factory.create_trader().await?;
        
        // Create tokens
        let mut token_factory = TokenFactory::new(&mut self.env);
        let token_a = token_factory.create_feels_token(
            "Test Token A".to_string(),
            "TTA".to_string(),
            9,
            &lp,
        ).await?;
        
        let token_b = token_factory.create_feels_token(
            "Test Token B".to_string(),
            "TTB".to_string(),
            9,
            &lp,
        ).await?;
        
        // Mint tokens to liquidity provider
        let mint_amount = 1_000_000 * 10u64.pow(9); // 1M tokens
        token_factory.mint_to(&token_a, &lp.pubkey(), mint_amount, &lp.keypair).await?;
        token_factory.mint_to(&token_b, &lp.pubkey(), mint_amount, &lp.keypair).await?;
        
        // Mint tokens to traders
        let trader_amount = 10_000 * 10u64.pow(9); // 10k tokens
        token_factory.mint_to(&token_a, &trader1.pubkey(), trader_amount, &lp.keypair).await?;
        token_factory.mint_to(&token_b, &trader2.pubkey(), trader_amount, &lp.keypair).await?;
        
        // Create pool
        let mut pool_factory = PoolFactory::new(&mut self.env);
        let pool = pool_factory.create_standard_pool(&token_a, &token_b, 1.0).await?;
        
        // Add liquidity
        let mut liquidity_sim = LiquiditySimulator::new(&mut self.env);
        let position = liquidity_sim.add_liquidity_around_price(
            &pool,
            &lp,
            100_000 * 10u64.pow(9), // 100k token A
            100_000 * 10u64.pow(9), // 100k token B
            10.0, // 10% range
        ).await?;
        
        // Execute swaps
        let mut swap_sim = SwapSimulator::new(&mut self.env);
        
        let swap1 = swap_sim.swap_exact_input(
            &pool,
            &trader1,
            1_000 * 10u64.pow(9), // 1k tokens
            true, // base input (A -> B)
            50, // 0.5% slippage
        ).await?;
        
        let swap2 = swap_sim.swap_exact_input(
            &pool,
            &trader2,
            500 * 10u64.pow(9), // 500 tokens
            false, // quote input (B -> A)
            50,
        ).await?;
        
        Ok(BasicAmmScenarioResult {
            pool,
            position,
            swaps: vec![swap1, swap2],
        })
    }
    
    /// Run a liquidity provision scenario
    pub async fn run_liquidity_scenario(&mut self) -> Result<LiquidityScenarioResult> {
        // Create multiple liquidity providers
        let mut account_factory = AccountFactory::new(&mut self.env);
        let lps = account_factory.create_accounts(5, 100_000_000_000).await?;
        
        // Create tokens
        let mut token_factory = TokenFactory::new(&mut self.env);
        let token_a = token_factory.create_feels_token(
            "USDC".to_string(),
            "USDC".to_string(),
            6,
            &lps[0],
        ).await?;
        
        let token_b = token_factory.create_feels_token(
            "ETH".to_string(),
            "ETH".to_string(),
            9,
            &lps[0],
        ).await?;
        
        // Mint tokens to all LPs
        for lp in &lps {
            token_factory.mint_to(&token_a, &lp.pubkey(), 1_000_000 * 10u64.pow(6), &lps[0].keypair).await?;
            token_factory.mint_to(&token_b, &lp.pubkey(), 100 * 10u64.pow(9), &lps[0].keypair).await?;
        }
        
        // Create pool with initial price of 2000 USDC/ETH
        let mut pool_factory = PoolFactory::new(&mut self.env);
        let pool = pool_factory.create_standard_pool(&token_a, &token_b, 2000.0).await?;
        
        // Add liquidity at different ranges
        let mut liquidity_sim = LiquiditySimulator::new(&mut self.env);
        let mut positions = Vec::new();
        
        // Full range position
        positions.push(
            liquidity_sim.add_full_range_liquidity(
                &pool,
                &lps[0],
                100_000 * 10u64.pow(6), // 100k USDC
                50 * 10u64.pow(9), // 50 ETH
            ).await?
        );
        
        // Narrow range positions
        for (i, lp) in lps[1..4].iter().enumerate() {
            let range = 5.0 + (i as f64 * 2.0); // 5%, 7%, 9% ranges
            positions.push(
                liquidity_sim.add_liquidity_around_price(
                    &pool,
                    lp,
                    50_000 * 10u64.pow(6),
                    25 * 10u64.pow(9),
                    range,
                ).await?
            );
        }
        
        // Out of range position
        positions.push(
            liquidity_sim.add_liquidity(
                &pool,
                &lps[4],
                20_000 * 10u64.pow(6),
                10 * 10u64.pow(9),
                2200.0, // Above current price
                2500.0,
            ).await?
        );
        
        Ok(LiquidityScenarioResult {
            pool,
            positions,
        })
    }
    
    /// Run a stress test scenario
    pub async fn run_stress_test(&mut self, num_swaps: usize) -> Result<StressTestResult> {
        // Setup basic pool
        let mut account_factory = AccountFactory::new(&mut self.env);
        let lp = account_factory.create_liquidity_provider().await?;
        let traders = account_factory.create_accounts(10, 10_000_000_000).await?;
        
        let mut token_factory = TokenFactory::new(&mut self.env);
        let token_a = token_factory.create_feels_token(
            "Token A".to_string(),
            "TKA".to_string(),
            9,
            &lp,
        ).await?;
        
        let token_b = token_factory.create_feels_token(
            "Token B".to_string(),
            "TKB".to_string(),
            9,
            &lp,
        ).await?;
        
        // Setup liquidity
        token_factory.mint_to(&token_a, &lp.pubkey(), 10_000_000 * 10u64.pow(9), &lp.keypair).await?;
        token_factory.mint_to(&token_b, &lp.pubkey(), 10_000_000 * 10u64.pow(9), &lp.keypair).await?;
        
        for trader in &traders {
            token_factory.mint_to(&token_a, &trader.pubkey(), 100_000 * 10u64.pow(9), &lp.keypair).await?;
            token_factory.mint_to(&token_b, &trader.pubkey(), 100_000 * 10u64.pow(9), &lp.keypair).await?;
        }
        
        let mut pool_factory = PoolFactory::new(&mut self.env);
        let pool = pool_factory.create_standard_pool(&token_a, &token_b, 1.0).await?;
        
        let mut liquidity_sim = LiquiditySimulator::new(&mut self.env);
        liquidity_sim.add_full_range_liquidity(
            &pool,
            &lp,
            1_000_000 * 10u64.pow(9),
            1_000_000 * 10u64.pow(9),
        ).await?;
        
        // Execute random swaps
        let mut swap_sim = SwapSimulator::new(&mut self.env);
        let mut swaps = Vec::new();
        let start_time = std::time::Instant::now();
        
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        for i in 0..num_swaps {
            let trader = &traders[i % traders.len()];
            let amount = rng.gen_range(100..10_000) * 10u64.pow(9);
            let is_base_input = rng.gen_bool(0.5);
            
            let swap = swap_sim.swap_exact_input(
                &pool,
                trader,
                amount,
                is_base_input,
                100, // 1% slippage
            ).await?;
            
            swaps.push(swap);
        }
        
        let elapsed = start_time.elapsed();
        
        Ok(StressTestResult {
            num_swaps,
            total_time: elapsed,
            average_time_per_swap: elapsed / num_swaps as u32,
        })
    }
}

/// Result of basic AMM scenario
pub struct BasicAmmScenarioResult {
    pub pool: TestPool,
    pub position: LiquidityPosition,
    pub swaps: Vec<SwapExecution>,
}

/// Result of liquidity scenario
pub struct LiquidityScenarioResult {
    pub pool: TestPool,
    pub positions: Vec<LiquidityPosition>,
}

/// Result of stress test
pub struct StressTestResult {
    pub num_swaps: usize,
    pub total_time: std::time::Duration,
    pub average_time_per_swap: std::time::Duration,
}