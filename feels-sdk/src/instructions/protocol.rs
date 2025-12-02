use crate::prelude::*;
use solana_sdk::instruction::Instruction;

use crate::{
    core::SdkResult,
    impl_instruction,
    instructions::{FeelsInstructionBuilder, InstructionBuilder},
    protocol::PdaBuilder,
};

// Instruction discriminators
const INITIALIZE_PROTOCOL_DISCRIMINATOR: [u8; 8] = [188, 233, 252, 106, 134, 146, 202, 91];
const UPDATE_PROTOCOL_DISCRIMINATOR: [u8; 8] = [206, 25, 218, 114, 109, 41, 74, 173];
const UPDATE_FLOOR_DISCRIMINATOR: [u8; 8] = [38, 80, 204, 37, 6, 62, 192, 200];
const SET_PROTOCOL_OWNED_OVERRIDE_DISCRIMINATOR: [u8; 8] = [250, 164, 109, 69, 170, 65, 157, 140];
const INITIALIZE_HUB_DISCRIMINATOR: [u8; 8] = [202, 27, 126, 27, 54, 182, 68, 169];
const UPDATE_NATIVE_RATE_DISCRIMINATOR: [u8; 8] = [100, 175, 161, 10, 254, 80, 99, 77];

/// Parameters for initializing protocol
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeProtocolParams {
    /// Initial mint fee in FeelsSOL lamports
    pub mint_fee: u64,
    /// Treasury account to receive fees
    pub treasury: Pubkey,
    /// Default protocol fee rate (basis points, e.g. 100 = 1%)
    pub default_protocol_fee_rate: Option<u16>,
    /// Default creator fee rate for protocol tokens (basis points, e.g. 50 = 0.5%)
    pub default_creator_fee_rate: Option<u16>,
    /// Maximum allowed protocol fee rate (basis points)
    pub max_protocol_fee_rate: Option<u16>,
    /// DEX TWAP updater authority
    pub dex_twap_updater: Pubkey,
    /// De-peg threshold (bps)
    pub depeg_threshold_bps: u16,
    /// Consecutive breaches to pause
    pub depeg_required_obs: u8,
    /// Consecutive clears to resume
    pub clear_required_obs: u8,
    /// DEX TWAP window seconds
    pub dex_twap_window_secs: u32,
    /// DEX TWAP stale age seconds
    pub dex_twap_stale_age_secs: u32,
    /// Initial DEX whitelist (optional; empty ok)
    pub dex_whitelist: Vec<Pubkey>,
}

impl_instruction!(InitializeProtocolParams, INITIALIZE_PROTOCOL_DISCRIMINATOR);

/// Parameters for updating protocol
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UpdateProtocolParams {
    pub new_authority: Option<Pubkey>,
    pub new_treasury: Option<Pubkey>,
    pub new_protocol_fee_share_bps: Option<u16>,
    pub new_oracle_authority: Option<Pubkey>,
}

impl_instruction!(UpdateProtocolParams, UPDATE_PROTOCOL_DISCRIMINATOR);

/// Parameters for initializing hub
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeHubParams {
    pub jitosol_mint: Pubkey,
}

impl_instruction!(InitializeHubParams, INITIALIZE_HUB_DISCRIMINATOR);

/// Parameters for updating floor (no params)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UpdateFloorParams {}

impl_instruction!(UpdateFloorParams, UPDATE_FLOOR_DISCRIMINATOR);

/// Parameters for setting protocol owned override
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct SetProtocolOwnedOverrideParams {
    pub override_amount: u64,
}

impl_instruction!(
    SetProtocolOwnedOverrideParams,
    SET_PROTOCOL_OWNED_OVERRIDE_DISCRIMINATOR
);

/// Parameters for updating native rate (no params)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UpdateNativeRateParams {}

impl_instruction!(UpdateNativeRateParams, UPDATE_NATIVE_RATE_DISCRIMINATOR);

/// Protocol instruction builder
pub struct ProtocolInstructionBuilder {
    pda: PdaBuilder,
}

impl ProtocolInstructionBuilder {
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            pda: PdaBuilder::new(program_id),
        }
    }

    /// Build initialize protocol instruction
    pub fn initialize_protocol(
        &self,
        authority: Pubkey,
        params: InitializeProtocolParams,
    ) -> SdkResult<Instruction> {
        let (protocol_config, _) = self.pda.protocol_config();
        let (protocol_oracle, _) = self.pda.protocol_oracle();
        let (safety, _) = Pubkey::find_program_address(&[b"safety_controller"], &self.pda.program_id);

        Ok(FeelsInstructionBuilder::with_program_id(self.pda.program_id)
            .add_signer(authority)
            .add_writable(protocol_config)
            .add_readonly(solana_program::system_program::id())
            .add_writable(protocol_oracle)
            .add_writable(safety)
            .with_data(params.build_data()?)
            .build())
    }
    
    /// Helper to create default InitializeProtocolParams
    pub fn default_init_params(
        treasury: Pubkey,
        dex_twap_updater: Pubkey,
    ) -> InitializeProtocolParams {
        InitializeProtocolParams {
            mint_fee: 1_000_000, // 1 FeelsSOL
            treasury,
            default_protocol_fee_rate: Some(100), // 1%
            default_creator_fee_rate: Some(50),   // 0.5%
            max_protocol_fee_rate: Some(1000),    // 10%
            dex_twap_updater,
            depeg_threshold_bps: 100,             // 1%
            depeg_required_obs: 3,
            clear_required_obs: 3,
            dex_twap_window_secs: 300,            // 5 minutes
            dex_twap_stale_age_secs: 600,         // 10 minutes
            dex_whitelist: vec![],
        }
    }

    /// Build update protocol instruction
    pub fn update_protocol(
        &self,
        authority: Pubkey,
        params: UpdateProtocolParams,
    ) -> SdkResult<Instruction> {
        let (protocol_config, _) = self.pda.protocol_config();

        Ok(FeelsInstructionBuilder::with_program_id(self.pda.program_id)
            .add_signer(authority)
            .add_writable(protocol_config)
            .with_data(params.build_data()?)
            .build())
    }

    /// Build initialize hub instruction
    pub fn initialize_hub(
        &self,
        payer: Pubkey,
        params: InitializeHubParams,
    ) -> SdkResult<Instruction> {
        let (feels_mint, _) = self.pda.feels_mint();
        let (feels_hub, _) = Pubkey::find_program_address(
            &[b"feels_hub", feels_mint.as_ref()],
            &self.pda.program_id
        );
        let (jitosol_vault, _) = Pubkey::find_program_address(
            &[b"jitosol_vault", feels_mint.as_ref()],
            &self.pda.program_id
        );
        let (vault_authority, _) = Pubkey::find_program_address(
            &[b"vault_authority", feels_mint.as_ref()],
            &self.pda.program_id
        );

        Ok(FeelsInstructionBuilder::with_program_id(self.pda.program_id)
            .add_signer(payer)           // payer (mut)
            .add_readonly(feels_mint)     // feelssol_mint
            .add_readonly(params.jitosol_mint)  // jitosol_mint
            .add_writable(feels_hub)      // hub (init)
            .add_writable(jitosol_vault)  // jitosol_vault (init)
            .add_readonly(vault_authority)  // vault_authority
            .add_readonly(spl_token::id())      // token_program
            .add_readonly(solana_program::system_program::id())  // system_program
            .with_data(params.build_data()?)
            .build())
    }

    /// Build update floor instruction
    pub fn update_floor(
        &self,
        market: Pubkey,
        vault_0: Pubkey,
        vault_1: Pubkey,
        project_mint: Pubkey,
        escrow_token_account: Option<Pubkey>,
    ) -> SdkResult<Instruction> {
        let (buffer, _) = self.pda.buffer(&market);

        let mut builder = FeelsInstructionBuilder::new()
            .add_writable(market)
            .add_readonly(buffer)
            .add_writable(vault_0)
            .add_writable(vault_1)
            .add_readonly(project_mint);

        // Add optional escrow account
        if let Some(escrow) = escrow_token_account {
            builder = builder.add_readonly(escrow);
        } else {
            builder = builder.add_optional(None);
        }

        builder = builder.add_readonly(solana_program::sysvar::clock::id());

        Ok(builder
            .with_data(UpdateFloorParams {}.build_data()?)
            .build())
    }

    /// Build set protocol owned override instruction
    pub fn set_protocol_owned_override(
        &self,
        authority: Pubkey,
        buffer: Pubkey,
        override_amount: u64,
    ) -> SdkResult<Instruction> {
        let (protocol_config, _) = self.pda.protocol_config();

        Ok(FeelsInstructionBuilder::new()
            .add_readonly(protocol_config)
            .add_writable(buffer)
            .add_signer(authority)
            .with_data(SetProtocolOwnedOverrideParams { override_amount }.build_data()?)
            .build())
    }

    /// Build update native rate instruction  
    pub fn update_native_rate(
        &self,
        market: Pubkey,
        feelssol_mint: Pubkey,
    ) -> SdkResult<Instruction> {
        let (oracle, _) = self.pda.oracle(&market);
        let (feels_hub, _) = self.pda.feels_hub();

        Ok(FeelsInstructionBuilder::new()
            .add_readonly(market)
            .add_writable(oracle)
            .add_readonly(feelssol_mint)
            .add_readonly(feels_hub)
            .add_readonly(solana_program::sysvar::clock::id())
            .with_data(UpdateNativeRateParams {}.build_data()?)
            .build())
    }
}
