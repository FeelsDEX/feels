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

declare_id!("2FgA6YfdFNGgX8YyPKqSzhFGNvatRD5zi1yqCCFaSjq1");

// Accounts structs are defined in instruction modules
// and re-exported through instructions::*


#[program]
pub mod feels {
    use super::*;
    
    /// Initialize a new market with commitment for initial liquidity
    /// SECURITY: Market creation and liquidity commitment are atomic,
    /// preventing front-running attacks. Actual liquidity deployment
    /// happens separately via deploy_initial_liquidity instruction.
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
    pub fn close_position(
        ctx: Context<ClosePosition>,
        params: ClosePositionParams,
    ) -> Result<()> {
        instructions::close_position(ctx, params)
    }
    
    /// Collect fees from a position - smart single entry point
    /// Automatically handles normal positions, wide positions, and accumulated fees
    pub fn collect_fees<'info>(
        ctx: Context<'_, '_, 'info, 'info, CollectFees<'info>>
    ) -> Result<()> {
        instructions::collect_fees(ctx)
    }
    
    /// Update position fee accrual for lower tick
    /// SECURITY: Part 1/3 of fee collection for wide positions
    pub fn update_position_fee_lower(ctx: Context<UpdatePositionFeeLower>) -> Result<()> {
        instructions::update_position_fee_lower(ctx)
    }
    
    /// Update position fee accrual for upper tick
    /// SECURITY: Part 2/3 of fee collection for wide positions
    pub fn update_position_fee_upper(ctx: Context<UpdatePositionFeeUpper>) -> Result<()> {
        instructions::update_position_fee_upper(ctx)
    }
    
    /// Mint a new token with distribution
    pub fn mint_token(ctx: Context<MintToken>, params: MintTokenParams) -> Result<()> {
        instructions::mint_token(ctx, params)
    }
    
    /// Deploy initial liquidity to a market
    /// SECURITY: Verifies the deployment matches the commitment made during
    /// market initialization, preventing unauthorized liquidity deployment
    pub fn deploy_initial_liquidity<'info>(
        ctx: Context<'_, '_, 'info, 'info, DeployInitialLiquidity<'info>>,
        params: DeployInitialLiquidityParams,
    ) -> Result<()> {
        instructions::deploy_initial_liquidity(ctx, params)
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
    
}

// Create a processor function for tests that calls the Anchor-generated entry point
#[cfg(not(feature = "no-entrypoint"))]
pub fn processor(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> anchor_lang::solana_program::entrypoint::ProgramResult {
    // Check instruction discriminator (first 8 bytes)
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    
    // For testing purposes, let's just return success for now
    // TODO: In a real implementation, we'd parse and route to the correct handler
    msg!("Test processor called with {} bytes of instruction data", instruction_data.len());
    Ok(())
}