//! Feels Protocol

// Suppress specific deprecated warnings from Anchor/Solana runtime
#![allow(deprecated_safe)]
#![allow(deprecated)]

pub mod constants;
pub mod error;
pub mod events;
pub mod instructions;
pub mod logic;
pub mod macros;
pub mod state;
pub mod utils;

use anchor_lang::prelude::*;

// Import instruction modules using Raydium pattern
// This makes all Accounts structs available at crate root
use instructions::*;
use state::PoolPhase;

declare_id!("FQSZnecUCVc2HnKsdPgNic641etrPT7gYiSic9NDPuTx");

// Accounts structs are defined in instruction modules
// and re-exported through instructions::*

#[program]
pub mod feels {
    use super::*;

    /// Initialize protocol configuration (one-time setup)
    pub fn initialize_protocol(
        ctx: Context<InitializeProtocol>,
        params: InitializeProtocolParams,
    ) -> Result<()> {
        instructions::initialize_protocol(ctx, params)
    }

    /// Permissionless floor update crank (computes floor from reserves & supply)
    pub fn update_floor(ctx: Context<UpdateFloor>) -> Result<()> {
        instructions::update_floor(ctx)
    }

    /// Update protocol configuration
    pub fn update_protocol(
        ctx: Context<UpdateProtocol>,
        params: UpdateProtocolParams,
    ) -> Result<()> {
        instructions::update_protocol(ctx, params)
    }

    /// Set protocol owned override for floor calculation (governance only)
    pub fn set_protocol_owned_override(
        ctx: Context<SetProtocolOwnedOverride>,
        override_amount: u64,
    ) -> Result<()> {
        instructions::set_protocol_owned_override(ctx, override_amount)
    }

    /// Initialize the pool registry (one-time setup)
    pub fn initialize_pool_registry(ctx: Context<InitializePoolRegistry>) -> Result<()> {
        instructions::initialize_pool_registry(ctx)
    }

    /// Register a pool in the registry
    pub fn register_pool(ctx: Context<RegisterPool>) -> Result<()> {
        instructions::register_pool(ctx)
    }

    /// Update pool phase in registry
    pub fn update_pool_phase(ctx: Context<UpdatePoolPhase>, new_phase: PoolPhase) -> Result<()> {
        instructions::update_pool_phase(ctx, new_phase)
    }

    /// Initialize a POMM (Protocol-Owned Market Making) position
    pub fn initialize_pomm_position(
        ctx: Context<InitializePommPosition>,
        position_index: u8,
    ) -> Result<()> {
        instructions::initialize_pomm_position(ctx, position_index)
    }

    /// Manage POMM (Protocol-Owned Market Making) positions
    pub fn manage_pomm_position(
        ctx: Context<ManagePommPosition>,
        params: ManagePommParams,
    ) -> Result<()> {
        instructions::manage_pomm_position(ctx, params)
    }

    /// Transition market between phases
    pub fn transition_market_phase(
        ctx: Context<TransitionMarketPhase>,
        params: TransitionPhaseParams,
    ) -> Result<()> {
        instructions::transition_market_phase(ctx, params)
    }

    /// Initialize a new market with commitment for initial liquidity
    /// Market creation and liquidity commitment are atomic, preventing
    /// front-running. Actual liquidity deployment happens separately via
    /// deploy_initial_liquidity instruction.
    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        params: InitializeMarketParams,
    ) -> Result<()> {
        instructions::initialize_market(ctx, params)
    }

    /// Enter FeelsSOL - deposit JitoSOL to mint FeelsSOL
    pub fn enter_feelssol(ctx: Context<EnterFeelsSOL>, amount: u64) -> Result<()> {
        instructions::enter_feelssol(ctx, amount)
    }

    /// Exit FeelsSOL - burn FeelsSOL to redeem JitoSOL
    pub fn exit_feelssol(ctx: Context<ExitFeelsSOL>, amount: u64) -> Result<()> {
        instructions::exit_feelssol(ctx, amount)
    }

    /// Initialize FeelsHub for enter/exit operations
    pub fn initialize_hub(ctx: Context<InitializeHub>) -> Result<()> {
        instructions::initialize_hub(ctx)
    }

    /// Swap tokens through the AMM
    pub fn swap<'info>(
        ctx: Context<'_, '_, 'info, 'info, Swap<'info>>,
        params: SwapParams,
    ) -> Result<()> {
        // Box the params to reduce stack usage
        let params = Box::new(params);
        instructions::swap(ctx, *params)
    }

    /// Open a new liquidity position
    pub fn open_position(
        ctx: Context<OpenPosition>,
        tick_lower: i32,
        tick_upper: i32,
        liquidity_amount: u128,
    ) -> Result<()> {
        instructions::open_position(ctx, tick_lower, tick_upper, liquidity_amount)
    }

    /// Close a liquidity position
    pub fn close_position(ctx: Context<ClosePosition>, params: ClosePositionParams) -> Result<()> {
        instructions::close_position(ctx, params)
    }

    /// Collect fees from a position - smart single entry point
    /// Automatically handles normal positions, wide positions, and accumulated fees
    pub fn collect_fees<'info>(
        ctx: Context<'_, '_, 'info, 'info, CollectFees<'info>>,
    ) -> Result<()> {
        instructions::collect_fees(ctx)
    }

    /// Update position fee accrual for lower tick
    /// Part 1/3 of fee collection for wide positions
    pub fn update_position_fee_lower(ctx: Context<UpdatePositionFeeLower>) -> Result<()> {
        instructions::update_position_fee_lower(ctx)
    }

    /// Update position fee accrual for upper tick
    /// Part 2/3 of fee collection for wide positions
    pub fn update_position_fee_upper(ctx: Context<UpdatePositionFeeUpper>) -> Result<()> {
        instructions::update_position_fee_upper(ctx)
    }

    /// Mint a new token with distribution
    pub fn mint_token(ctx: Context<MintToken>, params: MintTokenParams) -> Result<()> {
        instructions::mint_token(ctx, params)
    }

    /// Deploy initial liquidity to a market
    /// Verifies the deployment matches the commitment made during market
    /// initialization, preventing unauthorized liquidity deployment
    pub fn deploy_initial_liquidity<'info>(
        ctx: Context<'_, '_, 'info, 'info, DeployInitialLiquidity<'info>>,
        params: DeployInitialLiquidityParams,
    ) -> Result<()> {
        instructions::deploy_initial_liquidity(ctx, params)
    }

    /// Permissionless crank to initialize tranche TickArrays and boundary ticks
    pub fn initialize_tranche_ticks<'info>(
        ctx: Context<'_, '_, 'info, 'info, InitializeTrancheTicks<'info>>,
        params: InitializeTrancheTicksParams,
    ) -> Result<()> {
        instructions::initialize_tranche_ticks(ctx, params)
    }

    /// Cleanup bonding curve plan and mark cleanup complete
    pub fn cleanup_bonding_curve(ctx: Context<CleanupBondingCurve>) -> Result<()> {
        instructions::cleanup_bonding_curve(ctx)
    }

    /// Open a position with NFT metadata
    pub fn open_position_with_metadata(
        ctx: Context<OpenPositionWithMetadata>,
        tick_lower: i32,
        tick_upper: i32,
        liquidity_amount: u128,
    ) -> Result<()> {
        instructions::open_position_with_metadata(ctx, tick_lower, tick_upper, liquidity_amount)
    }

    /// Close a position with NFT metadata
    pub fn close_position_with_metadata(
        ctx: Context<ClosePositionWithMetadata>,
        amount_0_min: u64,
        amount_1_min: u64,
    ) -> Result<()> {
        // For metadata version, always close the account
        let _params = ClosePositionParams {
            amount_0_min,
            amount_1_min,
            close_account: true,
        };
        instructions::close_position_with_metadata(ctx, amount_0_min, amount_1_min)
    }

    /// Destroy an expired token that hasn't had liquidity deployed
    pub fn destroy_expired_token(ctx: Context<DestroyExpiredToken>) -> Result<()> {
        instructions::destroy_expired_token(ctx)
    }

    /// Graduate pool to steady state (idempotent)
    pub fn graduate_pool(ctx: Context<GraduatePool>) -> Result<()> {
        instructions::graduate_pool(ctx)
    }

    /// Update DEX TWAP for protocol oracle (keeper-updated)
    pub fn update_dex_twap(ctx: Context<UpdateDexTwap>, params: UpdateDexTwapParams) -> Result<()> {
        instructions::update_dex_twap(ctx, params)
    }

    /// Update native reserve rate for protocol oracle (authority)
    pub fn update_native_rate(
        ctx: Context<UpdateNativeRate>,
        params: UpdateNativeRateParams,
    ) -> Result<()> {
        instructions::update_native_rate(ctx, params)
    }
}

// Minimal processor for non-anchor entrypoint tests: validate discriminator length.
#[cfg(not(feature = "no-entrypoint"))]
pub fn processor(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> anchor_lang::solana_program::entrypoint::ProgramResult {
    if instruction_data.len() < 8 {
        // must contain an Anchor discriminator
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}
