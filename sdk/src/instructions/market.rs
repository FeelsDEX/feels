use anchor_lang::prelude::*;
use solana_sdk::instruction::Instruction;

use crate::{
    core::SdkResult,
    impl_instruction,
    instructions::{FeelsInstructionBuilder, InstructionBuilder},
    protocol::PdaBuilder,
};

// Instruction discriminators
const TRANSITION_MARKET_PHASE_DISCRIMINATOR: [u8; 8] = [192, 45, 250, 40, 31, 139, 115, 62];
const GRADUATE_POOL_DISCRIMINATOR: [u8; 8] = [210, 29, 144, 133, 25, 219, 183, 247];
const CLEANUP_BONDING_CURVE_DISCRIMINATOR: [u8; 8] = [205, 225, 206, 146, 97, 186, 14, 238];
const DESTROY_EXPIRED_TOKEN_DISCRIMINATOR: [u8; 8] = [72, 107, 101, 121, 217, 54, 144, 155];
const INITIALIZE_TRANCHE_TICKS_DISCRIMINATOR: [u8; 8] = [118, 74, 31, 238, 66, 167, 66, 93];
const UPDATE_DEX_TWAP_DISCRIMINATOR: [u8; 8] = [144, 64, 180, 12, 223, 33, 140, 232];

/// Parameters for transitioning market phase
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct TransitionMarketPhaseParams {
    pub new_phase: u8, // MarketPhase enum value
}

impl_instruction!(TransitionMarketPhaseParams, TRANSITION_MARKET_PHASE_DISCRIMINATOR);

/// Parameters for graduating pool
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct GraduatePoolParams {
    pub target_pool: Pubkey,
}

impl_instruction!(GraduatePoolParams, GRADUATE_POOL_DISCRIMINATOR);

/// Parameters for cleaning up bonding curve (no params)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CleanupBondingCurveParams {}

impl_instruction!(CleanupBondingCurveParams, CLEANUP_BONDING_CURVE_DISCRIMINATOR);

/// Parameters for destroying expired token
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct DestroyExpiredTokenParams {
    pub refund_recipient: Pubkey,
}

impl_instruction!(DestroyExpiredTokenParams, DESTROY_EXPIRED_TOKEN_DISCRIMINATOR);

/// Parameters for initializing tranche ticks
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeTrancheTicksParams {
    pub start_tick_index: i32,
}

impl_instruction!(InitializeTrancheTicksParams, INITIALIZE_TRANCHE_TICKS_DISCRIMINATOR);

/// Parameters for updating DEX TWAP (no params)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UpdateDexTwapParams {}

impl_instruction!(UpdateDexTwapParams, UPDATE_DEX_TWAP_DISCRIMINATOR);

/// Market instruction builder
pub struct MarketInstructionBuilder {
    pda: PdaBuilder,
}

impl MarketInstructionBuilder {
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            pda: PdaBuilder::new(program_id),
        }
    }

    /// Build transition market phase instruction
    pub fn transition_market_phase(
        &self,
        authority: Pubkey,
        market: Pubkey,
        new_phase: u8,
    ) -> SdkResult<Instruction> {
        let (protocol_config, _) = self.pda.protocol_config();

        Ok(FeelsInstructionBuilder::new()
            .add_signer(authority)
            .add_writable(market)
            .add_readonly(protocol_config)
            .add_readonly(solana_sdk::sysvar::clock::id())
            .with_data(
                TransitionMarketPhaseParams { new_phase }
                    .build_data()?,
            )
            .build())
    }

    /// Build graduate pool instruction
    pub fn graduate_pool(
        &self,
        creator: Pubkey,
        market: Pubkey,
        target_pool: Pubkey,
        feelssol_mint: Pubkey,
        other_mint: Pubkey,
    ) -> SdkResult<Instruction> {
        let (buffer, _) = self.pda.buffer(&market);
        let (vault_authority, _) = self.pda.vault_authority(&market);

        // Derive vault addresses
        let (vault_0, _) = Pubkey::find_program_address(
            &[b"vault", feelssol_mint.as_ref(), other_mint.as_ref(), b"0"],
            &self.pda.program_id,
        );
        let (vault_1, _) = Pubkey::find_program_address(
            &[b"vault", feelssol_mint.as_ref(), other_mint.as_ref(), b"1"],
            &self.pda.program_id,
        );

        Ok(FeelsInstructionBuilder::new()
            .add_signer(creator)
            .add_writable(market)
            .add_readonly(buffer)
            .add_writable(vault_0)
            .add_writable(vault_1)
            .add_readonly(vault_authority)
            .add_readonly(target_pool)
            .add_readonly(spl_token::id())
            .add_readonly(solana_sdk::sysvar::clock::id())
            .with_data(GraduatePoolParams { target_pool }.build_data()?)
            .build())
    }

    /// Build cleanup bonding curve instruction
    pub fn cleanup_bonding_curve(
        &self,
        authority: Pubkey,
        market: Pubkey,
        feelssol_mint: Pubkey,
        other_mint: Pubkey,
    ) -> SdkResult<Instruction> {
        let (vault_authority, _) = self.pda.vault_authority(&market);
        
        // Derive vault addresses
        let (vault_0, _) = Pubkey::find_program_address(
            &[b"vault", feelssol_mint.as_ref(), other_mint.as_ref(), b"0"],
            &self.pda.program_id,
        );
        let (vault_1, _) = Pubkey::find_program_address(
            &[b"vault", feelssol_mint.as_ref(), other_mint.as_ref(), b"1"],
            &self.pda.program_id,
        );

        Ok(FeelsInstructionBuilder::new()
            .add_signer(authority)
            .add_writable(market)
            .add_writable(vault_0)
            .add_writable(vault_1)
            .add_readonly(vault_authority)
            .add_readonly(spl_token::id())
            .with_data(CleanupBondingCurveParams {}.build_data()?)
            .build())
    }

    /// Build destroy expired token instruction
    pub fn destroy_expired_token(
        &self,
        anyone: Pubkey,
        market: Pubkey,
        expired_mint: Pubkey,
        refund_recipient: Pubkey,
    ) -> SdkResult<Instruction> {
        let (vault_authority, _) = self.pda.vault_authority(&market);

        Ok(FeelsInstructionBuilder::new()
            .add_signer(anyone)
            .add_writable(market)
            .add_writable(expired_mint)
            .add_readonly(vault_authority)
            .add_readonly(solana_sdk::sysvar::clock::id())
            .add_readonly(spl_token::id())
            .with_data(
                DestroyExpiredTokenParams { refund_recipient }
                    .build_data()?,
            )
            .build())
    }

    /// Build initialize tranche ticks instruction
    pub fn initialize_tranche_ticks(
        &self,
        payer: Pubkey,
        market: Pubkey,
        start_tick_index: i32,
    ) -> SdkResult<Instruction> {
        // Derive the tick arrays for this tranche
        let mut builder = FeelsInstructionBuilder::new()
            .add_signer(payer)
            .add_writable(market)
            .add_readonly(solana_sdk::system_program::id());

        // Add tick arrays for the tranche (usually multiple)
        // This is simplified - in practice would need tick spacing info
        for i in 0..3 {
            let tick_index = start_tick_index + (i * crate::core::TICK_ARRAY_SIZE);
            let (tick_array, _) = self.pda.tick_array(&market, tick_index);
            builder = builder.add_writable(tick_array);
        }

        Ok(builder
            .with_data(
                InitializeTrancheTicksParams { start_tick_index }
                    .build_data()?,
            )
            .build())
    }

    /// Build update DEX TWAP instruction
    pub fn update_dex_twap(
        &self,
        market: Pubkey,
        target_dex_pool: Pubkey,
    ) -> SdkResult<Instruction> {
        let (oracle, _) = self.pda.oracle(&market);

        Ok(FeelsInstructionBuilder::new()
            .add_readonly(market)
            .add_writable(oracle)
            .add_readonly(target_dex_pool)
            .add_readonly(solana_sdk::sysvar::clock::id())
            .with_data(UpdateDexTwapParams {}.build_data()?)
            .build())
    }
}