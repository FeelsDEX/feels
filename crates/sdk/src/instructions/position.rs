/// SDK builders for Position flow instructions (FeelsSOL <-> Position).
/// Provides convenient client-side interfaces for position management.

use anchor_lang::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use feels::instructions::{EnterPositionParams, ExitPositionParams, PositionType};

// ============================================================================
// Position Result Types
// ============================================================================

/// Result of a liquidity operation
#[derive(Debug, Clone)]
pub struct LiquidityResult {
    pub position_pubkey: Pubkey,
    pub position_mint: Pubkey,
    pub liquidity_amount: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub signature: Signature,
}

/// Result of a liquidity addition operation (alias)
pub type AddLiquidityResult = LiquidityResult;

/// Position information
#[derive(Debug, Clone)]
pub struct PositionInfo {
    pub pubkey: Pubkey,
    pub mint: Pubkey,
    pub pool: Pubkey,
    pub owner: Pubkey,
    pub liquidity: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub fee_growth_0_checkpoint: u128,
    pub fee_growth_1_checkpoint: u128,
    pub tokens_owed_0: u64,
    pub tokens_owed_1: u64,
}

/// Builder for entering a position (FeelsSOL -> Position)
pub struct EnterPositionBuilder {
    /// User entering the position
    pub user: Pubkey,
    /// FeelsSOL mint address
    pub feelssol_mint: Pubkey,
    /// Position token mint
    pub position_mint: Pubkey,
    /// Position token metadata account
    pub position_token: Pubkey,
    /// Position FeelsSOL vault
    pub position_feelssol_vault: Pubkey,
    /// Market field account
    pub market_field: Pubkey,
    /// Buffer account
    pub buffer: Pubkey,
    /// Oracle account
    pub oracle: Pubkey,
    /// Field commitment account
    pub field_commitment: Pubkey,
    /// Amount of FeelsSOL to convert
    pub amount_in: u64,
    /// Type of position
    pub position_type: PositionType,
    /// Minimum position tokens to receive
    pub min_position_tokens: u64,
}

impl EnterPositionBuilder {
    /// Create a new position entry builder
    pub fn new(
        user: Pubkey,
        feelssol_mint: Pubkey,
        position_mint: Pubkey,
        amount_in: u64,
        position_type: PositionType,
    ) -> Self {
        Self {
            user,
            feelssol_mint,
            position_mint,
            position_token: Pubkey::default(),
            position_feelssol_vault: Pubkey::default(),
            market_field: Pubkey::default(),
            buffer: Pubkey::default(),
            oracle: Pubkey::default(),
            field_commitment: Pubkey::default(),
            amount_in,
            position_type,
            min_position_tokens: 0,
        }
    }

    /// Set position metadata account
    pub fn with_position_token(mut self, position_token: Pubkey) -> Self {
        self.position_token = position_token;
        self
    }

    /// Set position vault
    pub fn with_position_vault(mut self, vault: Pubkey) -> Self {
        self.position_feelssol_vault = vault;
        self
    }

    /// Set market accounts
    pub fn with_market_accounts(
        mut self,
        market_field: Pubkey,
        buffer: Pubkey,
        oracle: Pubkey,
        field_commitment: Pubkey,
    ) -> Self {
        self.market_field = market_field;
        self.buffer = buffer;
        self.oracle = oracle;
        self.field_commitment = field_commitment;
        self
    }

    /// Set minimum tokens out (slippage protection)
    pub fn with_min_tokens_out(mut self, min_tokens: u64) -> Self {
        self.min_position_tokens = min_tokens;
        self
    }

    /// Derive user token accounts
    pub fn user_feelssol_account(&self) -> Pubkey {
        anchor_spl::associated_token::get_associated_token_address(
            &self.user,
            &self.feelssol_mint,
        )
    }

    pub fn user_position_account(&self) -> Pubkey {
        anchor_spl::associated_token::get_associated_token_address(
            &self.user,
            &self.position_mint,
        )
    }

    /// Build the instruction
    pub fn build(&self, program_id: Pubkey) -> Result<Instruction> {
        let accounts = feels::accounts::EnterPosition {
            user: self.user,
            user_feelssol_account: self.user_feelssol_account(),
            user_position_account: self.user_position_account(),
            feelssol_mint: self.feelssol_mint,
            position_mint: self.position_mint,
            position_token: self.position_token,
            position_feelssol_vault: self.position_feelssol_vault,
            market_field: self.market_field,
            buffer: self.buffer,
            oracle: self.oracle,
            field_commitment: self.field_commitment,
            token_program: anchor_spl::token_2022::ID,
        };

        let params = EnterPositionParams {
            amount_in: self.amount_in,
            position_type: self.position_type.clone(),
            min_position_tokens: self.min_position_tokens,
        };

        let data = feels::instruction::EnterPosition { params };

        Ok(Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        })
    }
}

/// Builder for exiting a position (Position -> FeelsSOL)
pub struct ExitPositionBuilder {
    /// User exiting the position
    pub user: Pubkey,
    /// Position token mint
    pub position_mint: Pubkey,
    /// FeelsSOL mint address
    pub feelssol_mint: Pubkey,
    /// Position token metadata account
    pub position_token: Pubkey,
    /// Position FeelsSOL vault
    pub position_feelssol_vault: Pubkey,
    /// Market field account
    pub market_field: Pubkey,
    /// Buffer account
    pub buffer: Pubkey,
    /// Oracle account
    pub oracle: Pubkey,
    /// Field commitment account
    pub field_commitment: Pubkey,
    /// Amount of position tokens to convert
    pub amount_in: u64,
    /// Minimum FeelsSOL to receive
    pub min_feelssol_out: u64,
}

impl ExitPositionBuilder {
    /// Create a new position exit builder
    pub fn new(
        user: Pubkey,
        position_mint: Pubkey,
        feelssol_mint: Pubkey,
        amount_in: u64,
    ) -> Self {
        Self {
            user,
            position_mint,
            feelssol_mint,
            position_token: Pubkey::default(),
            position_feelssol_vault: Pubkey::default(),
            market_field: Pubkey::default(),
            buffer: Pubkey::default(),
            oracle: Pubkey::default(),
            field_commitment: Pubkey::default(),
            amount_in,
            min_feelssol_out: 0,
        }
    }

    /// Set position metadata account
    pub fn with_position_token(mut self, position_token: Pubkey) -> Self {
        self.position_token = position_token;
        self
    }

    /// Set position vault
    pub fn with_position_vault(mut self, vault: Pubkey) -> Self {
        self.position_feelssol_vault = vault;
        self
    }

    /// Set market accounts
    pub fn with_market_accounts(
        mut self,
        market_field: Pubkey,
        buffer: Pubkey,
        oracle: Pubkey,
        field_commitment: Pubkey,
    ) -> Self {
        self.market_field = market_field;
        self.buffer = buffer;
        self.oracle = oracle;
        self.field_commitment = field_commitment;
        self
    }

    /// Set minimum FeelsSOL out (slippage protection)
    pub fn with_min_feelssol_out(mut self, min_feelssol: u64) -> Self {
        self.min_feelssol_out = min_feelssol;
        self
    }

    /// Derive user token accounts
    pub fn user_position_account(&self) -> Pubkey {
        anchor_spl::associated_token::get_associated_token_address(
            &self.user,
            &self.position_mint,
        )
    }

    pub fn user_feelssol_account(&self) -> Pubkey {
        anchor_spl::associated_token::get_associated_token_address(
            &self.user,
            &self.feelssol_mint,
        )
    }

    /// Build the instruction
    pub fn build(&self, program_id: Pubkey) -> Result<Instruction> {
        let accounts = feels::accounts::ExitPosition {
            user: self.user,
            user_position_account: self.user_position_account(),
            user_feelssol_account: self.user_feelssol_account(),
            position_mint: self.position_mint,
            feelssol_mint: self.feelssol_mint,
            position_token: self.position_token,
            position_feelssol_vault: self.position_feelssol_vault,
            market_field: self.market_field,
            buffer: self.buffer,
            oracle: self.oracle,
            field_commitment: self.field_commitment,
            token_program: anchor_spl::token_2022::ID,
        };

        let params = ExitPositionParams {
            amount_in: self.amount_in,
            min_feelssol_out: self.min_feelssol_out,
        };

        let data = feels::instruction::ExitPosition { params };

        Ok(Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        })
    }
}