use super::*;
use std::sync::OnceLock;

/// Common test constants and configurations
pub mod test_constants {

    // Token decimals
    pub const JITOSOL_DECIMALS: u8 = 9;
    pub const FEELSSOL_DECIMALS: u8 = 9;
    pub const TEST_TOKEN_DECIMALS: u8 = 6;

    // Common amounts
    pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
    pub const DEFAULT_AIRDROP: u64 = 10 * LAMPORTS_PER_SOL;
    pub const SMALL_SWAP_AMOUNT: u64 = 100_000; // 0.0001 token
    pub const MEDIUM_SWAP_AMOUNT: u64 = 1_000_000; // 1 token
    pub const LARGE_SWAP_AMOUNT: u64 = 100_000_000; // 100 tokens

    // Tick spacing configurations
    pub const STABLE_TICK_SPACING: u16 = 1;
    pub const LOW_FEE_TICK_SPACING: u16 = 8;
    pub const MEDIUM_FEE_TICK_SPACING: u16 = 64;
    pub const HIGH_FEE_TICK_SPACING: u16 = 128;

    // Fee tiers (in basis points)
    pub const STABLE_FEE_TIER: u16 = 50; // 0.5 bps
    pub const LOW_FEE_TIER: u16 = 500; // 5 bps
    pub const MEDIUM_FEE_TIER: u16 = 3000; // 30 bps
    pub const HIGH_FEE_TIER: u16 = 10000; // 100 bps

    // Common tick values
    pub const MIN_TICK: i32 = -443636;
    pub const MAX_TICK: i32 = 443636;
    pub const PRICE_1_TO_1_TICK: i32 = 0;

    // Common sqrt prices (Q64.64 format)
    pub const MIN_SQRT_PRICE: u128 = 4295048016;
    pub const MAX_SQRT_PRICE: u128 = 79226673515401279992447579055;
    pub const PRICE_1_TO_1: u128 = 79228162514264337593543950336; // sqrt(1) in Q64.64
}

/// Pre-configured test accounts
pub struct TestAccounts {
    pub alice: Keypair,
    pub bob: Keypair,
    pub charlie: Keypair,
    pub market_creator: Keypair,
    pub fee_authority: Keypair,
}

impl Default for TestAccounts {
    fn default() -> Self {
        // Use deterministic keypairs for reproducible tests
        use solana_sdk::signature::keypair_from_seed;

        Self {
            alice: keypair_from_seed(&[1; 32]).unwrap(),
            bob: keypair_from_seed(&[2; 32]).unwrap(),
            charlie: keypair_from_seed(&[3; 32]).unwrap(),
            market_creator: keypair_from_seed(&[4; 32]).unwrap(),
            fee_authority: keypair_from_seed(&[5; 32]).unwrap(),
        }
    }
}

static TEST_ACCOUNTS: OnceLock<TestAccounts> = OnceLock::new();

pub fn get_test_accounts() -> &'static TestAccounts {
    TEST_ACCOUNTS.get_or_init(TestAccounts::default)
}

/// Common market configurations
#[derive(Debug, Clone)]
pub struct MarketConfig {
    pub name: &'static str,
    pub tick_spacing: u16,
    pub fee_tier: u16,
    pub initial_price: u128,
    pub description: &'static str,
}

pub fn get_market_configs() -> Vec<MarketConfig> {
    vec![
        MarketConfig {
            name: "stable_pool",
            tick_spacing: test_constants::STABLE_TICK_SPACING,
            fee_tier: test_constants::STABLE_FEE_TIER,
            initial_price: test_constants::PRICE_1_TO_1,
            description: "Stablecoin pool with minimal fees",
        },
        MarketConfig {
            name: "standard_pool",
            tick_spacing: test_constants::MEDIUM_FEE_TICK_SPACING,
            fee_tier: test_constants::LOW_FEE_TIER,
            initial_price: test_constants::PRICE_1_TO_1,
            description: "Standard pool for most token pairs",
        },
        MarketConfig {
            name: "volatile_pool",
            tick_spacing: test_constants::HIGH_FEE_TICK_SPACING,
            fee_tier: test_constants::HIGH_FEE_TIER,
            initial_price: test_constants::PRICE_1_TO_1,
            description: "High fee pool for volatile pairs",
        },
    ]
}

/// Common liquidity distributions
#[derive(Debug, Clone)]
pub struct LiquidityDistribution {
    pub name: &'static str,
    pub positions: Vec<(i32, i32, u128)>, // (tick_lower, tick_upper, liquidity)
}

pub fn get_liquidity_distributions(tick_spacing: u16) -> Vec<LiquidityDistribution> {
    let spacing = tick_spacing as i32;

    vec![
        LiquidityDistribution {
            name: "concentrated",
            positions: vec![(-spacing * 10, spacing * 10, 1_000_000_000)],
        },
        LiquidityDistribution {
            name: "normal",
            positions: vec![
                (-spacing * 100, spacing * 100, 500_000_000),
                (-spacing * 50, spacing * 50, 1_000_000_000),
                (-spacing * 20, spacing * 20, 2_000_000_000),
            ],
        },
        LiquidityDistribution {
            name: "wide",
            positions: vec![
                (-spacing * 1000, spacing * 1000, 100_000_000),
                (-spacing * 500, spacing * 500, 200_000_000),
                (-spacing * 200, spacing * 200, 500_000_000),
                (-spacing * 100, spacing * 100, 1_000_000_000),
            ],
        },
        LiquidityDistribution {
            name: "skewed_buy",
            positions: vec![
                (-spacing * 200, -spacing * 50, 2_000_000_000),
                (-spacing * 100, spacing * 20, 1_000_000_000),
                (spacing * 10, spacing * 100, 500_000_000),
            ],
        },
        LiquidityDistribution {
            name: "skewed_sell",
            positions: vec![
                (-spacing * 100, -spacing * 10, 500_000_000),
                (-spacing * 20, spacing * 100, 1_000_000_000),
                (spacing * 50, spacing * 200, 2_000_000_000),
            ],
        },
    ]
}

/// Test swap scenarios
#[derive(Debug, Clone)]
pub struct SwapScenario {
    pub name: &'static str,
    pub swaps: Vec<SwapConfig>,
    pub expected_behavior: &'static str,
}

#[derive(Debug, Clone)]
pub struct SwapConfig {
    pub amount: u64,
    pub zero_for_one: bool,
    pub price_limit: Option<u128>,
}

pub fn get_swap_scenarios() -> Vec<SwapScenario> {
    vec![
        SwapScenario {
            name: "small_swaps",
            swaps: vec![
                SwapConfig {
                    amount: test_constants::SMALL_SWAP_AMOUNT,
                    zero_for_one: true,
                    price_limit: None,
                },
                SwapConfig {
                    amount: test_constants::SMALL_SWAP_AMOUNT,
                    zero_for_one: false,
                    price_limit: None,
                },
            ],
            expected_behavior: "Minimal price impact, single tick",
        },
        SwapScenario {
            name: "tick_crossing",
            swaps: vec![SwapConfig {
                amount: test_constants::LARGE_SWAP_AMOUNT,
                zero_for_one: true,
                price_limit: None,
            }],
            expected_behavior: "Multiple tick crossings, fee growth updates",
        },
        SwapScenario {
            name: "price_limit_hit",
            swaps: vec![SwapConfig {
                amount: test_constants::LARGE_SWAP_AMOUNT * 10,
                zero_for_one: true,
                price_limit: Some(test_constants::PRICE_1_TO_1 / 2),
            }],
            expected_behavior: "Partial fill at price limit",
        },
        SwapScenario {
            name: "liquidity_exhaustion",
            swaps: vec![SwapConfig {
                amount: u64::MAX,
                zero_for_one: true,
                price_limit: None,
            }],
            expected_behavior: "Swap all available liquidity to bound",
        },
    ]
}

/// Helper to create a standard test environment
pub async fn create_standard_test_env(ctx: &TestContext) -> TestResult<StandardTestEnv> {
    let accounts = get_test_accounts();

    // Airdrop SOL to test accounts
    ctx.airdrop(&accounts.alice.pubkey(), test_constants::DEFAULT_AIRDROP)
        .await?;
    ctx.airdrop(&accounts.bob.pubkey(), test_constants::DEFAULT_AIRDROP)
        .await?;
    ctx.airdrop(
        &accounts.market_creator.pubkey(),
        test_constants::DEFAULT_AIRDROP,
    )
    .await?;

    // Create mints
    let jitosol_mint = ctx
        .create_mint(
            &accounts.market_creator.pubkey(),
            test_constants::JITOSOL_DECIMALS,
        )
        .await?;

    let feelssol_mint = ctx
        .create_mint(
            &accounts.market_creator.pubkey(),
            test_constants::FEELSSOL_DECIMALS,
        )
        .await?;

    let test_token_mint = ctx
        .create_mint(
            &accounts.market_creator.pubkey(),
            test_constants::TEST_TOKEN_DECIMALS,
        )
        .await?;

    Ok(StandardTestEnv {
        jitosol_mint,
        feelssol_mint,
        test_token_mint,
    })
}

#[derive(Debug)]
pub struct StandardTestEnv {
    pub jitosol_mint: Keypair,
    pub feelssol_mint: Keypair,
    pub test_token_mint: Keypair,
}
