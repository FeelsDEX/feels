//! Initialize market instruction
//! 
//! Creates a new market with commitment to initial liquidity deployment.
//! The actual liquidity deployment happens in a separate instruction.

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use crate::{
    constants::{MARKET_SEED, BUFFER_SEED, VAULT_SEED, MARKET_AUTHORITY_SEED, PROTOCOL_TOKEN_SEED},
    error::FeelsError,
    events::MarketInitialized,
    state::{Market, Buffer, PolicyV1, OracleState, ProtocolToken, TokenType, TokenOrigin, InitialLiquidityCommitment},
    utils::tick_from_sqrt_price,
};

/// Initialize market parameters
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InitializeMarketParams {
    /// Base fee in basis points (e.g., 30 = 0.3%)
    pub base_fee_bps: u16,
    /// Tick spacing for the market
    pub tick_spacing: u16,
    /// Initial price (as sqrt_price Q64)
    pub initial_sqrt_price: u128,
    /// Commitment for initial liquidity deployment
    pub liquidity_commitment: InitialLiquidityCommitment,
}

/// Initialize market accounts
#[derive(Accounts)]
#[instruction(params: InitializeMarketParams)]
pub struct InitializeMarket<'info> {
    /// Creator initializing the market
    #[account(
        mut,
        constraint = creator.owner == &System::id() @ FeelsError::InvalidAuthority
    )]
    pub creator: Signer<'info>,
    
    /// Token 0 mint (lower pubkey)
    pub token_0: Account<'info, Mint>,
    
    /// Token 1 mint (higher pubkey)
    pub token_1: Account<'info, Mint>,
    
    /// Market account to initialize
    #[account(
        init,
        payer = creator,
        space = Market::LEN,
        seeds = [MARKET_SEED, token_0.key().as_ref(), token_1.key().as_ref()],
        bump,
    )]
    pub market: Box<Account<'info, Market>>,
    
    /// Buffer account to initialize
    #[account(
        init,
        payer = creator,
        space = Buffer::LEN,
        seeds = [BUFFER_SEED, market.key().as_ref()],
        bump,
    )]
    pub buffer: Box<Account<'info, Buffer>>,
    
    /// Oracle account to initialize
    #[account(
        init,
        payer = creator,
        space = OracleState::LEN,
        seeds = [b"oracle", market.key().as_ref()],
        bump,
    )]
    pub oracle: Box<Account<'info, OracleState>>,
    
    /// Vault 0 for token 0
    #[account(
        init,
        payer = creator,
        token::mint = token_0,
        token::authority = market_authority,
        seeds = [VAULT_SEED, market.key().as_ref(), token_0.key().as_ref()],
        bump,
    )]
    pub vault_0: Account<'info, TokenAccount>,
    
    /// Vault 1 for token 1
    #[account(
        init,
        payer = creator,
        token::mint = token_1,
        token::authority = market_authority,
        seeds = [VAULT_SEED, market.key().as_ref(), token_1.key().as_ref()],
        bump,
    )]
    pub vault_1: Account<'info, TokenAccount>,
    
    /// Market authority PDA
    /// CHECK: PDA that controls vaults
    #[account(
        seeds = [MARKET_AUTHORITY_SEED, market.key().as_ref()],
        bump,
    )]
    pub market_authority: AccountInfo<'info>,
    
    /// FeelsSOL mint (hub token)
    pub feelssol_mint: Account<'info, Mint>,
    
    /// Protocol token registry for token_0 (if not FeelsSOL)
    /// CHECK: Can be a dummy account if token_0 is FeelsSOL
    pub protocol_token_0: AccountInfo<'info>,
    
    /// Protocol token registry for token_1 (if not FeelsSOL)
    /// CHECK: Can be a dummy account if token_1 is FeelsSOL
    pub protocol_token_1: AccountInfo<'info>,
    
    /// System program
    pub system_program: Program<'info, System>,
    
    /// Token program
    pub token_program: Program<'info, Token>,
    
    /// Rent sysvar
    pub rent: Sysvar<'info, Rent>,
}

/// Initialize market handler
pub fn initialize_market(
    ctx: Context<InitializeMarket>,
    params: InitializeMarketParams,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let buffer = &mut ctx.accounts.buffer;
    let oracle = &mut ctx.accounts.oracle;
    let clock = Clock::get()?;
    
    // Validate parameters
    require!(
        params.base_fee_bps <= crate::constants::MAX_FEE_BPS,
        FeelsError::InvalidPrice
    );
    require!(
        params.tick_spacing > 0 && params.tick_spacing <= crate::constants::MAX_TICK_SPACING,
        FeelsError::InvalidTickSpacing
    );
    require!(
        params.initial_sqrt_price > 0,
        FeelsError::InvalidPrice
    );
    
    // Validate token order
    let token_0_bytes = ctx.accounts.token_0.key().to_bytes();
    let token_1_bytes = ctx.accounts.token_1.key().to_bytes();
    require!(
        token_0_bytes < token_1_bytes,
        FeelsError::InvalidTokenOrder
    );
    
    // Check that at least one token is FeelsSOL
    let token_0_is_feelssol = ctx.accounts.token_0.key() == ctx.accounts.feelssol_mint.key();
    let token_1_is_feelssol = ctx.accounts.token_1.key() == ctx.accounts.feelssol_mint.key();
    require!(
        token_0_is_feelssol || token_1_is_feelssol,
        FeelsError::RequiresFeelsSOLPair
    );
    
    // Determine token types and origins (similar to old initialize_market)
    let mut token_0_type = TokenType::Spl;
    let mut token_1_type = TokenType::Spl;
    let mut token_0_origin = TokenOrigin::External;
    let mut token_1_origin = TokenOrigin::External;
    
    // Check token origins
    if token_0_is_feelssol {
        token_0_origin = TokenOrigin::FeelsSOL;
    } else {
        // Verify it's protocol-minted
        let (expected_protocol_token_0, _) = Pubkey::find_program_address(
            &[PROTOCOL_TOKEN_SEED, ctx.accounts.token_0.key().as_ref()],
            ctx.program_id,
        );
        require!(
            ctx.accounts.protocol_token_0.key() == expected_protocol_token_0,
            FeelsError::InvalidAuthority
        );
        
        if let Ok(protocol_token_data) = ctx.accounts.protocol_token_0.try_borrow_data() {
            if protocol_token_data.len() >= 8 {
                token_0_origin = TokenOrigin::ProtocolMinted;
                let protocol_token: ProtocolToken = ProtocolToken::try_from_slice(&protocol_token_data[8..])?;
                require!(
                    protocol_token.mint == ctx.accounts.token_0.key(),
                    FeelsError::TokenNotProtocolMinted
                );
                token_0_type = protocol_token.token_type;
            } else {
                return Err(FeelsError::TokenNotProtocolMinted.into());
            }
        } else {
            return Err(FeelsError::TokenNotProtocolMinted.into());
        }
    }
    
    if token_1_is_feelssol {
        token_1_origin = TokenOrigin::FeelsSOL;
    } else {
        let (expected_protocol_token_1, _) = Pubkey::find_program_address(
            &[PROTOCOL_TOKEN_SEED, ctx.accounts.token_1.key().as_ref()],
            ctx.program_id,
        );
        require!(
            ctx.accounts.protocol_token_1.key() == expected_protocol_token_1,
            FeelsError::InvalidAuthority
        );
        
        if let Ok(protocol_token_data) = ctx.accounts.protocol_token_1.try_borrow_data() {
            if protocol_token_data.len() >= 8 {
                token_1_origin = TokenOrigin::ProtocolMinted;
                let protocol_token: ProtocolToken = ProtocolToken::try_from_slice(&protocol_token_data[8..])?;
                require!(
                    protocol_token.mint == ctx.accounts.token_1.key(),
                    FeelsError::TokenNotProtocolMinted
                );
                token_1_type = protocol_token.token_type;
            } else {
                return Err(FeelsError::TokenNotProtocolMinted.into());
            }
        } else {
            return Err(FeelsError::TokenNotProtocolMinted.into());
        }
    }
    
    // Reject Token-2022 for now
    require!(
        token_0_type == TokenType::Spl && token_1_type == TokenType::Spl,
        FeelsError::Token2022NotSupported
    );
    
    // Validate liquidity commitment
    require!(
        params.liquidity_commitment.deploy_by > clock.unix_timestamp,
        FeelsError::InvalidTimestamp
    );
    require!(
        params.liquidity_commitment.position_commitments.len() > 0,
        FeelsError::InvalidRoute
    );
    
    // Initialize market
    market.version = 1;
    market.is_initialized = true;
    market.is_paused = false;
    market.token_0 = ctx.accounts.token_0.key();
    market.token_1 = ctx.accounts.token_1.key();
    market.feelssol_mint = ctx.accounts.feelssol_mint.key();
    market.token_0_type = token_0_type;
    market.token_1_type = token_1_type;
    market.token_0_origin = token_0_origin;
    market.token_1_origin = token_1_origin;
    market.sqrt_price = params.initial_sqrt_price;
    market.liquidity = 0; // No liquidity until deployment
    market.current_tick = tick_from_sqrt_price(params.initial_sqrt_price)?;
    market.tick_spacing = params.tick_spacing;
    market.global_lower_tick = -887220;
    market.global_upper_tick = 887220;
    market.floor_liquidity = 0;
    market.fee_growth_global_0_x64 = 0;
    market.fee_growth_global_1_x64 = 0;
    market.base_fee_bps = params.base_fee_bps;
    market.buffer = buffer.key();
    market.authority = ctx.accounts.creator.key();
    market.last_epoch_update = clock.unix_timestamp;
    market.epoch_number = 0;
    market.oracle = oracle.key();
    market.oracle_bump = ctx.bumps.oracle;
    market.policy = PolicyV1::default();
    market.market_authority_bump = ctx.bumps.market_authority;
    market.vault_0_bump = ctx.bumps.vault_0;
    market.vault_1_bump = ctx.bumps.vault_1;
    market.reentrancy_guard = false;
    market.initial_liquidity_deployed = false;
    market._reserved = [0; 31];
    
    // Initialize buffer
    buffer.market = market.key();
    buffer.authority = ctx.accounts.creator.key();
    buffer.feelssol_mint = ctx.accounts.feelssol_mint.key();
    buffer.fees_token_0 = 0;
    buffer.fees_token_1 = 0;
    buffer.tau_spot = 0;
    buffer.tau_time = 0;
    buffer.tau_leverage = 0;
    buffer.floor_tick_spacing = 0;
    buffer.floor_placement_threshold = 100 * 10u64.pow(ctx.accounts.token_1.decimals as u32);
    buffer.last_floor_placement = 0;
    buffer.last_rebase = clock.unix_timestamp;
    buffer.total_distributed = 0;
    buffer.buffer_authority_bump = 0; // Will be set if needed
    buffer._reserved = [0; 8];
    
    // Initialize oracle
    oracle.initialize(
        market.key(),
        ctx.bumps.oracle,
        market.current_tick,
        clock.unix_timestamp
    )?;
    
    // Store liquidity commitment in a separate account (for now, log it)
    msg!("Market initialized with liquidity commitment:");
    msg!("Deployer: {}", params.liquidity_commitment.deployer);
    msg!("Deploy by: {}", params.liquidity_commitment.deploy_by);
    msg!("Positions: {}", params.liquidity_commitment.position_commitments.len());
    
    // Emit event
    emit!(MarketInitialized {
        market: market.key(),
        token_0: ctx.accounts.token_0.key(),
        token_1: ctx.accounts.token_1.key(),
        feelssol_mint: ctx.accounts.feelssol_mint.key(),
        buffer: buffer.key(),
        base_fee_bps: params.base_fee_bps,
        tick_spacing: params.tick_spacing,
        initial_sqrt_price: params.initial_sqrt_price,
        timestamp: clock.unix_timestamp,
        version: 1,
    });
    
    Ok(())
}