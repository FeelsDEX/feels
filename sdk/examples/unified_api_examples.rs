/// Examples demonstrating the simplified unified API for the Feels Protocol
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::solana_sdk::signer::Signer;
use feels_sdk::client::FeelsClient;
use feels_sdk::types::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let payer = Keypair::new();
    let client = FeelsClient::new(
        "https://api.devnet.solana.com".to_string(),
        payer,
    )?;

    // Example pool for demonstrations
    let pool_pubkey = "ExamplePoolPubkey11111111111111111111111111".parse()?;
    
    // ============================================================================
    // Unified Order Examples
    // ============================================================================
    
    println!("=== Unified Order API Examples ===\n");
    
    // Example 1: Simple Swap
    println!("1. Simple Swap (Token A to Token B)");
    let swap_params = UnifiedOrderParams {
        amount: 1_000_000, // 1 token (assuming 6 decimals)
        config: OrderConfig::Swap {
            is_token_a_to_b: true,
            min_amount_out: 950_000, // 5% slippage tolerance
            sqrt_rate_limit: None, // Use default (no limit)
        },
        advanced: None, // Use all defaults
    };
    // let swap_result = client.order_unified(pool_pubkey, swap_params).await?;
    println!("Swap params: {:?}\n", swap_params);
    
    // Example 2: Swap with Advanced Parameters
    println!("2. Leveraged Swap with MEV Protection");
    let advanced_swap_params = UnifiedOrderParams {
        amount: 1_000_000,
        config: OrderConfig::Swap {
            is_token_a_to_b: false, // B to A
            min_amount_out: 980_000, // 2% slippage
            sqrt_rate_limit: Some(79_228_162_514_264_337_593_543_950_336), // Specific rate limit
        },
        advanced: Some(AdvancedOrderParams {
            duration: Duration::Swap, // Immediate
            leverage: 3_000_000, // 3x leverage
            mev_protection: Some(MevProtection {
                max_slippage_bps: 100, // 1% max slippage
                min_blocks_delay: 2, // Wait 2 blocks
                validator_signature: None,
            }),
            hook_data: None,
        }),
    };
    println!("Advanced swap params: {:?}\n", advanced_swap_params);
    
    // Example 3: Add Liquidity
    println!("3. Add Liquidity (Full Range)");
    let liquidity_params = UnifiedOrderParams {
        amount: 10_000_000, // 10 tokens
        config: OrderConfig::AddLiquidity {
            tick_lower: -887_272, // Full range lower bound
            tick_upper: 887_272,   // Full range upper bound
            token_amounts: None,   // Let protocol calculate optimal amounts
        },
        advanced: None,
    };
    println!("Liquidity params: {:?}\n", liquidity_params);
    
    // Example 4: Add Concentrated Liquidity with Leverage
    println!("4. Add Concentrated Liquidity with Leverage");
    let concentrated_liquidity_params = UnifiedOrderParams {
        amount: 5_000_000,
        config: OrderConfig::AddLiquidity {
            tick_lower: -5000,  // Narrow range around current price
            tick_upper: 5000,
            token_amounts: Some((5_000_000, 5_000_000)), // Specific amounts
        },
        advanced: Some(AdvancedOrderParams {
            duration: Duration::Weekly, // Lock for 1 week
            leverage: 2_000_000, // 2x leverage
            mev_protection: None,
            hook_data: None,
        }),
    };
    println!("Concentrated liquidity params: {:?}\n", concentrated_liquidity_params);
    
    // Example 5: Create Limit Order
    println!("5. Create Limit Order");
    let limit_order_params = UnifiedOrderParams {
        amount: 2_000_000,
        config: OrderConfig::LimitOrder {
            is_buy: true, // Buy order
            target_sqrt_rate: 70_710_678_118_654_752_440_084_436_210_485, // sqrt(0.5) = 0.707...
            expiry: 1735689600, // Unix timestamp for expiry
        },
        advanced: Some(AdvancedOrderParams {
            duration: Duration::Monthly, // Valid for 1 month
            leverage: 1_000_000, // No leverage for limit orders
            mev_protection: None,
            hook_data: Some(vec![1, 2, 3, 4]), // Custom hook data
        }),
    };
    println!("Limit order params: {:?}\n", limit_order_params);
    
    // Example 6: Flash Loan
    println!("6. Flash Loan");
    let flash_loan_params = UnifiedOrderParams {
        amount: 100_000_000, // 100 tokens
        config: OrderConfig::FlashLoan {
            borrow_token_a: true,
            callback_program: "CallbackProgram11111111111111111111111111111".parse()?,
            callback_data: vec![0x01, 0x02, 0x03], // Arbitrary callback data
        },
        advanced: None, // Flash loans always use Duration::Flash
    };
    println!("Flash loan params: {:?}\n", flash_loan_params);
    
    // ============================================================================
    // Pool Configuration Examples
    // ============================================================================
    
    println!("\n=== Pool Configuration API Examples ===\n");
    
    // Example 7: Enable Leverage
    println!("7. Enable Leverage for Pool");
    let enable_leverage_params = PoolConfigParams::Leverage(LeverageConfig {
        operation: LeverageOperation::Enable,
        max_leverage: Some(10_000_000), // 10x max
        current_ceiling: Some(5_000_000), // Start with 5x ceiling
        protection_curve: Some(ProtectionCurveConfig {
            curve_type: 1, // Exponential
            decay_rate: Some(50_000), // 5% decay
            points: None,
        }),
    });
    println!("Enable leverage params: {:?}\n", enable_leverage_params);
    
    // Example 8: Update Dynamic Fees
    println!("8. Update Dynamic Fee Configuration");
    let fee_config_params = PoolConfigParams::DynamicFees(DynamicFeeConfig {
        base_fee: 30, // 0.30% base fee
        min_fee: 5,   // 0.05% minimum
        max_fee: 100, // 1.00% maximum
        volatility_coefficient: 1_500_000, // 1.5x volatility multiplier
        volume_discount_threshold: 10_000_000_000_000, // $10M volume threshold
        min_multiplier: 5000,  // 0.5x minimum multiplier
        max_multiplier: 20000, // 2.0x maximum multiplier
        _padding: [0; 6],
    });
    println!("Fee config params: {:?}\n", fee_config_params);
    
    // Example 9: Register Hook
    println!("9. Register a Hook");
    let hook_params = PoolConfigParams::Hook(HookConfig::Register {
        hook_program: "HookProgram11111111111111111111111111111111".parse()?,
        permission: HookPermission::ReadOnly,
        event_mask: 0b00001111, // Subscribe to swap and liquidity events
        stage_mask: 0b0011,     // Run in validation and execution stages
    });
    println!("Hook registration params: {:?}\n", hook_params);
    
    // Example 10: Batch Configuration
    println!("10. Batch Multiple Configurations");
    let batch_params = PoolConfigParams::Batch(vec![
        // Update leverage ceiling
        PoolConfigParams::Leverage(LeverageConfig {
            operation: LeverageOperation::Update,
            max_leverage: None, // Keep existing
            current_ceiling: Some(7_000_000), // Raise to 7x
            protection_curve: None, // Keep existing curve
        }),
        // Update fees
        PoolConfigParams::DynamicFees(DynamicFeeConfig {
            base_fee: 25, // Lower base fee to 0.25%
            min_fee: 5,
            max_fee: 100,
            volatility_coefficient: 1_000_000,
            volume_discount_threshold: 5_000_000_000_000,
            min_multiplier: 5000,
            max_multiplier: 20000,
            _padding: [0; 6],
        }),
        // Toggle hooks
        PoolConfigParams::Hook(HookConfig::Toggle { enabled: true }),
    ]);
    println!("Batch config params: {:?}\n", batch_params);
    
    // ============================================================================
    // Order Modification Examples
    // ============================================================================
    
    println!("\n=== Order Modification Examples ===\n");
    
    // Example 11: Modify Liquidity Position
    println!("11. Add More Liquidity to Position");
    let modify_params = UnifiedModifyParams {
        target: ModifyTarget::Position("Position11111111111111111111111111111111111".parse()?),
        modification: OrderModification::Update {
            amount: Some(5_000_000), // Add 5 more tokens
            rate: None,
            leverage: Some(3_000_000), // Increase leverage to 3x
            duration: None,
        },
    };
    println!("Modify params: {:?}\n", modify_params);
    
    // Example 12: Cancel Limit Order
    println!("12. Cancel Limit Order");
    let cancel_params = UnifiedModifyParams {
        target: ModifyTarget::Order("Order111111111111111111111111111111111111111".parse()?),
        modification: OrderModification::Cancel,
    };
    println!("Cancel params: {:?}\n", cancel_params);
    
    // Example 13: Batch Modify Orders
    println!("13. Batch Modify All Limit Orders in Range");
    let batch_modify_params = UnifiedModifyParams {
        target: ModifyTarget::Batch(BatchCriteria {
            pool: pool_pubkey,
            order_type: Some(OrderTypeFilter::Limit),
            rate_range: Some((-1000, 1000)), // Orders near current price
        }),
        modification: OrderModification::Update {
            amount: None,
            rate: Some(RateUpdate::TargetRate(80_000_000_000_000_000_000_000_000_000_000_000)),
            leverage: None,
            duration: Some(Duration::Weekly), // Extend duration
        },
    };
    println!("Batch modify params: {:?}\n", batch_modify_params);
    
    // ============================================================================
    // Order Computation Examples
    // ============================================================================
    
    println!("\n=== Order Computation Examples ===\n");
    
    // Example 14: Compute Swap Route
    println!("14. Compute Optimal Swap Route");
    let compute_swap_params = UnifiedComputeParams {
        order_config: OrderConfig::Swap {
            is_token_a_to_b: true,
            min_amount_out: 0, // Just computing, not executing
            sqrt_rate_limit: None,
        },
        route_preference: Some(RoutePreference::MostLiquid),
    };
    println!("Compute swap params: {:?}\n", compute_swap_params);
    
    // Example 15: Compute with Specific Route
    println!("15. Compute with Specific Route");
    let specific_route_params = UnifiedComputeParams {
        order_config: OrderConfig::Swap {
            is_token_a_to_b: false,
            min_amount_out: 0,
            sqrt_rate_limit: None,
        },
        route_preference: Some(RoutePreference::Specific(vec![
            "Pool1111111111111111111111111111111111111111".parse()?,
            "Pool2222222222222222222222222222222222222222".parse()?,
        ])),
    };
    println!("Specific route params: {:?}\n", specific_route_params);
    
    println!("=== Examples Complete ===");
    
    Ok(())
}

// Mock type definitions (these would come from the actual SDK)
#[derive(Debug)]
struct UnifiedOrderParams {
    amount: u64,
    config: OrderConfig,
    advanced: Option<AdvancedOrderParams>,
}

#[derive(Debug)]
enum OrderConfig {
    Swap {
        is_token_a_to_b: bool,
        min_amount_out: u64,
        sqrt_rate_limit: Option<u128>,
    },
    AddLiquidity {
        tick_lower: i32,
        tick_upper: i32,
        token_amounts: Option<(u64, u64)>,
    },
    LimitOrder {
        is_buy: bool,
        target_sqrt_rate: u128,
        expiry: i64,
    },
    FlashLoan {
        borrow_token_a: bool,
        callback_program: Pubkey,
        callback_data: Vec<u8>,
    },
}

#[derive(Debug)]
struct AdvancedOrderParams {
    duration: Duration,
    leverage: u64,
    mev_protection: Option<MevProtection>,
    hook_data: Option<Vec<u8>>,
}

#[derive(Debug)]
struct MevProtection {
    max_slippage_bps: u16,
    min_blocks_delay: u8,
    validator_signature: Option<[u8; 64]>,
}

#[derive(Debug)]
enum Duration {
    Flash,
    Swap,
    Weekly,
    Monthly,
}

#[derive(Debug)]
enum PoolConfigParams {
    Leverage(LeverageConfig),
    DynamicFees(DynamicFeeConfig),
    Hook(HookConfig),
    Batch(Vec<PoolConfigParams>),
}

#[derive(Debug)]
struct LeverageConfig {
    operation: LeverageOperation,
    max_leverage: Option<u64>,
    current_ceiling: Option<u64>,
    protection_curve: Option<ProtectionCurveConfig>,
}

#[derive(Debug)]
enum LeverageOperation {
    Enable,
    Update,
}

#[derive(Debug)]
struct ProtectionCurveConfig {
    curve_type: u8,
    decay_rate: Option<u64>,
    points: Option<[[u64; 2]; 8]>,
}

#[derive(Debug)]
struct DynamicFeeConfig {
    base_fee: u16,
    min_fee: u16,
    max_fee: u16,
    volatility_coefficient: u64,
    volume_discount_threshold: u128,
    min_multiplier: u16,
    max_multiplier: u16,
    _padding: [u8; 6],
}

#[derive(Debug)]
enum HookConfig {
    Register {
        hook_program: Pubkey,
        permission: HookPermission,
        event_mask: u32,
        stage_mask: u32,
    },
    Toggle {
        enabled: bool,
    },
}

#[derive(Debug)]
enum HookPermission {
    ReadOnly,
}

#[derive(Debug)]
struct UnifiedModifyParams {
    target: ModifyTarget,
    modification: OrderModification,
}

#[derive(Debug)]
enum ModifyTarget {
    Order(Pubkey),
    Position(Pubkey),
    Batch(BatchCriteria),
}

#[derive(Debug)]
struct BatchCriteria {
    pool: Pubkey,
    order_type: Option<OrderTypeFilter>,
    rate_range: Option<(i32, i32)>,
}

#[derive(Debug)]
enum OrderTypeFilter {
    Limit,
}

#[derive(Debug)]
enum OrderModification {
    Cancel,
    Update {
        amount: Option<u64>,
        rate: Option<RateUpdate>,
        leverage: Option<u64>,
        duration: Option<Duration>,
    },
}

#[derive(Debug)]
enum RateUpdate {
    TargetRate(u128),
}

#[derive(Debug)]
struct UnifiedComputeParams {
    order_config: OrderConfig,
    route_preference: Option<RoutePreference>,
}

#[derive(Debug)]
enum RoutePreference {
    MostLiquid,
    Specific(Vec<Pubkey>),
}

use solana_program::pubkey::Pubkey;