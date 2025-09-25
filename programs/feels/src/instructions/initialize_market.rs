//! Initialize market instruction
//!
//! Creates a new market with commitment to initial liquidity deployment.
//! The actual liquidity deployment happens in a separate instruction.

use crate::{
    constants::{
        BUFFER_SEED, ESCROW_AUTHORITY_SEED, MARKET_AUTHORITY_SEED, MARKET_SEED,
        PROTOCOL_TOKEN_SEED, VAULT_SEED,
    },
    error::FeelsError,
    events::MarketInitialized,
    state::{
        Buffer, Market, MarketPhase, OracleState, PhaseTrigger, PolicyV1, PreLaunchEscrow,
        ProtocolToken, TokenOrigin, TokenType,
    },
    utils::{calculate_token_out_from_sqrt_price, tick_from_sqrt_price},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, spl_token::instruction::AuthorityType, Mint, Token, TokenAccount};
use solana_program::program_pack::Pack;
use spl_token::state::Account as TokenAccountState;

/// Initialize market parameters
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InitializeMarketParams {
    /// Base fee in basis points (e.g., 30 = 0.3%)
    pub base_fee_bps: u16,
    /// Tick spacing for the market
    pub tick_spacing: u16,
    /// Initial price (as sqrt_price Q64)
    pub initial_sqrt_price: u128,
    /// Optional initial buy amount in FeelsSOL (0 = no initial buy)
    pub initial_buy_feelssol_amount: u64,
}

/// Initialize market accounts
#[derive(Accounts)]
#[instruction(params: InitializeMarketParams)]
pub struct InitializeMarket<'info> {
    /// Creator initializing the market
    #[account(mut)]
    pub creator: Signer<'info>,

    /// Token 0 mint (lower pubkey)
    #[account(mut)]
    pub token_0: Account<'info, Mint>,

    /// Token 1 mint (higher pubkey)
    #[account(mut)]
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
        seeds = [VAULT_SEED, token_0.key().as_ref(), token_1.key().as_ref(), b"0"],
        bump,
        token::mint = token_0,
        token::authority = market_authority,
    )]
    pub vault_0: Account<'info, TokenAccount>,

    /// Vault 1 for token 1
    #[account(
        init,
        payer = creator,
        seeds = [VAULT_SEED, token_0.key().as_ref(), token_1.key().as_ref(), b"1"],
        bump,
        token::mint = token_1,
        token::authority = market_authority,
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
    /// CHECK: Validated as either token_0 or token_1
    pub feelssol_mint: AccountInfo<'info>,

    /// Protocol token registry for token_0 (if not FeelsSOL)
    /// CHECK: Can be a dummy account if token_0 is FeelsSOL
    pub protocol_token_0: AccountInfo<'info>,

    /// Protocol token registry for token_1 (if not FeelsSOL)
    /// CHECK: Can be a dummy account if token_1 is FeelsSOL
    pub protocol_token_1: AccountInfo<'info>,

    /// Pre-launch escrow for the protocol token
    /// CHECK: Validated manually in handler
    #[account(mut)]
    pub escrow: AccountInfo<'info>,

    /// Creator's FeelsSOL account for initial buy
    /// CHECK: Only validated if initial_buy_feelssol_amount > 0
    pub creator_feelssol: AccountInfo<'info>,

    /// Creator's token account for receiving initial buy tokens
    /// CHECK: Only validated if initial_buy_feelssol_amount > 0
    pub creator_token_out: AccountInfo<'info>,

    /// Escrow authority PDA (holds mint/freeze authorities)
    /// CHECK: PDA validated in handler
    #[account(mut)]
    pub escrow_authority: AccountInfo<'info>,

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
    // Early validation - fail fast before accessing mutable references

    // 1. Validate parameters first
    crate::utils::validate_base_fee_bps(params.base_fee_bps)?;
    crate::utils::validate_tick_spacing_param(params.tick_spacing)?;
    crate::utils::validate_initial_sqrt_price(params.initial_sqrt_price)?;

    // 2. Validate token order
    let token_0_bytes = ctx.accounts.token_0.key().to_bytes();
    let token_1_bytes = ctx.accounts.token_1.key().to_bytes();
    require!(token_0_bytes < token_1_bytes, FeelsError::InvalidTokenOrder);

    // 3. Manually deserialize and validate escrow account
    msg!("Attempting to deserialize escrow");
    msg!("  Escrow account: {}", ctx.accounts.escrow.key());

    let escrow_data = ctx.accounts.escrow.try_borrow_data()?;
    msg!("  Escrow data length: {}", escrow_data.len());
    msg!("  Escrow first 8 bytes: {:?}", &escrow_data[0..8]);

    require!(escrow_data.len() >= 8, FeelsError::InvalidAccount);

    // Deserialize the escrow
    let mut escrow_slice = &escrow_data[..];
    let escrow: PreLaunchEscrow =
        PreLaunchEscrow::try_deserialize(&mut escrow_slice).map_err(|e| {
            msg!("Failed to deserialize escrow: {:?}", e);
            FeelsError::InvalidAccount
        })?;

    msg!("  escrow.token_mint: {}", escrow.token_mint);
    msg!("  escrow.market: {}", escrow.market);
    msg!("  escrow.creator: {}", escrow.creator);

    // Validate escrow is for one of the tokens
    let escrow_for_token_0 = escrow.token_mint == ctx.accounts.token_0.key();
    let escrow_for_token_1 = escrow.token_mint == ctx.accounts.token_1.key();
    require!(
        escrow_for_token_0 || escrow_for_token_1,
        FeelsError::InvalidAccount
    );

    // Validate escrow market is not yet set
    require!(
        escrow.market == Pubkey::default(),
        FeelsError::MarketAlreadyInitialized
    );

    // Validate creator matches escrow creator
    require!(
        escrow.creator == ctx.accounts.creator.key(),
        FeelsError::UnauthorizedSigner
    );

    // 4. Check that at least one token is FeelsSOL
    let token_0_is_feelssol = ctx.accounts.token_0.key() == ctx.accounts.feelssol_mint.key();
    let token_1_is_feelssol = ctx.accounts.token_1.key() == ctx.accounts.feelssol_mint.key();

    msg!("Token validation:");
    msg!("  token_0: {}", ctx.accounts.token_0.key());
    msg!("  token_1: {}", ctx.accounts.token_1.key());
    msg!("  feelssol_mint: {}", ctx.accounts.feelssol_mint.key());
    msg!("  token_0_is_feelssol: {}", token_0_is_feelssol);
    msg!("  token_1_is_feelssol: {}", token_1_is_feelssol);

    require!(
        token_0_is_feelssol || token_1_is_feelssol,
        FeelsError::RequiresFeelsSOLPair
    );

    // Additional validation: FeelsSOL must be token_0
    require!(token_0_is_feelssol, FeelsError::InvalidTokenOrder);

    // 4. If initial buy requested, validate accounts exist
    if params.initial_buy_feelssol_amount > 0 {
        require!(
            !ctx.accounts.creator_feelssol.data_is_empty(),
            FeelsError::InvalidAuthority
        );
        require!(
            !ctx.accounts.creator_token_out.data_is_empty(),
            FeelsError::InvalidAuthority
        );
    }

    // Now safe to get mutable references
    let market = &mut ctx.accounts.market;
    let buffer = &mut ctx.accounts.buffer;
    let oracle = &mut ctx.accounts.oracle;
    let clock = Clock::get()?;

    // Determine token types and origins (similar to old initialize_market)
    let mut token_0_type = TokenType::Spl;
    let mut token_1_type = TokenType::Spl;
    let token_0_origin;
    let token_1_origin;

    // Check token origins
    if token_0_is_feelssol {
        token_0_origin = TokenOrigin::FeelsSOL;
    } else {
        // Verify it's protocol-minted
        let (expected_protocol_token_0, _) = Pubkey::find_program_address(
            &[PROTOCOL_TOKEN_SEED, ctx.accounts.token_0.key().as_ref()],
            ctx.program_id,
        );

        // If it's not the expected protocol token PDA, it might be a dummy account
        if ctx.accounts.protocol_token_0.key() != expected_protocol_token_0 {
            return Err(FeelsError::TokenNotProtocolMinted.into());
        }

        if let Ok(protocol_token_data) = ctx.accounts.protocol_token_0.try_borrow_data() {
            if protocol_token_data.len() >= 8 {
                token_0_origin = TokenOrigin::ProtocolMinted;
                let protocol_token: ProtocolToken =
                    ProtocolToken::try_from_slice(&protocol_token_data[8..])?;
                require!(
                    protocol_token.mint == ctx.accounts.token_0.key(),
                    FeelsError::TokenNotProtocolMinted
                );
                // Verify creator is launching their own token's market
                require!(
                    protocol_token.creator == ctx.accounts.creator.key(),
                    FeelsError::UnauthorizedSigner
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

        // If it's not the expected protocol token PDA, it might be a dummy account
        if ctx.accounts.protocol_token_1.key() != expected_protocol_token_1 {
            return Err(FeelsError::TokenNotProtocolMinted.into());
        }

        if let Ok(protocol_token_data) = ctx.accounts.protocol_token_1.try_borrow_data() {
            if protocol_token_data.len() >= 8 {
                token_1_origin = TokenOrigin::ProtocolMinted;
                let protocol_token: ProtocolToken =
                    ProtocolToken::try_from_slice(&protocol_token_data[8..])?;
                require!(
                    protocol_token.mint == ctx.accounts.token_1.key(),
                    FeelsError::TokenNotProtocolMinted
                );
                // Verify creator is launching their own token's market
                require!(
                    protocol_token.creator == ctx.accounts.creator.key(),
                    FeelsError::UnauthorizedSigner
                );
                token_1_type = protocol_token.token_type;
            } else {
                return Err(FeelsError::TokenNotProtocolMinted.into());
            }
        } else {
            return Err(FeelsError::TokenNotProtocolMinted.into());
        }
    }

    // Reject Token-2022, not currently supported
    require!(
        token_0_type == TokenType::Spl && token_1_type == TokenType::Spl,
        FeelsError::Token2022NotSupported
    );

    // Now that creator verification is complete, revoke mint and freeze authorities
    // This ensures token has fixed supply and cannot be frozen

    // Check and revoke authorities for token_0 if it's protocol-minted
    if token_0_origin == TokenOrigin::ProtocolMinted {
        // Validate escrow authority PDA
        let expected_escrow_authority = Pubkey::find_program_address(
            &[ESCROW_AUTHORITY_SEED, ctx.accounts.escrow.key().as_ref()],
            ctx.program_id,
        )
        .0;
        require!(
            ctx.accounts.escrow_authority.key() == expected_escrow_authority,
            FeelsError::InvalidAuthority
        );

        // Check if escrow authority has mint authority (transferred from creator in mint_token)
        if ctx.accounts.token_0.mint_authority.is_some()
            && ctx.accounts.token_0.mint_authority.unwrap() == ctx.accounts.escrow_authority.key()
        {
            // Revoke mint authority using escrow authority seeds
            let escrow_key = ctx.accounts.escrow.key();
            let escrow_authority_seeds = &[
                ESCROW_AUTHORITY_SEED,
                escrow_key.as_ref(),
                &[escrow.escrow_authority_bump],
            ];

            token::set_authority(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    token::SetAuthority {
                        current_authority: ctx.accounts.escrow_authority.to_account_info(),
                        account_or_mint: ctx.accounts.token_0.to_account_info(),
                    },
                    &[escrow_authority_seeds],
                ),
                AuthorityType::MintTokens,
                None, // Permanently disable minting
            )?;
            msg!("Revoked mint authority for token_0");
        }

        // Check if escrow authority has freeze authority
        if ctx.accounts.token_0.freeze_authority.is_some()
            && ctx.accounts.token_0.freeze_authority.unwrap() == ctx.accounts.escrow_authority.key()
        {
            // Revoke freeze authority using escrow authority seeds
            let escrow_key = ctx.accounts.escrow.key();
            let escrow_authority_seeds = &[
                ESCROW_AUTHORITY_SEED,
                escrow_key.as_ref(),
                &[escrow.escrow_authority_bump],
            ];

            token::set_authority(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    token::SetAuthority {
                        current_authority: ctx.accounts.escrow_authority.to_account_info(),
                        account_or_mint: ctx.accounts.token_0.to_account_info(),
                    },
                    &[escrow_authority_seeds],
                ),
                AuthorityType::FreezeAccount,
                None, // Permanently disable freezing
            )?;
            msg!("Revoked freeze authority for token_0");
        }
    }

    // Check and revoke authorities for token_1 if it's protocol-minted
    if token_1_origin == TokenOrigin::ProtocolMinted {
        // Check if escrow authority has mint authority (transferred from creator in mint_token)
        if ctx.accounts.token_1.mint_authority.is_some()
            && ctx.accounts.token_1.mint_authority.unwrap() == ctx.accounts.escrow_authority.key()
        {
            // Revoke mint authority using escrow authority seeds
            let escrow_key = ctx.accounts.escrow.key();
            let escrow_authority_seeds = &[
                ESCROW_AUTHORITY_SEED,
                escrow_key.as_ref(),
                &[escrow.escrow_authority_bump],
            ];

            token::set_authority(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    token::SetAuthority {
                        current_authority: ctx.accounts.escrow_authority.to_account_info(),
                        account_or_mint: ctx.accounts.token_1.to_account_info(),
                    },
                    &[escrow_authority_seeds],
                ),
                AuthorityType::MintTokens,
                None, // Permanently disable minting
            )?;
            msg!("Revoked mint authority for token_1");
        }

        // Check if escrow authority has freeze authority
        if ctx.accounts.token_1.freeze_authority.is_some()
            && ctx.accounts.token_1.freeze_authority.unwrap() == ctx.accounts.escrow_authority.key()
        {
            // Revoke freeze authority using escrow authority seeds
            let escrow_key = ctx.accounts.escrow.key();
            let escrow_authority_seeds = &[
                ESCROW_AUTHORITY_SEED,
                escrow_key.as_ref(),
                &[escrow.escrow_authority_bump],
            ];

            token::set_authority(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    token::SetAuthority {
                        current_authority: ctx.accounts.escrow_authority.to_account_info(),
                        account_or_mint: ctx.accounts.token_1.to_account_info(),
                    },
                    &[escrow_authority_seeds],
                ),
                AuthorityType::FreezeAccount,
                None, // Permanently disable freezing
            )?;
            msg!("Revoked freeze authority for token_1");
        }
    }

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
    market.jit_enabled = true; // default on per docs
    market.jit_base_cap_bps = 300; // 3% base cap
    market.jit_per_slot_cap_bps = 500; // 5% per slot cap
    market.jit_concentration_width = 10; // 10 ticks concentration width
    market.jit_max_multiplier = 10; // 10x max concentration
    market.jit_drain_protection_bps = 7000; // 70% drain protection threshold
    market.jit_circuit_breaker_bps = 3000; // 30% circuit breaker threshold
    market.floor_tick = market.global_lower_tick; // start at lowest
    market.floor_buffer_ticks = (market.tick_spacing as i32) * 2;
    market.last_floor_ratchet_ts = 0;
    market.floor_cooldown_secs = 300; // 5 minutes default
    market.steady_state_seeded = false;
    market.cleanup_complete = false;

    // Initialize phase tracking
    market.phase = MarketPhase::Created as u8;
    market.phase_start_slot = clock.slot;
    market.phase_start_timestamp = clock.unix_timestamp;
    market.last_phase_transition_slot = 0;
    market.last_phase_trigger = PhaseTrigger::Creator as u8;
    market.total_volume_token_0 = 0;
    market.total_volume_token_1 = 0;

    // Initialize v0.5 directional tracking
    market.rolling_buy_volume = 0;
    market.rolling_sell_volume = 0;
    market.rolling_total_volume = 0;
    market.rolling_window_start_slot = clock.slot;
    market.tick_snapshot_1hr = market.current_tick;
    market.last_snapshot_timestamp = clock.unix_timestamp;

    market._reserved = [0; 1];

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
    buffer.jit_last_slot = 0;
    buffer.jit_slot_used_q = 0;

    // Initialize v0.5 JIT tracking
    buffer.jit_rolling_consumption = 0;
    buffer.jit_rolling_window_start = clock.slot;
    buffer.jit_last_heavy_usage_slot = 0;
    buffer.jit_total_consumed_epoch = 0;
    buffer.initial_tau_spot = 0; // Will be set when buffer receives initial funding

    // Initialize oracle
    oracle.initialize(
        market.key(),
        ctx.bumps.oracle,
        market.current_tick,
        clock.unix_timestamp,
    )?;

    // Handle initial buy if requested
    if params.initial_buy_feelssol_amount > 0 {
        // Load and validate token accounts
        let creator_feelssol_data = &ctx.accounts.creator_feelssol.try_borrow_data()?;
        let creator_feelssol = TokenAccountState::unpack(creator_feelssol_data)?;

        let creator_token_out_data = &ctx.accounts.creator_token_out.try_borrow_data()?;
        let creator_token_out = TokenAccountState::unpack(creator_token_out_data)?;

        // Determine which token is FeelsSOL and validate accounts
        let (feelssol_in, token_out_mint) = if token_0_is_feelssol {
            // Buying token_1 with FeelsSOL
            require!(
                creator_feelssol.mint == ctx.accounts.token_0.key(),
                FeelsError::InvalidMint
            );
            require!(
                creator_token_out.mint == ctx.accounts.token_1.key(),
                FeelsError::InvalidMint
            );
            (true, ctx.accounts.token_1.key())
        } else {
            // Buying token_0 with FeelsSOL
            require!(
                creator_feelssol.mint == ctx.accounts.token_1.key(),
                FeelsError::InvalidMint
            );
            require!(
                creator_token_out.mint == ctx.accounts.token_0.key(),
                FeelsError::InvalidMint
            );
            (false, ctx.accounts.token_0.key())
        };

        // Validate creator has enough FeelsSOL
        require!(
            creator_feelssol.amount >= params.initial_buy_feelssol_amount,
            FeelsError::InsufficientBalance
        );

        // Transfer FeelsSOL to vault
        let feelssol_vault = if feelssol_in {
            &ctx.accounts.vault_0
        } else {
            &ctx.accounts.vault_1
        };

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.creator_feelssol.to_account_info(),
                    to: feelssol_vault.to_account_info(),
                    authority: ctx.accounts.creator.to_account_info(),
                },
            ),
            params.initial_buy_feelssol_amount,
        )?;

        // Calculate output amount based on initial price
        // Since there's no liquidity yet, we calculate based on the initial price directly
        let token_0_decimals = ctx.accounts.token_0.decimals;
        let token_1_decimals = ctx.accounts.token_1.decimals;

        // Calculate the output amount
        let output_amount = calculate_token_out_from_sqrt_price(
            params.initial_buy_feelssol_amount,
            params.initial_sqrt_price,
            token_0_decimals,
            token_1_decimals,
            feelssol_in, // true if token_0 (FeelsSOL) is input
        )?;

        msg!("Initial buy calculated:");
        msg!("  FeelsSOL in: {}", params.initial_buy_feelssol_amount);
        msg!("  Token out: {}", output_amount);
        msg!("  Token out mint: {}", token_out_mint);
        msg!("  At sqrt price: {}", params.initial_sqrt_price);

        // Note: The actual transfer of output tokens would happen after liquidity deployment
        // from the buffer's token vault. For now, we've transferred the FeelsSOL to the vault
        // and calculated the expected output. The execution will complete when liquidity is deployed.
    }

    // Update escrow to link to the new market
    // We need to deserialize, modify, and serialize back
    {
        let mut escrow_data = ctx.accounts.escrow.try_borrow_mut_data()?;
        let mut escrow_slice = &escrow_data[..];
        let mut escrow: PreLaunchEscrow = PreLaunchEscrow::try_deserialize(&mut escrow_slice)
            .map_err(|_| FeelsError::InvalidAccount)?;

        escrow.market = market.key();

        // Serialize back
        let mut data = &mut escrow_data[..];
        escrow.try_serialize(&mut data)?;
    }

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
