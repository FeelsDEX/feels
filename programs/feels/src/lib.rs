
/// Feels Protocol - Concentrated Liquidity AMM
/// A next-generation automated market maker implementing concentrated liquidity positions
/// with advanced features like hooks, Token-2022 support, and oracle rate feeds.
/// Built on Solana for high-performance decentralized trading.
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint, TokenAccount};

pub mod constant;
pub mod error;
pub mod instructions;
pub mod logic;
pub mod state;
pub mod utils;

// Import logic modules
use logic::tick::TickManager;

// Import error types
pub use error::{FeelsError, FeelsProtocolError};

// Import all state types explicitly
use state::{
    // Core types
    Pool,
    ProtocolState,
    TickArray,
    TokenMetadata,
    FeelsSOL,
    PoolMetrics,
    PoolHooks,
    PoolRebase,
    // Hook types
    HookRegistry,
    HookMessageQueue,
    HookType,
    HookPermission,
    // Position types
    TickPositionMetadata,
    // Leverage types
    // Metrics types
    Oracle,
    OracleData,
};

// Re-export instruction types for SDK
pub use instructions::{
    OrderParams, 
    OrderResult, 
    OrderType, 
    RateParams,
    OrderComputeParams,
    RateComputeParams,
};

// Re-export Duration from state
pub use state::duration::Duration;

// Import instruction contexts

// Required for Anchor's #[program] macro
#[doc(hidden)]
pub mod __client_accounts_crate;
#[doc(hidden)]
pub mod __client_accounts_instructions;

// Import instruction contexts

// Import instruction contexts

declare_id!("Fee1sProtoco11111111111111111111111111111111");

#[derive(Accounts)]
pub struct InitializeFeels<'info> {
    /// Protocol state account
    #[account(
        init,
        payer = authority,
        space = ProtocolState::SIZE,
        seeds = [b"protocol"],
        bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,

    /// Protocol authority and payer
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Protocol treasury account
    /// CHECK: Can be any account that will receive fees
    pub treasury: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeFeelsSOL<'info> {
    /// FeelsSOL wrapper account
    #[account(
        init,
        payer = authority,
        space = FeelsSOL::SIZE,
        seeds = [b"feelssol"],
        bump
    )]
    pub feelssol: Account<'info, FeelsSOL>,

    /// FeelsSOL Token-2022 mint
    #[account(
        init,
        payer = authority,
        mint::decimals = 9,
        mint::authority = feelssol,
        mint::freeze_authority = feelssol,
    )]
    pub feels_mint: InterfaceAccount<'info, Mint>,
    
    /// Underlying token mint
    pub underlying_mint: InterfaceAccount<'info, Mint>,
    
    /// Vault to hold underlying tokens
    #[account(
        init,
        payer = authority,
        token::mint = underlying_mint,
        token::authority = feelssol,
        seeds = [b"feelssol_vault", underlying_mint.key().as_ref()],
        bump
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    /// Protocol authority
    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(ticker: String, name: String, symbol: String, decimals: u8, initial_supply: u64)]
pub struct CreateToken<'info> {
    /// New token mint to create
    #[account(
        init,
        payer = authority,
        mint::decimals = decimals,
        mint::authority = authority,
        mint::freeze_authority = authority,
    )]
    pub token_mint: InterfaceAccount<'info, Mint>,

    /// Token metadata account to store ticker, name, symbol
    #[account(
        init,
        payer = authority,
        space = TokenMetadata::SIZE,
        seeds = [
            b"token_metadata",
            token_mint.key().as_ref()
        ],
        bump
    )]
    pub token_metadata: Account<'info, TokenMetadata>,

    /// Authority's token account for initial mint
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = token_mint,
        associated_token::authority = authority,
    )]
    pub authority_token_account: InterfaceAccount<'info, TokenAccount>,

    /// Token create authority (becomes mint authority)
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Required programs
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(fee_rate: u16, initial_sqrt_rate: u128, base_rate: u16, protocol_share: u16)]
pub struct InitializePool<'info> {
    /// The pool account to initialize
    /// Seeds are validated to ensure canonical token ordering
    #[account(
        init,
        payer = authority,
        space = Pool::SIZE,
        seeds = [
            b"pool",
            token_a_mint.key().as_ref(),
            token_b_mint.key().as_ref(),
            fee_rate.to_le_bytes().as_ref()
        ],
        bump
    )]
    pub pool: AccountLoader<'info, Pool>,

    // Fee configuration now handled through GradientCache and BufferAccount

    /// Token A mint (order doesn't matter - will be canonicalized)
    pub token_a_mint: InterfaceAccount<'info, Mint>,

    /// Token B mint (order doesn't matter - will be canonicalized)
    pub token_b_mint: InterfaceAccount<'info, Mint>,

    /// FeelsSOL wrapper account for validation
    #[account(
        seeds = [b"feelssol"],
        bump,
        constraint = feelssol.feels_mint == token_a_mint.key() || feelssol.feels_mint == token_b_mint.key() @ FeelsProtocolError::NotFeelsSOLPair
    )]
    pub feelssol: Account<'info, FeelsSOL>,

    /// Token A vault
    #[account(
        init,
        payer = authority,
        token::mint = token_a_mint,
        token::authority = pool,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            token_a_mint.key().as_ref(),
        ],
        bump
    )]
    pub token_a_vault: InterfaceAccount<'info, TokenAccount>,

    /// Token B vault
    #[account(
        init,
        payer = authority,
        token::mint = token_b_mint,
        token::authority = pool,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            token_b_mint.key().as_ref(),
        ],
        bump
    )]
    pub token_b_vault: InterfaceAccount<'info, TokenAccount>,

    /// Unified oracle for rate tracking and volatility
    #[account(
        init,
        payer = authority,
        space = Oracle::SIZE,
        seeds = [b"oracle", pool.key().as_ref()],
        bump
    )]
    pub oracle: Box<Account<'info, Oracle>>,

    /// Oracle data storage for observation buffer
    #[account(
        init,
        payer = authority,
        space = OracleData::SIZE,
        seeds = [b"oracle_data", pool.key().as_ref()],
        bump
    )]
    pub oracle_data: AccountLoader<'info, OracleData>,

    /// Pool metrics account for cold data
    #[account(
        init,
        payer = authority,
        space = PoolMetrics::SIZE,
        seeds = [b"pool_metrics", pool.key().as_ref()],
        bump
    )]
    pub pool_metrics: Box<Account<'info, PoolMetrics>>,

    /// Pool hooks account for hook configuration
    #[account(
        init,
        payer = authority,
        space = PoolHooks::SIZE,
        seeds = [b"pool_hooks", pool.key().as_ref()],
        bump
    )]
    pub pool_hooks: Box<Account<'info, PoolHooks>>,

    /// Pool rebase account for rebase state
    #[account(
        init,
        payer = authority,
        space = PoolRebase::SIZE,
        seeds = [b"pool_rebase", pool.key().as_ref()],
        bump
    )]
    pub pool_rebase: Box<Account<'info, PoolRebase>>,

    /// Protocol state for validation
    #[account(
        seeds = [b"protocol"],
        bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,

    /// Pool authority and payer
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Required programs
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CleanupTickArray<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,

    /// The tick array to cleanup (will be closed)
    /// CHECK: Validated in handler
    #[account(
        mut,
        close = cleaner,
        constraint = tick_array.to_account_info().owner == __program_id @ FeelsProtocolError::InvalidAccountOwner
    )]
    pub tick_array: AccountLoader<'info, TickArray>,

    /// The cleaner who initiated the cleanup (receives 80% of rent)
    #[account(mut)]
    pub cleaner: Signer<'info>,

    /// Protocol treasury (receives 20% of rent)
    /// CHECK: Treasury address validated by pool
    #[account(
        mut,
        constraint = protocol_fee_recipient.key() == pool.load()?.authority @ FeelsProtocolError::InvalidAuthority
    )]
    pub protocol_fee_recipient: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Hook Management Contexts
// ============================================================================

#[derive(Accounts)]
pub struct RegisterHook<'info> {
    /// Pool to register hook for
    pub pool: AccountLoader<'info, Pool>,
    
    /// Hook registry
    #[account(
        mut,
        seeds = [b"hook_registry", pool.key().as_ref()],
        bump
    )]
    pub hook_registry: Account<'info, HookRegistry>,
    
    /// Hook program
    /// CHECK: Validated in handler
    pub hook_program: UncheckedAccount<'info>,
    
    /// Authority
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UnregisterHook<'info> {
    /// Pool to unregister hook from
    pub pool: AccountLoader<'info, Pool>,
    
    /// Hook registry
    #[account(
        mut,
        seeds = [b"hook_registry", pool.key().as_ref()],
        bump
    )]
    pub hook_registry: Account<'info, HookRegistry>,
    
    /// Authority
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct InitializeHookRegistry<'info> {
    /// Pool to initialize registry for
    pub pool: AccountLoader<'info, Pool>,
    
    /// Hook registry
    #[account(
        init,
        payer = authority,
        space = HookRegistry::SIZE,
        seeds = [b"hook_registry", pool.key().as_ref()],
        bump
    )]
    pub hook_registry: Account<'info, HookRegistry>,
    
    /// Authority
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ToggleHooks<'info> {
    /// Pool to toggle hooks for
    pub pool: AccountLoader<'info, Pool>,
    
    /// Hook registry
    #[account(
        mut,
        seeds = [b"hook_registry", pool.key().as_ref()],
        bump
    )]
    pub hook_registry: Account<'info, HookRegistry>,
    
    /// Authority
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct BatchCleanupTickArrays<'info> {
    /// Pool containing the tick arrays
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    /// Cleaner receiving rent
    #[account(mut)]
    pub cleaner: Signer<'info>,
    
    /// Protocol fee recipient (optional)
    /// CHECK: Validated in handler
    pub protocol_fee_recipient: Option<UncheckedAccount<'info>>,
}

/// Unified pool configuration context
#[derive(Accounts)]
pub struct ConfigurePool<'info> {
    /// Pool to configure
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    // Fee configuration now handled through GradientCache
    
    /// Hook registry (optional, for hook operations)
    #[account(
        mut,
        seeds = [b"hook_registry", pool.key().as_ref()],
        bump,
    )]
    pub hook_registry: Option<Account<'info, HookRegistry>>,
    
    /// Oracle account (optional, for oracle updates)
    /// CHECK: Validated in handler
    pub oracle: Option<UncheckedAccount<'info>>,
    
    /// Authority (must be pool authority)
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

// ============================================================================
// 3D Order System Contexts
// ============================================================================

/// Context for executing 3D orders (unified trading)
#[derive(Accounts)]
pub struct Order<'info> {
    /// Pool account
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    // Fee configuration now handled through GradientCache
    
    /// User executing the order
    #[account(mut)]
    pub user: Signer<'info>,
    
    /// User's token A account
    #[account(mut)]
    pub user_token_a: InterfaceAccount<'info, TokenAccount>,
    
    /// User's token B account
    #[account(mut)]
    pub user_token_b: InterfaceAccount<'info, TokenAccount>,
    
    /// Pool's token A vault
    #[account(mut)]
    pub pool_token_a: InterfaceAccount<'info, TokenAccount>,
    
    /// Pool's token B vault
    #[account(mut)]
    pub pool_token_b: InterfaceAccount<'info, TokenAccount>,
    
    /// Token program
    pub token_program: Program<'info, Token2022>,
    
    /// System program
    pub system_program: Program<'info, System>,
    
    // Optional accounts
    /// Tick array router for efficient tick access
    #[account(
        seeds = [b"router", pool.key().as_ref()],
        bump,
    )]
    pub tick_array_router: Option<Account<'info, TickArrayRouter>>,
    
    /// Hook registry
    #[account(
        seeds = [b"hook_registry", pool.key().as_ref()],
        bump,
    )]
    pub hook_registry: Option<Account<'info, HookRegistry>>,
    
    /// Hook message queue
    #[account(
        mut,
        seeds = [b"hook_messages", pool.key().as_ref()],
        bump,
    )]
    pub hook_message_queue: Option<Account<'info, HookMessageQueue>>,
}

/// Context for computing 3D order tick arrays
#[derive(Accounts)]
pub struct OrderCompute<'info> {
    /// Pool to analyze
    pub pool: AccountLoader<'info, Pool>,
    
    /// Optional: TickArrayRouter to populate with computed arrays
    #[account(
        mut,
        seeds = [b"router", pool.key().as_ref()],
        bump,
    )]
    pub tick_array_router: Option<Account<'info, TickArrayRouter>>,
    
    /// Authority for router updates (required if router provided)
    pub authority: Option<Signer<'info>>,
}

/// Context for modifying 3D orders
#[derive(Accounts)]
pub struct OrderModify<'info> {
    /// Pool account
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    /// Position/Order owner
    pub owner: Signer<'info>,
    
    /// Position metadata (for liquidity orders)
    #[account(
        mut,
        constraint = position.pool == pool.key() @ FeelsProtocolError::InvalidPool,
        constraint = position.owner == owner.key() @ FeelsProtocolError::InvalidOwner,
    )]
    pub position: Account<'info, TickPositionMetadata>,
    
    /// Optional: User token accounts for adjustments
    pub user_token_a: Option<InterfaceAccount<'info, TokenAccount>>,
    pub user_token_b: Option<InterfaceAccount<'info, TokenAccount>>,
    
    /// Pool vaults
    #[account(mut)]
    pub pool_token_a: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_b: InterfaceAccount<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token2022>,
}

/// Context for executing redenomination
#[derive(Accounts)]
pub struct Redenominate<'info> {
    /// Pool being redenominated
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    /// Protocol state for authority validation
    #[account(
        seeds = [b"protocol"],
        bump,
    )]
    pub protocol: Account<'info, ProtocolState>,
    
    /// Authority (must be protocol or pool redenomination authority)
    pub authority: Signer<'info>,
    
    // Optional hook accounts
    /// Hook registry
    #[account(
        seeds = [b"hook_registry", pool.key().as_ref()],
        bump,
    )]
    pub hook_registry: Option<Account<'info, HookRegistry>>,
    
    /// Hook message queue
    #[account(
        mut,
        seeds = [b"hook_messages", pool.key().as_ref()],
        bump,
    )]
    pub hook_message_queue: Option<Account<'info, HookMessageQueue>>,
}


#[derive(Accounts)]
pub struct CleanupEmptyTickArray<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,

    #[account(
        mut,
        close = beneficiary
    )]
    pub tick_array: AccountLoader<'info, TickArray>,

    /// Anyone can be the beneficiary (incentivizes cleanup)
    #[account(mut)]
    pub beneficiary: Signer<'info>,
}

#[derive(Accounts)]
pub struct CleanupTickArrayV2<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,

    /// The tick array to cleanup (will be closed)
    /// CHECK: Validated in handler
    #[account(
        mut,
        close = cleaner,
        constraint = tick_array.to_account_info().owner == __program_id @ FeelsProtocolError::InvalidAccountOwner
    )]
    pub tick_array: AccountLoader<'info, TickArray>,

    /// The cleaner who initiated the cleanup
    #[account(mut)]
    pub cleaner: Signer<'info>,

    /// Protocol treasury (required for incentivized mode)
    /// CHECK: Only needed for incentivized cleanup
    pub protocol_fee_recipient: Option<UncheckedAccount<'info>>,

    pub system_program: Program<'info, System>,
}

// ========================================================================
// Phase 2 Account Structs
// ========================================================================



// ExecuteRedenomination removed - use redenominate instruction with unified order system




/// Feels Protocol - Unified 3D Order System
/// 
/// All trading and liquidity operations are unified under a single 3D order model
/// that combines rate, duration, and leverage dimensions. This ensures consistent
/// execution paths for fees, hooks, and risk management.
/// 
/// # Core Trading Instructions
/// 
/// - `order`: Universal entry point for ALL trading operations
///   - OrderType::Immediate for swaps (spot and leveraged)
///   - OrderType::Liquidity for adding/removing liquidity  
///   - OrderType::Limit for limit orders
/// 
/// - `order_compute`: Pre-compute required accounts for complex orders
/// - `order_modify`: Modify existing orders (leverage, duration, etc.)
/// - `redenominate`: Handle leveraged position redenomination
/// 
/// # Why Unified?
/// 
/// The unified order system ensures:
/// 1. Consistent fee calculation across all operations
/// 2. Proper hook execution for all trade types
/// 3. Unified risk management and leverage handling
/// 4. Simplified client integration
/// 5. Better composability for complex operations
/// 
#[program]
pub mod feels {
    use super::*;

    pub fn initialize_feels(ctx: Context<InitializeFeels>) -> Result<()> {
        instructions::pool::initialize_protocol(ctx)
    }

    pub fn initialize_feelssol(
        ctx: Context<InitializeFeelsSOL>,
    ) -> Result<()> {
        let underlying_mint = ctx.accounts.underlying_mint.key();
        instructions::pool::initialize_feelssol(ctx, underlying_mint)
    }

    pub fn create_token(
        ctx: Context<CreateToken>,
        ticker: String,
        name: String,
        symbol: String,
        decimals: u8,
        initial_supply: u64,
    ) -> Result<()> {
        instructions::token::create_token(ctx, ticker, name, symbol, decimals, initial_supply)
    }

    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        fee_rate: u16,
        initial_sqrt_rate: u128,
        base_rate: u16,
        protocol_share: u16,
    ) -> Result<()> {
        instructions::pool::initialize_pool(ctx, fee_rate, initial_sqrt_rate, base_rate, protocol_share)
    }

    // ========================================================================
    // UNIFIED TRADING OPERATIONS
    // ========================================================================
    // ALL trading operations (swaps, liquidity, limits) go through the
    // unified 3D order system. All trading operations are now consolidated
    // removed to ensure consistent execution paths.
    //
    // Examples:
    // - Swap: order(...) with OrderType::Immediate, Duration::Swap
    // - Add Liquidity: order(...) with OrderType::Liquidity, Duration::Medium
    // - Leveraged Swap: order(...) with leverage > 1x
    // - Limit Order: order(...) with OrderType::Limit
    // ========================================================================

    // Fee collection is now handled through the unified order system
    // Use order(...) with appropriate parameters for fee collection

    // Protocol fee collection is now handled through the unified order system

    /// Clean up empty tick arrays
    /// 
    /// **DEPRECATED** - Use `cleanup_tick_array_v2` or `cleanup_empty_tick_array` instead.
    /// This instruction will be removed in a future release.
    #[deprecated(since = "1.1.0", note = "Use cleanup_tick_array_v2 instead")]
    pub fn cleanup_tick_array(ctx: Context<CleanupTickArray>) -> Result<()> {
        let params = instructions::cleanup::CleanupTickArrayParams {
            incentivized: true, // Default to incentivized mode
        };
        instructions::cleanup::cleanup_tick_array(ctx, params)
    }

    // All trading operations use the unified order system:
    // - For swaps: order(...) with OrderType::Immediate
    // - For liquidity: order(...) with OrderType::Liquidity
    // - For limit orders: order(...) with OrderType::Limit
    
    // ========================================================================
    // 3D Order System Instructions
    // ========================================================================
    
    /// Execute a 3D order (unified trading)
    /// 
    /// **INTERNAL USE ONLY** - External developers should use `order_unified` instead.
    /// This instruction uses the complex internal parameter structure and will be
    /// made private in a future release.
    #[deprecated(since = "1.1.0", note = "Use order_unified instead")]
    pub fn order<'info>(
        ctx: Context<'_, '_, 'info, 'info, Order<'info>>,
        params: instructions::order::OrderParams,
    ) -> Result<instructions::order::OrderResult> {
        instructions::order::handler(ctx, params)
    }
    
    /// Execute order with simplified unified parameters (RECOMMENDED)
    /// 
    /// This is the primary interface for all order operations including:
    /// - Swaps
    /// - Adding/removing liquidity
    /// - Limit orders
    /// - Flash loans
    pub fn order_unified<'info>(
        ctx: Context<'_, '_, 'info, 'info, Order<'info>>,
        params: instructions::unified_order::UnifiedOrderParams,
    ) -> Result<instructions::order::OrderResult> {
        // Convert unified params to internal format
        let internal_params = params.to_internal_params();
        instructions::order::handler(ctx, internal_params)
    }
    
    /// Compute tick arrays for 3D order
    /// 
    /// **INTERNAL USE ONLY** - External developers should use `order_compute_unified` instead.
    /// This instruction uses the complex internal parameter structure and will be
    /// made private in a future release.
    #[deprecated(since = "1.1.0", note = "Use order_compute_unified instead")]
    pub fn order_compute(
        ctx: Context<OrderCompute>,
        params: instructions::order_compute::OrderComputeParams,
    ) -> Result<instructions::order_compute::Tick3DArrayInfo> {
        instructions::order_compute::handler(ctx, params)
    }
    
    /// Compute order routing with simplified unified parameters (RECOMMENDED)
    /// 
    /// Pre-computes the optimal route and tick arrays needed for order execution.
    pub fn order_compute_unified(
        ctx: Context<OrderCompute>,
        params: instructions::unified_order::UnifiedComputeParams,
    ) -> Result<instructions::order_compute::Tick3DArrayInfo> {
        // Use the conversion method for consistency
        let internal_params = params.to_internal_compute_params();
        instructions::order_compute::handler(ctx, internal_params)
    }
    
    /// Modify an existing 3D order
    /// 
    /// **INTERNAL USE ONLY** - External developers should use `order_modify_unified` instead.
    /// This instruction uses the complex internal parameter structure and will be
    /// made private in a future release.
    #[deprecated(since = "1.1.0", note = "Use order_modify_unified instead")]
    pub fn order_modify<'info>(
        ctx: Context<'_, '_, 'info, 'info, OrderModify<'info>>,
        params: instructions::order_modify::OrderModifyParams,
    ) -> Result<()> {
        instructions::order_modify::handler(ctx, params)
    }
    
    /// Modify order with simplified unified parameters (RECOMMENDED)
    /// 
    /// Allows modification of existing orders and positions including:
    /// - Cancellation
    /// - Amount updates
    /// - Leverage adjustments
    /// - Duration changes
    pub fn order_modify_unified<'info>(
        ctx: Context<'_, '_, 'info, 'info, OrderModify<'info>>,
        params: instructions::unified_order::UnifiedModifyParams,
    ) -> Result<()> {
        // Convert unified params to internal format
        let internal_params = params.to_internal_params();
        instructions::order_modify::handler(ctx, internal_params)
    }
    
    /// Execute redenomination for leveraged orders
    pub fn redenominate<'info>(
        ctx: Context<'_, '_, 'info, 'info, Redenominate<'info>>,
        params: instructions::order_redenominate::RedenominateParams,
    ) -> Result<instructions::order_redenominate::RedenominationResult> {
        instructions::order_redenominate::handler(ctx, params)
    }

    /// Clean up completely empty tick arrays (RECOMMENDED)
    /// 
    /// More efficient version that only works on tick arrays with no initialized ticks.
    /// Preferred over cleanup_tick_array for better gas efficiency.
    pub fn cleanup_empty_tick_array(ctx: Context<CleanupEmptyTickArray>) -> Result<()> {
        // Load tick array to validate it's empty
        let tick_array = ctx.accounts.tick_array.load()?;
        let mut pool = ctx.accounts.pool.load_mut()?;
        
        // Validate tick array belongs to pool and is empty
        require!(
            tick_array.pool == ctx.accounts.pool.key(),
            FeelsProtocolError::InvalidPool
        );
        require!(
            tick_array.initialized_tick_count == 0,
            FeelsProtocolError::TickArrayNotEmpty
        );
        
        // Update bitmap to mark array as uninitialized
        TickManager::update_tick_array_bitmap(
            &mut *pool,
            tick_array.start_tick_index,
            false
        )?;
        
        // The close constraint in the account definition handles rent reclamation
        // The beneficiary receives 100% of the rent automatically
        
        Ok(())
    }

    /// Clean up tick arrays with enhanced validation (RECOMMENDED)
    /// 
    /// Advanced version with additional safety checks and configurable rent distribution.
    /// This is the preferred method for cleaning up tick arrays in production.
    pub fn cleanup_tick_array_v2(
        ctx: Context<CleanupTickArrayV2>,
        params: instructions::cleanup::CleanupTickArrayParams,
    ) -> Result<()> {
        instructions::cleanup::cleanup_tick_array_v2(ctx, params)
    }

    // ========================================================================
    // Unified Configuration Instruction
    // ========================================================================
    
    /// Configure pool parameters using unified configuration system
    /// This replaces multiple individual configuration instructions
    pub fn configure_pool(
        ctx: Context<ConfigurePool>,
        params: instructions::configure_pool::PoolConfigParams,
    ) -> Result<()> {
        instructions::configure_pool::handler(ctx, params)
    }
}
