//! Initial liquidity commitment structures
//! 
//! Defines the commitment made at market creation for initial
//! liquidity deployment. This allows market initialization
//! to be decoupled from the initial liquidity strategy.

use anchor_lang::prelude::*;

/// Commitment for a single position in the initial liquidity deployment
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct PositionCommitment {
    /// Lower tick of the position
    pub tick_lower: i32,
    /// Upper tick of the position
    pub tick_upper: i32,
    /// Liquidity amount for this position
    pub liquidity: u128,
    /// Pre-generated position mint pubkey
    pub position_mint: Pubkey,
}

/// Initial liquidity commitment stored in market
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct InitialLiquidityCommitment {
    /// Expected amount of token 0 to deploy
    pub token_0_amount: u64,
    /// Expected amount of token 1 to deploy
    pub token_1_amount: u64,
    /// Authority who can deploy the initial liquidity
    pub deployer: Pubkey,
    /// Unix timestamp by which liquidity must be deployed
    pub deploy_by: i64,
    /// Committed positions to create
    pub position_commitments: Vec<PositionCommitment>,
}

impl InitialLiquidityCommitment {
    /// Calculate serialized size for a given number of positions
    pub fn size_for_positions(num_positions: usize) -> usize {
        8 + // token_0_amount
        8 + // token_1_amount
        32 + // deployer
        8 + // deploy_by
        4 + // vec length
        (num_positions * (4 + 4 + 16 + 32)) // positions
    }
}