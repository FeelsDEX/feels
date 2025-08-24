#![allow(deprecated)]

/// Feels Protocol - Concentrated Liquidity AMM
/// A next-generation automated market maker implementing concentrated liquidity positions
/// with advanced features like hooks, Token-2022 support, and oracle price feeds.
/// Built on Solana for high-performance decentralized trading.

use anchor_lang::prelude::*;
use anchor_lang::accounts::interface_account::InterfaceAccount;
use anchor_spl::token_interface::{Mint, TokenAccount};
use anchor_spl::token_2022::Token2022;
use anchor_spl::associated_token::AssociatedToken;

pub mod instructions;
pub mod state;
pub mod utils;
pub mod logic;
pub mod constant;

// Import instruction contexts

declare_id!("Fee1sProtoco11111111111111111111111111111111");

#[derive(Accounts)]
pub struct InitializeFeels<'info> {
    /// Protocol state account
    #[account(
        init,
        payer = authority,
        space = state::ProtocolState::SIZE,
        seeds = [b"protocol"],
        bump
    )]
    pub protocol_state: Account<'info, state::ProtocolState>,
    
    /// Protocol authority and payer
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// Protocol treasury account
    /// CHECK: Can be any account that will receive fees
    pub treasury: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(underlying_mint: Pubkey)]
pub struct InitializeFeelsSOL<'info> {
    /// FeelsSOL wrapper account
    #[account(
        init,
        payer = authority,
        space = state::FeelsSOL::SIZE,
        seeds = [b"feelssol"],
        bump
    )]
    pub feelssol: Account<'info, state::FeelsSOL>,
    
    /// FeelsSOL Token-2022 mint
    #[account(
        init,
        payer = authority,
        mint::decimals = 9,
        mint::authority = feelssol,
        mint::freeze_authority = feelssol,
    )]
    pub feels_mint: InterfaceAccount<'info, Mint>,
    
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
        space = state::TokenMetadata::SIZE,
        seeds = [
            b"token_metadata",
            token_mint.key().as_ref()
        ],
        bump
    )]
    pub token_metadata: Account<'info, state::TokenMetadata>,
    
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
#[instruction(fee_rate: u16, initial_sqrt_price: u128)]
pub struct InitializePool<'info> {
    /// The pool account to initialize
    /// Seeds are validated to ensure canonical token ordering
    #[account(
        init,
        payer = authority,
        space = state::Pool::SIZE,
        seeds = [
            b"pool",
            token_a_mint.key().as_ref(),
            token_b_mint.key().as_ref(),
            fee_rate.to_le_bytes().as_ref()
        ],
        bump
    )]
    pub pool: AccountLoader<'info, state::Pool>,
    
    /// Token A mint (order doesn't matter - will be canonicalized)
    pub token_a_mint: InterfaceAccount<'info, Mint>,
    
    /// Token B mint (order doesn't matter - will be canonicalized)
    pub token_b_mint: InterfaceAccount<'info, Mint>,
    
    /// FeelsSOL wrapper account for validation
    #[account(
        seeds = [b"feelssol"],
        bump,
        constraint = feelssol.feels_mint == token_a_mint.key() || feelssol.feels_mint == token_b_mint.key() @ state::PoolError::NotFeelsSOLPair
    )]
    pub feelssol: Account<'info, state::FeelsSOL>,
    
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
    
    /// Protocol state for validation
    #[account(
        seeds = [b"protocol"],
        bump
    )]
    pub protocol_state: Account<'info, state::ProtocolState>,
    
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
pub struct AddLiquidity<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, state::Pool>,
    
    /// Tick position NFT metadata
    #[account(
        mut,
        constraint = tick_position_metadata.pool == pool.key() @ state::PoolError::InvalidPool,
        constraint = tick_position_metadata.owner == user.key() @ state::PoolError::Unauthorized,
    )]
    pub tick_position_metadata: Account<'info, state::TickPositionMetadata>,
    
    /// Tick array containing lower tick
    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_lower.load()?.start_tick_index.to_le_bytes()
        ],
        bump,
        constraint = tick_array_lower.load()?.pool == pool.key() @ state::PoolError::InvalidTickArray
    )]
    pub tick_array_lower: AccountLoader<'info, state::TickArray>,
    
    /// Tick array containing upper tick
    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_upper.load()?.start_tick_index.to_le_bytes()
        ],
        bump,
        constraint = tick_array_upper.load()?.pool == pool.key() @ state::PoolError::InvalidTickArray
    )]
    pub tick_array_upper: AccountLoader<'info, state::TickArray>,
    
    /// User account
    #[account(mut)]
    pub user: Signer<'info>,
    
    /// Payer for tick array creation (can be same as user)
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// User's token 0 account
    #[account(mut)]
    pub user_token_0: InterfaceAccount<'info, TokenAccount>,
    
    /// User's token 1 account
    #[account(mut)]
    pub user_token_1: InterfaceAccount<'info, TokenAccount>,
    
    /// Pool's token 0 vault
    #[account(mut)]
    pub pool_token_0: InterfaceAccount<'info, TokenAccount>,
    
    /// Pool's token 1 vault
    #[account(mut)]
    pub pool_token_1: InterfaceAccount<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
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
    pub pool: AccountLoader<'info, state::Pool>,
    
    /// The tick position for which to collect fees
    #[account(
        mut,
        seeds = [
            b"position",
            position.pool.as_ref(),
            position.tick_position_mint.as_ref()
        ],
        bump,
        constraint = position.pool == pool.key() @ state::PoolError::InvalidPool,
        constraint = position.owner == owner.key() @ state::PoolError::InvalidOwner,
    )]
    pub position: Account<'info, state::TickPositionMetadata>,
    
    /// Token vault for token 0
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
    pub token_vault_0: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// Token vault for token 1
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
    pub token_vault_1: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// User's token account for token 0
    #[account(
        mut,
        token::mint = pool.load()?.token_a_mint,
        token::authority = owner,
        token::token_program = token_program,
    )]
    pub token_account_0: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// User's token account for token 1
    #[account(
        mut,
        token::mint = pool.load()?.token_b_mint,
        token::authority = owner,
        token::token_program = token_program,
    )]
    pub token_account_1: Box<InterfaceAccount<'info, TokenAccount>>,
    
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
    pub pool: AccountLoader<'info, state::Pool>,
    
    /// Token vault for token 0
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
    pub token_vault_0: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// Token vault for token 1
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
    pub token_vault_1: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// Recipient token account for token 0
    #[account(
        mut,
        token::mint = pool.load()?.token_a_mint,
        token::authority = authority,
        token::token_program = token_program,
    )]
    pub recipient_0: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// Recipient token account for token 1
    #[account(
        mut,
        token::mint = pool.load()?.token_b_mint,
        token::authority = authority,
        token::token_program = token_program,
    )]
    pub recipient_1: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// Protocol authority
    pub authority: Signer<'info>,
    
    /// Token program
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct CleanupTickArray<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, state::Pool>,
    
    /// The tick array to cleanup (will be closed)
    /// CHECK: Validated in handler
    #[account(
        mut,
        close = cleaner,
        constraint = tick_array.to_account_info().owner == __program_id @ state::PoolError::InvalidAccountOwner
    )]
    pub tick_array: AccountLoader<'info, state::TickArray>,
    
    /// The cleaner who initiated the cleanup (receives 80% of rent)
    #[account(mut)]
    pub cleaner: Signer<'info>,
    
    /// Protocol treasury (receives 20% of rent)
    /// CHECK: Treasury address validated by pool
    #[account(
        mut,
        constraint = protocol_fee_recipient.key() == pool.load()?.authority @ state::PoolError::InvalidAuthority
    )]
    pub protocol_fee_recipient: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    /// Pool account
    #[account(mut)]
    pub pool: AccountLoader<'info, state::Pool>,
    
    /// Oracle state for TWAP updates
    #[account(mut)]
    pub oracle_state: Account<'info, state::ObservationState>,
    
    /// User account
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
    
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct ExecuteRoutedSwap<'info> {
    /// First pool (Token A / FeelsSOL or Token A / Token B if direct)
    #[account(mut)]
    pub pool_1: AccountLoader<'info, state::Pool>,
    
    /// Second pool (FeelsSOL / Token B) - only used for two-hop swaps
    #[account(mut)]
    pub pool_2: AccountLoader<'info, state::Pool>,
    
    /// FeelsSOL wrapper for routing validation
    pub feelssol: Account<'info, state::FeelsSOL>,
    
    /// Input token mint
    pub token_in_mint: InterfaceAccount<'info, Mint>,
    
    /// Output token mint
    pub token_out_mint: InterfaceAccount<'info, Mint>,
    
    /// User executing the swap
    #[account(mut)]
    pub user: Signer<'info>,
    
    /// User's input token account
    #[account(mut)]
    pub user_token_in: InterfaceAccount<'info, TokenAccount>,
    
    /// User's output token account
    #[account(mut)]
    pub user_token_out: InterfaceAccount<'info, TokenAccount>,
    
    /// Pool 1's input token vault
    #[account(mut)]
    pub pool_1_token_in: InterfaceAccount<'info, TokenAccount>,
    
    /// Pool 1's output token vault
    #[account(mut)]
    pub pool_1_token_out: InterfaceAccount<'info, TokenAccount>,
    
    /// Pool 2's input token vault (for two-hop swaps)
    #[account(mut)]
    pub pool_2_token_in: InterfaceAccount<'info, TokenAccount>,
    
    /// Pool 2's output token vault (for two-hop swaps)
    #[account(mut)]
    pub pool_2_token_out: InterfaceAccount<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
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
    pub pool: AccountLoader<'info, state::Pool>,
    
    /// The position NFT metadata
    #[account(
        mut,
        seeds = [
            b"position",
            position.pool.as_ref(),
            position.tick_position_mint.as_ref()
        ],
        bump,
        constraint = position.pool == pool.key() @ state::PoolError::InvalidPool,
        constraint = position.owner == owner.key() @ state::PoolError::InvalidOwner,
    )]
    pub position: Account<'info, state::TickPositionMetadata>,
    
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
    pub tick_array_lower: AccountLoader<'info, state::TickArray>,
    
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
    pub tick_array_upper: AccountLoader<'info, state::TickArray>,
    
    /// Token vault for token 0
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
    pub token_vault_0: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// Token vault for token 1 (FeelsSOL)
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
    pub token_vault_1: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// User's token account for token 0
    #[account(
        mut,
        token::mint = pool.load()?.token_a_mint,
        token::authority = owner,
        token::token_program = token_program,
    )]
    pub token_account_0: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// User's token account for token 1 (FeelsSOL)
    #[account(
        mut,
        token::mint = pool.load()?.token_b_mint,
        token::authority = owner,
        token::token_program = token_program,
    )]
    pub token_account_1: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// The tick position NFT owner
    pub owner: Signer<'info>,
    
    /// Token program (Token-2022)
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct GetSwapTickArrays<'info> {
    /// Pool to analyze for tick arrays
    pub pool: AccountLoader<'info, state::Pool>,
}

#[derive(Accounts)]
pub struct InitializeTransientUpdates<'info> {
    /// Pool this batch belongs to
    #[account(
        seeds = [
            b"pool",
            pool.load()?.token_a_mint.as_ref(),
            pool.load()?.token_b_mint.as_ref(),
            &pool.load()?.fee_rate.to_le_bytes()
        ],
        bump,
    )]
    pub pool: AccountLoader<'info, state::Pool>,
    
    /// Transient updates account
    /// Note: Using slot as PDA seed means only one batch per pool per slot.
    /// This is intentional - prevents multiple conflicting batches in same slot.
    /// For higher throughput, use different slots or wait for next slot.
    #[account(
        init,
        payer = authority,
        space = state::TransientTickUpdates::SIZE,
        seeds = [
            b"transient_updates",
            pool.key().as_ref(),
            &Clock::get()?.slot.to_le_bytes(),
        ],
        bump
    )]
    pub transient_updates: AccountLoader<'info, state::TransientTickUpdates>,
    
    /// Authority (typically a keeper or the protocol)
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddTickUpdate<'info> {
    /// Pool this batch belongs to
    pub pool: AccountLoader<'info, state::Pool>,
    
    /// Transient updates account
    #[account(
        mut,
        seeds = [
            b"transient_updates",
            pool.key().as_ref(),
            &transient_updates.load()?.slot.to_le_bytes(),
        ],
        bump
    )]
    pub transient_updates: AccountLoader<'info, state::TransientTickUpdates>,
    
    /// Authority (keeper or protocol)
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct FinalizeTransientUpdates<'info> {
    /// Pool this batch belongs to
    #[account(mut)]
    pub pool: AccountLoader<'info, state::Pool>,
    
    /// Transient updates account
    #[account(
        mut,
        seeds = [
            b"transient_updates",
            pool.key().as_ref(),
            &transient_updates.load()?.slot.to_le_bytes(),
        ],
        bump
    )]
    pub transient_updates: AccountLoader<'info, state::TransientTickUpdates>,
    
    /// Authority (keeper or protocol)
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CleanupTransientUpdates<'info> {
    /// Transient updates account to close
    #[account(
        mut,
        close = authority,
        seeds = [
            b"transient_updates",
            transient_updates.load()?.pool.as_ref(),
            &transient_updates.load()?.slot.to_le_bytes(),
        ],
        bump
    )]
    pub transient_updates: AccountLoader<'info, state::TransientTickUpdates>,
    
    /// Authority to receive rent refund
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ResetTransientUpdates<'info> {
    /// Transient updates account to reset
    #[account(
        mut,
        seeds = [
            b"transient_updates",
            transient_updates.load()?.pool.as_ref(),
            &transient_updates.load()?.slot.to_le_bytes(),
        ],
        bump
    )]
    pub transient_updates: AccountLoader<'info, state::TransientTickUpdates>,
    
    /// Authority (keeper or protocol)
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CleanupEmptyTickArray<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, state::Pool>,
    
    #[account(
        mut,
        close = beneficiary
    )]
    pub tick_array: AccountLoader<'info, state::TickArray>,
    
    /// Anyone can be the beneficiary (incentivizes cleanup)
    #[account(mut)]
    pub beneficiary: Signer<'info>,
}

#[program]
pub mod feels {
    use super::*;
    
    pub fn initialize_feels(ctx: Context<InitializeFeels>) -> Result<()> {
        instructions::initialize_protocol::handler(ctx)
    }
    
    pub fn initialize_feelssol(ctx: Context<InitializeFeelsSOL>, underlying_mint: Pubkey) -> Result<()> {
        instructions::initialize_feelssol::handler(ctx, underlying_mint)
    }
    
    pub fn create_token(ctx: Context<CreateToken>, ticker: String, name: String, symbol: String, decimals: u8, initial_supply: u64) -> Result<()> {
        instructions::token_create::handler(ctx, ticker, name, symbol, decimals, initial_supply)
    }
    
    pub fn initialize_pool(ctx: Context<InitializePool>, fee_rate: u16, initial_sqrt_price: u128) -> Result<()> {
        instructions::initialize_pool::handler(ctx, fee_rate, initial_sqrt_price)
    }
    
    pub fn add_liquidity(ctx: Context<AddLiquidity>, liquidity_amount: u128, amount_0_max: u64, amount_1_max: u64) -> Result<(u64, u64)> {
        instructions::liquidity_add::handler(ctx, liquidity_amount, amount_0_max, amount_1_max)
    }
    
    pub fn remove_liquidity(ctx: Context<RemoveLiquidity>, liquidity_amount: u128, amount_0_min: u64, amount_1_min: u64) -> Result<(u64, u64)> {
        instructions::liquidity_remove::handler(ctx, liquidity_amount, amount_0_min, amount_1_min)
    }
    
    pub fn collect_fees(ctx: Context<CollectFees>, amount_0_requested: u64, amount_1_requested: u64) -> Result<(u64, u64)> {
        instructions::fee_collect_pool::handler(ctx, amount_0_requested, amount_1_requested)
    }
    
    pub fn collect_protocol_fees(ctx: Context<CollectProtocolFees>, amount_0_requested: u64, amount_1_requested: u64) -> Result<(u64, u64)> {
        instructions::fee_collect_protocol::handler(ctx, amount_0_requested, amount_1_requested)
    }
    
    pub fn cleanup_tick_array(ctx: Context<CleanupTickArray>) -> Result<()> {
        instructions::tick_cleanup::handler(ctx)
    }
    
    pub fn swap_execute<'info>(ctx: Context<'_, '_, 'info, 'info, Swap<'info>>, amount_in: u64, amount_out_minimum: u64, sqrt_price_limit: u128, is_token_0_to_1: bool) -> Result<u64> {
        instructions::swap_execute::handler(ctx, amount_in, amount_out_minimum, sqrt_price_limit, is_token_0_to_1)
    }
    
    pub fn execute_routed_swap<'info>(ctx: Context<'_, '_, 'info, 'info, ExecuteRoutedSwap<'info>>, amount_in: u64, amount_out_minimum: u64, sqrt_price_limit_1: u128, sqrt_price_limit_2: Option<u128>) -> Result<u64> {
        instructions::swap_execute::execute_routed_swap_handler(ctx, amount_in, amount_out_minimum, sqrt_price_limit_1, sqrt_price_limit_2)
    }
    
    pub fn get_swap_tick_arrays(ctx: Context<GetSwapTickArrays>, amount_in: u64, sqrt_price_limit: u128, zero_for_one: bool) -> Result<instructions::swap_compute_tick::SwapTickArrayInfo> {
        instructions::swap_compute_tick::handler(ctx, amount_in, sqrt_price_limit, zero_for_one)
    }
    
    pub fn initialize_transient_updates(ctx: Context<InitializeTransientUpdates>) -> Result<()> {
        instructions::keeper_update_tick::initialize_transient_updates(ctx)
    }
    
    pub fn add_tick_update(ctx: Context<AddTickUpdate>, tick_array_pubkey: Pubkey, tick_index: i32, liquidity_net_delta: i128) -> Result<()> {
        instructions::keeper_update_tick::add_tick_update(ctx, tick_array_pubkey, tick_index, liquidity_net_delta)
    }
    
    pub fn finalize_transient_updates(ctx: Context<FinalizeTransientUpdates>) -> Result<()> {
        instructions::keeper_update_tick::finalize_transient_updates(ctx)
    }
    
    pub fn cleanup_transient_updates(ctx: Context<CleanupTransientUpdates>) -> Result<()> {
        instructions::keeper_update_tick::cleanup_transient_updates(ctx)
    }
    
    pub fn reset_transient_updates(ctx: Context<ResetTransientUpdates>) -> Result<()> {
        instructions::keeper_update_tick::reset_transient_updates(ctx)
    }
    
    pub fn cleanup_empty_tick_array(ctx: Context<CleanupEmptyTickArray>) -> Result<()> {
        instructions::tick_cleanup::handler_empty(ctx)
    }
}