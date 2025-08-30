
/// Feels Protocol - Concentrated Liquidity AMM
/// A next-generation automated market maker implementing concentrated liquidity positions
/// with advanced features like hooks, Token-2022 support, and oracle rate feeds.
/// Built on Solana for high-performance decentralized trading.
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint, TokenAccount};

pub mod constant;
pub mod instructions;
pub mod logic;
pub mod state;
pub mod utils;

// Import logic modules
use logic::tick::TickManager;

// Import all state types explicitly
use state::{
    // Core types
    Pool,
    FeelsProtocolError,
    ProtocolState,
    TickArray,
    TokenMetadata,
    FeelsSOL,
    // Hook types
    HookRegistry,
    HookMessageQueue,
    HookType,
    HookPermission,
    // Fee types
    FeeConfig,
    // Position types
    TickPositionMetadata,
    PositionVault,
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

    /// Fee configuration account for this pool
    #[account(
        init,
        payer = authority,
        space = FeeConfig::SIZE,
        seeds = [
            b"fee_config",
            pool.key().as_ref()
        ],
        bump
    )]
    pub fee_config: Account<'info, FeeConfig>,

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

    /// Position vault for automated management
    #[account(
        init,
        payer = authority,
        space = PositionVault::SIZE,
        seeds = [b"position_vault", pool.key().as_ref()],
        bump
    )]
    pub position_vault: Box<Account<'info, PositionVault>>,

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

// Type alias for backwards compatibility
pub type AddLiquidity<'info> = OpenPosition<'info>;

#[derive(Accounts)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,

    /// Tick position NFT metadata
    #[account(
        mut,
        constraint = tick_position_metadata.pool == pool.key() @ FeelsProtocolError::InvalidPool,
        constraint = tick_position_metadata.owner == user.key() @ FeelsProtocolError::Unauthorized,
    )]
    pub tick_position_metadata: Account<'info, TickPositionMetadata>,

    /// Tick array containing lower tick
    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_lower.load()?.start_tick_index.to_le_bytes()
        ],
        bump,
        constraint = tick_array_lower.load()?.pool == pool.key() @ FeelsProtocolError::InvalidTickArray
    )]
    pub tick_array_lower: AccountLoader<'info, TickArray>,

    /// Tick array containing upper tick
    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_upper.load()?.start_tick_index.to_le_bytes()
        ],
        bump,
        constraint = tick_array_upper.load()?.pool == pool.key() @ FeelsProtocolError::InvalidTickArray
    )]
    pub tick_array_upper: AccountLoader<'info, TickArray>,

    /// User account
    #[account(mut)]
    pub user: Signer<'info>,

    /// Payer for tick array creation (can be same as user)
    #[account(mut)]
    pub payer: Signer<'info>,

    /// User's token a account
    #[account(mut)]
    pub user_token_a: InterfaceAccount<'info, TokenAccount>,

    /// User's token b account
    #[account(mut)]
    pub user_token_b: InterfaceAccount<'info, TokenAccount>,

    /// Pool's token a vault
    #[account(mut)]
    pub pool_token_a: InterfaceAccount<'info, TokenAccount>,

    /// Pool's token b vault
    #[account(mut)]
    pub pool_token_b: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    
    // Optional hook accounts
    /// Hook registry for this pool
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
pub struct CollectFees<'info> {
    /// The pool from which to collect fees
    #[account(
        seeds = [
            b"pool",
            pool.load()?.token_a_mint.as_ref(),
            pool.load()?.token_b_mint.as_ref(),
            &pool.load()?.fee_rate.to_le_bytes()
        ],
        bump,
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// The tick position for which to collect fees
    #[account(
        mut,
        seeds = [
            b"position",
            position.pool.as_ref(),
            position.tick_position_mint.as_ref()
        ],
        bump,
        constraint = position.pool == pool.key() @ FeelsProtocolError::InvalidPool,
        constraint = position.owner == owner.key() @ FeelsProtocolError::InvalidOwner,
    )]
    pub position: Account<'info, TickPositionMetadata>,

    /// Token vault for token a
    #[account(
        mut,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            pool.load()?.token_a_mint.as_ref()
        ],
        bump,
        token::mint = pool.load()?.token_a_mint,
        token::authority = pool,
        token::token_program = token_program,
    )]
    pub token_vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token vault for token b
    #[account(
        mut,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            pool.load()?.token_b_mint.as_ref()
        ],
        bump,
        token::mint = pool.load()?.token_b_mint,
        token::authority = pool,
        token::token_program = token_program,
    )]
    pub token_vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// User's token account for token a
    #[account(
        mut,
        token::mint = pool.load()?.token_a_mint,
        token::authority = owner,
        token::token_program = token_program,
    )]
    pub token_account_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// User's token account for token b
    #[account(
        mut,
        token::mint = pool.load()?.token_b_mint,
        token::authority = owner,
        token::token_program = token_program,
    )]
    pub token_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The tick position owner
    pub owner: Signer<'info>,

    /// Token program
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct CollectProtocolFees<'info> {
    /// The pool from which to collect protocol fees
    #[account(
        mut,
        seeds = [
            b"pool",
            pool.load()?.token_a_mint.as_ref(),
            pool.load()?.token_b_mint.as_ref(),
            &pool.load()?.fee_rate.to_le_bytes()
        ],
        bump
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// Token vault for token a
    #[account(
        mut,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            pool.load()?.token_a_mint.as_ref()
        ],
        bump,
        token::mint = pool.load()?.token_a_mint,
        token::authority = pool,
        token::token_program = token_program,
    )]
    pub token_vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token vault for token b
    #[account(
        mut,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            pool.load()?.token_b_mint.as_ref()
        ],
        bump,
        token::mint = pool.load()?.token_b_mint,
        token::authority = pool,
        token::token_program = token_program,
    )]
    pub token_vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Recipient token account for token a
    #[account(
        mut,
        token::mint = pool.load()?.token_a_mint,
        token::authority = authority,
        token::token_program = token_program,
    )]
    pub recipient_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Recipient token account for token b
    #[account(
        mut,
        token::mint = pool.load()?.token_b_mint,
        token::authority = authority,
        token::token_program = token_program,
    )]
    pub recipient_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Protocol authority
    pub authority: Signer<'info>,

    /// Token program
    pub token_program: Program<'info, Token2022>,
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

// Legacy ExecuteOrder and ExecuteRoutedSwap contexts removed
// Use the unified Order context with appropriate OrderType

// Type alias for backwards compatibility
pub type RemoveLiquidity<'info> = ClosePosition<'info>;

#[derive(Accounts)]
pub struct ClosePosition<'info> {
    /// The pool from which to remove liquidity
    #[account(
        mut,
        seeds = [
            b"pool",
            pool.load()?.token_a_mint.as_ref(),
            pool.load()?.token_b_mint.as_ref(),
            &pool.load()?.fee_rate.to_le_bytes()
        ],
        bump,
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// The position NFT metadata
    #[account(
        mut,
        seeds = [
            b"position",
            position.pool.as_ref(),
            position.tick_position_mint.as_ref()
        ],
        bump,
        constraint = position.pool == pool.key() @ FeelsProtocolError::InvalidPool,
        constraint = position.owner == owner.key() @ FeelsProtocolError::InvalidOwner,
    )]
    pub position: Account<'info, TickPositionMetadata>,

    /// The tick array for the lower tick
    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_lower.load()?.start_tick_index.to_le_bytes()
        ],
        bump,
    )]
    pub tick_array_lower: AccountLoader<'info, TickArray>,

    /// The tick array for the upper tick
    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_upper.load()?.start_tick_index.to_le_bytes()
        ],
        bump,
    )]
    pub tick_array_upper: AccountLoader<'info, TickArray>,

    /// Token vault for token a
    #[account(
        mut,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            pool.load()?.token_a_mint.as_ref()
        ],
        bump,
        token::mint = pool.load()?.token_a_mint,
        token::authority = pool,
        token::token_program = token_program,
    )]
    pub token_vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token vault for token b (FeelsSOL)
    #[account(
        mut,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            pool.load()?.token_b_mint.as_ref()
        ],
        bump,
        token::mint = pool.load()?.token_b_mint,
        token::authority = pool,
        token::token_program = token_program,
    )]
    pub token_vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// User's token account for token a
    #[account(
        mut,
        token::mint = pool.load()?.token_a_mint,
        token::authority = owner,
        token::token_program = token_program,
    )]
    pub token_account_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// User's token account for token b (FeelsSOL)
    #[account(
        mut,
        token::mint = pool.load()?.token_b_mint,
        token::authority = owner,
        token::token_program = token_program,
    )]
    pub token_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The tick position NFT owner
    pub owner: Signer<'info>,

    /// Token program (Token-2022)
    pub token_program: Program<'info, Token2022>,
    
    // Optional hook accounts
    /// Hook registry for this pool
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

// Legacy GetOrderTickArrays removed - use OrderCompute context

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

// ============================================================================
// 3D Order System Contexts
// ============================================================================

/// Context for executing 3D orders (unified trading)
#[derive(Accounts)]
pub struct Order<'info> {
    /// Pool account
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    /// Fee configuration account
    #[account(
        seeds = [b"fee_config", pool.key().as_ref()],
        bump
    )]
    pub fee_config: Account<'info, FeeConfig>,
    
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

#[derive(Accounts)]
pub struct EnableLeverage<'info> {
    #[account(
        constraint = protocol.authority == authority.key() @ FeelsProtocolError::Unauthorized
    )]
    pub protocol: Account<'info, ProtocolState>,

    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateDynamicFees<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,

    #[account(
        constraint = authority.key() == pool.load()?.authority @ FeelsProtocolError::Unauthorized
    )]
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"fee_config", pool.key().as_ref()],
        bump
    )]
    pub fee_config: Account<'info, FeeConfig>,
}

#[derive(Accounts)]
pub struct RegisterValenceHook<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,

    /// Valence session account from the Valence kernel
    /// CHECK: Validated by CPI to Valence kernel
    pub valence_session: UncheckedAccount<'info>,

    /// Hook registry for this pool (created if needed)
    #[account(
        init_if_needed,
        payer = payer,
        space = HookRegistry::SIZE,
        seeds = [b"hook_registry", pool.key().as_ref()],
        bump
    )]
    pub hook_registry: Box<Account<'info, HookRegistry>>,

    #[account(
        constraint = authority.key() == pool.load()?.authority @ FeelsProtocolError::Unauthorized
    )]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// Valence kernel program
    /// CHECK: Program ID validation happens in handler
    pub valence_kernel: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteRedenomination<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,

    /// Protocol account for authorization
    #[account(
        seeds = [b"protocol"],
        bump,
        constraint = protocol.authority == authority.key() @ FeelsProtocolError::Unauthorized
    )]
    pub protocol: Account<'info, ProtocolState>,

    /// Unified oracle for rate verification
    #[account(
        constraint = oracle.pool == pool.key() @ FeelsProtocolError::InvalidPool
    )]
    pub oracle: Box<Account<'info, Oracle>>,

    /// Oracle data account
    #[account(
        constraint = oracle_data.key() == oracle.data_account @ FeelsProtocolError::InvalidOracle
    )]
    pub oracle_data: AccountLoader<'info, OracleData>,

    /// Protocol authority (required for redenomination)
    pub authority: Signer<'info>,

    pub clock: Sysvar<'info, Clock>,
}


#[derive(Accounts)]
pub struct UpdateLeverageCeiling<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,

    #[account(
        constraint = authority.key() == pool.load()?.authority @ FeelsProtocolError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct RedenominateLeveragedPosition<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,

    /// Tick position NFT metadata
    #[account(
        mut,
        constraint = tick_position_metadata.pool == pool.key() @ FeelsProtocolError::InvalidPool,
        constraint = tick_position_metadata.owner == user.key() @ FeelsProtocolError::Unauthorized,
    )]
    pub tick_position_metadata: Account<'info, TickPositionMetadata>,

    pub user: Signer<'info>,
}

// ========================================================================
// Phase 2 Instruction Parameters
// ========================================================================

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct EnableLeverageParams {
    /// Maximum leverage allowed (6 decimals, e.g., 10_000_000 = 10x)
    pub max_leverage: u64,
    /// Initial leverage ceiling (usually lower than max)
    pub initial_ceiling: u64,
    /// Protection curve type (0 = Linear, 1 = Exponential, 2 = Piecewise)
    pub protection_curve_type: u8,
    /// Protection curve data (decay_rate for exponential, unused for linear)
    pub protection_curve_decay_rate: u64,
    /// Protection curve points for piecewise (8 points of [leverage, protection])
    pub protection_curve_points: [[u64; 2]; 8],
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateDynamicFeesParams {
    /// Base fee rate in basis points
    pub base_fee: u16,
    /// Minimum allowed fee
    pub min_fee: u16,
    /// Maximum allowed fee
    pub max_fee: u16,
    /// Coefficient for volatility adjustment (6 decimals)
    pub volatility_coefficient: u64,
    /// Volume threshold for discounts
    pub volume_discount_threshold: u128,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RegisterHookParams {
    /// Hook type to register
    pub hook_type: HookType,
    /// Hook permission level
    pub permission: HookPermission,
    /// Events this hook is interested in
    pub event_mask: u32,
    /// Stages this hook runs in (validation and/or execution)
    pub stage_mask: u32,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RegisterValenceHookParams {
    /// Hook type to register
    pub hook_type: HookType,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RedenominationParams {
    /// Target rate after redenomination (Q96)
    pub target_sqrt_rate: u128,
    /// Redenomination factor (6 decimals, e.g., 900_000 = 0.9x)
    pub redenomination_factor: u64,
}

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
    // unified 3D order system. Legacy swap/liquidity instructions have been
    // removed to ensure consistent execution paths.
    //
    // Examples:
    // - Swap: order(...) with OrderType::Immediate, Duration::Swap
    // - Add Liquidity: order(...) with OrderType::Liquidity, Duration::Medium
    // - Leveraged Swap: order(...) with leverage > 1x
    // - Limit Order: order(...) with OrderType::Limit
    // ========================================================================

    pub fn collect_fees(
        ctx: Context<CollectFees>,
        amount_a_requested: u64,
        amount_b_requested: u64,
    ) -> Result<(u64, u64)> {
        instructions::fee::collect_pool_fees(ctx, amount_a_requested, amount_b_requested)
    }

    pub fn collect_protocol_fees(
        ctx: Context<CollectProtocolFees>,
        amount_a_requested: u64,
        amount_b_requested: u64,
    ) -> Result<(u64, u64)> {
        instructions::fee::collect_protocol_fees(ctx, amount_a_requested, amount_b_requested)
    }

    pub fn cleanup_tick_array(ctx: Context<CleanupTickArray>) -> Result<()> {
        let params = instructions::cleanup::CleanupTickArrayParams {
            incentivized: true, // Default to incentivized mode
        };
        instructions::cleanup::cleanup_tick_array(ctx, params)
    }

    // Legacy swap instructions removed - use the unified order system:
    // - For swaps: order(...) with OrderType::Immediate
    // - For liquidity: order(...) with OrderType::Liquidity
    // - For limit orders: order(...) with OrderType::Limit

    // Use order_compute instead for getting tick arrays
    
    // ========================================================================
    // 3D Order System Instructions
    // ========================================================================
    
    /// Execute a 3D order (unified trading)
    pub fn order<'info>(
        ctx: Context<'_, '_, 'info, 'info, Order<'info>>,
        params: instructions::order::OrderParams,
    ) -> Result<instructions::order::OrderResult> {
        instructions::order::handler(ctx, params)
    }
    
    /// Compute tick arrays for 3D order
    pub fn order_compute(
        ctx: Context<OrderCompute>,
        params: instructions::order_compute::OrderComputeParams,
    ) -> Result<instructions::order_compute::Tick3DArrayInfo> {
        instructions::order_compute::handler(ctx, params)
    }
    
    /// Modify an existing 3D order
    pub fn order_modify<'info>(
        ctx: Context<'_, '_, 'info, 'info, OrderModify<'info>>,
        params: instructions::order_modify::OrderModifyParams,
    ) -> Result<()> {
        instructions::order_modify::handler(ctx, params)
    }
    
    /// Execute redenomination for leveraged orders
    pub fn redenominate<'info>(
        ctx: Context<'_, '_, 'info, 'info, Redenominate<'info>>,
        params: instructions::order_redenominate::RedenominateParams,
    ) -> Result<instructions::order_redenominate::RedenominationResult> {
        instructions::order_redenominate::handler(ctx, params)
    }

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

    pub fn cleanup_tick_array_v2(
        ctx: Context<CleanupTickArrayV2>,
        params: instructions::cleanup::CleanupTickArrayParams,
    ) -> Result<()> {
        instructions::cleanup::cleanup_tick_array_v2(ctx, params)
    }

    // ========================================================================
    // Phase 2 Instructions
    // ========================================================================

    // Leverage management is now handled through 3D order system
    // Use order_modify_3d for leverage adjustments

    pub fn update_dynamic_fees(
        ctx: Context<UpdateDynamicFees>,
        params: UpdateDynamicFeesParams,
    ) -> Result<()> {
        instructions::fee::update_dynamic_fees(ctx, params)
    }

    pub fn register_valence_hook(
        ctx: Context<RegisterValenceHook>,
        params: RegisterValenceHookParams,
    ) -> Result<()> {
        // Initialize hook registry if needed
        let registry = &mut ctx.accounts.hook_registry;
        let clock = Clock::get()?;
        
        // If this is a new registry, initialize it
        if registry.pool == Pubkey::default() {
            registry.pool = ctx.accounts.pool.key();
            registry.authority = ctx.accounts.authority.key();
            registry.hook_count = 0;
            registry.hooks_enabled = true;
            registry.message_queue_enabled = false;
            registry.emergency_authority = Some(ctx.accounts.authority.key());
            registry.last_update_timestamp = clock.unix_timestamp;
        }
        
        // Validate authority
        require!(
            ctx.accounts.authority.key() == registry.authority,
            FeelsProtocolError::InvalidAuthority
        );
        
        // Register the Valence hook with appropriate permissions
        // Valence hooks typically need Modify permission to update state
        let permission = HookPermission::Modify;
        
        // Set event mask based on hook type
        let event_mask = match params.hook_type {
            HookType::BeforeSwap => 0b00000001,
            HookType::AfterSwap => 0b00000010,
            HookType::BeforeAdd => 0b00000100,
            HookType::AfterAdd => 0b00001000,
            HookType::BeforeRemove => 0b00010000,
            HookType::AfterRemove => 0b00100000,
            HookType::PriceFeed => 0b01000000,
            HookType::Liquidity => 0b10000000,
            HookType::Arbitrage => 0b10000001,
            HookType::Validation => 0b11111111, // All events for validation hooks
        };
        
        // Stage mask: both validation (1) and execution (2) stages
        let stage_mask = 0b11;
        
        // Register the hook
        let index = registry.register_hook(
            ctx.accounts.valence_session.key(),
            event_mask,
            stage_mask,
            permission,
        )?;
        
        // Update pool to store valence session reference
        let mut pool = ctx.accounts.pool.load_mut()?;
        pool.valence_session = ctx.accounts.valence_session.key();
        
        registry.last_update_timestamp = clock.unix_timestamp;
        
        emit!(logic::event::HookRegisteredEvent {
            pool: registry.pool,
            hook_program: ctx.accounts.valence_session.key(),
            event_mask,
            stage_mask,
            permission: permission as u8,
            index: index as u8,
            timestamp: clock.unix_timestamp,
        });
        
        Ok(())
    }

    // Redenomination is handled through the unified redenominate instruction
    
    // Flash loans are now handled through the 3D order system with Duration::Flash
}

// ============================================================================
// Vault Context Types
// ============================================================================

/// Vault deposit context
#[derive(Accounts)]
pub struct VaultDeposit<'info> {
    #[account(mut)]
    pub vault: Account<'info, PositionVault>,
    
    #[account(mut)]
    pub user: Signer<'info>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,
}

/// Vault withdraw context
#[derive(Accounts)]
pub struct VaultWithdraw<'info> {
    #[account(mut)]
    pub vault: Account<'info, PositionVault>,
    
    #[account(mut)]
    pub user: Signer<'info>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,
}

