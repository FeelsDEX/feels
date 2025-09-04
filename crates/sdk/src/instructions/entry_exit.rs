/// SDK builders for Entry/Exit protocol instructions (JitoSOL <-> FeelsSOL).
/// Provides convenient client-side interfaces for entering and exiting the protocol.

use anchor_lang::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use crate::accounts::*;
use feels::instructions::{EnterProtocolParams, ExitProtocolParams};

/// Builder for entering the protocol (JitoSOL -> FeelsSOL)
pub struct EnterProtocolBuilder {
    /// User entering the protocol
    pub user: Pubkey,
    /// JitoSOL mint address
    pub jitosol_mint: Pubkey,
    /// FeelsSOL mint address
    pub feelssol_mint: Pubkey,
    /// Market field account
    pub market_field: Pubkey,
    /// Buffer account
    pub buffer: Pubkey,
    /// Oracle account
    pub oracle: Pubkey,
    /// Field commitment account
    pub field_commitment: Pubkey,
    /// Amount of JitoSOL to convert
    pub amount_in: u64,
    /// Minimum FeelsSOL to receive
    pub min_amount_out: u64,
}

impl EnterProtocolBuilder {
    /// Create a new entry builder
    pub fn new(
        user: Pubkey,
        jitosol_mint: Pubkey,
        feelssol_mint: Pubkey,
        amount_in: u64,
    ) -> Self {
        Self {
            user,
            jitosol_mint,
            feelssol_mint,
            market_field: Pubkey::default(),
            buffer: Pubkey::default(),
            oracle: Pubkey::default(),
            field_commitment: Pubkey::default(),
            amount_in,
            min_amount_out: 0,
        }
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

    /// Set minimum amount out (slippage protection)
    pub fn with_min_amount_out(mut self, min_amount_out: u64) -> Self {
        self.min_amount_out = min_amount_out;
        self
    }

    /// Derive user token accounts
    pub fn user_jitosol_account(&self) -> Pubkey {
        anchor_spl::associated_token::get_associated_token_address(
            &self.user,
            &self.jitosol_mint,
        )
    }

    pub fn user_feelssol_account(&self) -> Pubkey {
        anchor_spl::associated_token::get_associated_token_address(
            &self.user,
            &self.feelssol_mint,
        )
    }

    /// Derive protocol JitoSOL vault
    pub fn protocol_jitosol_vault(&self, program_id: &Pubkey) -> Pubkey {
        let (vault, _) = Pubkey::find_program_address(
            &[b"vault", self.jitosol_mint.as_ref()],
            program_id,
        );
        vault
    }

    /// Build the instruction
    pub fn build(&self, program_id: Pubkey) -> Result<Instruction> {
        let accounts = feels::accounts::EnterProtocol {
            user: self.user,
            user_jitosol_account: self.user_jitosol_account(),
            user_feelssol_account: self.user_feelssol_account(),
            jitosol_mint: self.jitosol_mint,
            feelssol_mint: self.feelssol_mint,
            protocol_jitosol_vault: self.protocol_jitosol_vault(&program_id),
            market_field: self.market_field,
            buffer: self.buffer,
            oracle: self.oracle,
            field_commitment: self.field_commitment,
            token_program: anchor_spl::token_2022::ID,
        };

        let params = EnterProtocolParams {
            amount_in: self.amount_in,
            min_amount_out: self.min_amount_out,
        };

        let data = feels::instruction::EnterProtocol { params };

        Ok(Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        })
    }
}

/// Builder for exiting the protocol (FeelsSOL -> JitoSOL)
pub struct ExitProtocolBuilder {
    /// User exiting the protocol
    pub user: Pubkey,
    /// FeelsSOL mint address
    pub feelssol_mint: Pubkey,
    /// JitoSOL mint address
    pub jitosol_mint: Pubkey,
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
    /// Minimum JitoSOL to receive
    pub min_amount_out: u64,
}

impl ExitProtocolBuilder {
    /// Create a new exit builder
    pub fn new(
        user: Pubkey,
        feelssol_mint: Pubkey,
        jitosol_mint: Pubkey,
        amount_in: u64,
    ) -> Self {
        Self {
            user,
            feelssol_mint,
            jitosol_mint,
            market_field: Pubkey::default(),
            buffer: Pubkey::default(),
            oracle: Pubkey::default(),
            field_commitment: Pubkey::default(),
            amount_in,
            min_amount_out: 0,
        }
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

    /// Set minimum amount out (slippage protection)
    pub fn with_min_amount_out(mut self, min_amount_out: u64) -> Self {
        self.min_amount_out = min_amount_out;
        self
    }

    /// Derive user token accounts
    pub fn user_feelssol_account(&self) -> Pubkey {
        anchor_spl::associated_token::get_associated_token_address(
            &self.user,
            &self.feelssol_mint,
        )
    }

    pub fn user_jitosol_account(&self) -> Pubkey {
        anchor_spl::associated_token::get_associated_token_address(
            &self.user,
            &self.jitosol_mint,
        )
    }

    /// Derive protocol JitoSOL vault
    pub fn protocol_jitosol_vault(&self, program_id: &Pubkey) -> Pubkey {
        let (vault, _) = Pubkey::find_program_address(
            &[b"vault", self.jitosol_mint.as_ref()],
            program_id,
        );
        vault
    }

    /// Build the instruction
    pub fn build(&self, program_id: Pubkey) -> Result<Instruction> {
        let accounts = feels::accounts::ExitProtocol {
            user: self.user,
            user_feelssol_account: self.user_feelssol_account(),
            user_jitosol_account: self.user_jitosol_account(),
            feelssol_mint: self.feelssol_mint,
            jitosol_mint: self.jitosol_mint,
            protocol_jitosol_vault: self.protocol_jitosol_vault(&program_id),
            market_field: self.market_field,
            buffer: self.buffer,
            oracle: self.oracle,
            field_commitment: self.field_commitment,
            token_program: anchor_spl::token_2022::ID,
        };

        let params = ExitProtocolParams {
            amount_in: self.amount_in,
            min_amount_out: self.min_amount_out,
        };

        let data = feels::instruction::ExitProtocol { params };

        Ok(Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        })
    }
}

/// Helper to calculate minimum output with slippage tolerance
pub fn calculate_min_output(amount: u64, slippage_bps: u16) -> u64 {
    let slippage_factor = 10_000u64.saturating_sub(slippage_bps as u64);
    amount
        .saturating_mul(slippage_factor)
        .saturating_div(10_000)
}