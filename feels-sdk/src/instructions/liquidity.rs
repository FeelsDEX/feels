use crate::prelude::*;
use solana_sdk::instruction::Instruction;

use crate::{
    core::{SdkError, SdkResult},
    impl_instruction,
    instructions::{FeelsInstructionBuilder, InstructionBuilder},
    protocol::PdaBuilder,
};

// Instruction discriminators
const ENTER_FEELSSOL_DISCRIMINATOR: [u8; 8] = [0xc7, 0xcd, 0x31, 0xad, 0x51, 0x32, 0xba, 0x7e];
const EXIT_FEELSSOL_DISCRIMINATOR: [u8; 8] = [0x69, 0x76, 0xa8, 0x94, 0x3d, 0x98, 0x03, 0xaf];
const OPEN_POSITION_DISCRIMINATOR: [u8; 8] = [0x87, 0x80, 0x2f, 0x4d, 0x0f, 0x98, 0xf0, 0x31];
const CLOSE_POSITION_DISCRIMINATOR: [u8; 8] = [0x7b, 0x86, 0x51, 0x00, 0x31, 0x44, 0x62, 0x62];
const COLLECT_FEES_DISCRIMINATOR: [u8; 8] = [164, 152, 207, 99, 30, 186, 19, 182];
const INITIALIZE_MARKET_DISCRIMINATOR: [u8; 8] = [0x23, 0x23, 0xbd, 0xc1, 0x9b, 0x30, 0xaa, 0xcb];
const MINT_TOKEN_DISCRIMINATOR: [u8; 8] = [0xac, 0x89, 0xb7, 0x0e, 0xcf, 0x6e, 0xea, 0x38];
const DEPLOY_INITIAL_LIQUIDITY_DISCRIMINATOR: [u8; 8] = [226, 227, 73, 75, 85, 216, 151, 217];

/// Parameters for entering FeelsSOL
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct EnterFeelssolParams {
    pub amount: u64,
}

impl_instruction!(EnterFeelssolParams, ENTER_FEELSSOL_DISCRIMINATOR);

/// Parameters for exiting FeelsSOL
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ExitFeelssolParams {
    pub amount: u64,
}

impl_instruction!(ExitFeelssolParams, EXIT_FEELSSOL_DISCRIMINATOR);

/// Parameters for initializing a market
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeMarketParams {
    pub base_fee_bps: u16,
    pub tick_spacing: u16,
    pub initial_sqrt_price: u128,
    pub initial_buy_feelssol_amount: u64,
}

impl_instruction!(InitializeMarketParams, INITIALIZE_MARKET_DISCRIMINATOR);

/// Parameters for opening a position
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct OpenPositionParams {
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
}

impl_instruction!(OpenPositionParams, OPEN_POSITION_DISCRIMINATOR);

/// Parameters for closing a position
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ClosePositionParams {
    /// Minimum amount of token 0 to receive
    pub amount_0_min: u64,
    /// Minimum amount of token 1 to receive
    pub amount_1_min: u64,
    /// If true, close the position account after withdrawing liquidity
    pub close_account: bool,
}

impl_instruction!(ClosePositionParams, CLOSE_POSITION_DISCRIMINATOR);

/// Parameters for collecting fees (no parameters needed)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CollectFeesParams {}

impl_instruction!(CollectFeesParams, COLLECT_FEES_DISCRIMINATOR);

/// Parameters for minting tokens
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct MintTokenParams {
    pub ticker: String,
    pub name: String,
    pub uri: String,
}

impl_instruction!(MintTokenParams, MINT_TOKEN_DISCRIMINATOR);

/// Parameters for deploying initial liquidity
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct DeployInitialLiquidityParams {
    pub buy_feelssol_amount: u64,
}

impl_instruction!(
    DeployInitialLiquidityParams,
    DEPLOY_INITIAL_LIQUIDITY_DISCRIMINATOR
);

/// Liquidity instruction builder
pub struct LiquidityInstructionBuilder {
    pda: PdaBuilder,
}

impl LiquidityInstructionBuilder {
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            pda: PdaBuilder::new(program_id),
        }
    }

    /// Build enter FeelsSOL instruction
    pub fn enter_feelssol(
        &self,
        user: Pubkey,
        user_jitosol: Pubkey,
        user_feelssol: Pubkey,
        amount: u64,
    ) -> SdkResult<Instruction> {
        let (feels_hub, _) = self.pda.feels_hub();
        let (feels_mint, _) = self.pda.feels_mint();

        let params = EnterFeelssolParams { amount };

        Ok(FeelsInstructionBuilder::new()
            .add_signer(user)
            .add_readonly(feels_hub)
            .add_writable(feels_mint)
            .add_writable(user_jitosol)
            .add_writable(user_feelssol)
            .add_readonly(spl_token::id())
            .with_data(params.build_data()?)
            .build())
    }

    /// Build exit FeelsSOL instruction
    pub fn exit_feelssol(
        &self,
        user: Pubkey,
        user_jitosol: Pubkey,
        user_feelssol: Pubkey,
        amount: u64,
    ) -> SdkResult<Instruction> {
        let (feels_hub, _) = self.pda.feels_hub();
        let (feels_mint, _) = self.pda.feels_mint();

        let params = ExitFeelssolParams { amount };

        Ok(FeelsInstructionBuilder::new()
            .add_signer(user)
            .add_readonly(feels_hub)
            .add_writable(feels_mint)
            .add_writable(user_feelssol)
            .add_writable(user_jitosol)
            .add_readonly(spl_token::id())
            .with_data(params.build_data()?)
            .build())
    }

    /// Build initialize market instruction
    pub fn initialize_market(
        &self,
        deployer: Pubkey,
        token_0: Pubkey,
        token_1: Pubkey,
        params: InitializeMarketParams,
    ) -> SdkResult<Instruction> {
        let (market, _) = self.pda.market(&token_0, &token_1);
        let (buffer, _) = self.pda.buffer(&market);
        let (vault_authority, _) = self.pda.vault_authority(&market);
        let (oracle, _) = self.pda.oracle(&market);
        let (feels_mint, _) = self.pda.feels_mint();

        Ok(FeelsInstructionBuilder::new()
            .add_signer(deployer)
            .add_writable(market)
            .add_readonly(token_0)
            .add_readonly(token_1)
            .add_readonly(feels_mint)
            .add_writable(buffer)
            .add_readonly(vault_authority)
            .add_writable(oracle)
            .add_readonly(solana_program::system_program::id())
            .add_readonly(spl_token::id())
            .with_data(params.build_data()?)
            .build())
    }

    /// Build open position instruction
    pub fn open_position(
        &self,
        owner: Pubkey,
        market: Pubkey,
        params: OpenPositionParams,
    ) -> SdkResult<Instruction> {
        let (position, _) = self
            .pda
            .position(&owner, params.tick_lower, params.tick_upper);
        let (position_metadata, _) = self.pda.position_metadata(&position);

        // Derive tick arrays for the position range
        let lower_tick_array = self.get_tick_array_for_tick(&market, params.tick_lower);
        let upper_tick_array = self.get_tick_array_for_tick(&market, params.tick_upper);

        Ok(FeelsInstructionBuilder::new()
            .add_signer(owner)
            .add_writable(market)
            .add_writable(position)
            .add_writable(position_metadata)
            .add_writable(lower_tick_array)
            .add_writable(upper_tick_array)
            .add_readonly(solana_program::system_program::id())
            .with_data(params.build_data()?)
            .build())
    }

    /// Build close position instruction
    pub fn close_position(
        &self,
        owner: Pubkey,
        market: Pubkey,
        position: Pubkey,
        tick_lower: i32,
        tick_upper: i32,
        amount_0_min: u64,
        amount_1_min: u64,
        close_account: bool,
    ) -> SdkResult<Instruction> {
        let params = ClosePositionParams {
            amount_0_min,
            amount_1_min,
            close_account,
        };

        // Derive tick arrays for the position range
        let lower_tick_array = self.get_tick_array_for_tick(&market, tick_lower);
        let upper_tick_array = self.get_tick_array_for_tick(&market, tick_upper);

        Ok(FeelsInstructionBuilder::new()
            .add_signer(owner)
            .add_writable(market)
            .add_writable(position)
            .add_writable(lower_tick_array)
            .add_writable(upper_tick_array)
            .with_data(params.build_data()?)
            .build())
    }

    /// Build collect fees instruction
    pub fn collect_fees(
        &self,
        position_owner: Pubkey,
        position: Pubkey,
        position_token_account: Pubkey,
        token_owner_account_0: Pubkey,
        token_owner_account_1: Pubkey,
        market: Pubkey,
        feelssol_mint: Pubkey,
        other_mint: Pubkey,
    ) -> SdkResult<Instruction> {
        let (vault_authority, _) = self.pda.vault_authority(&market);

        // Derive vault addresses using token mints
        let (vault_0, _) = Pubkey::find_program_address(
            &[b"vault", feelssol_mint.as_ref(), other_mint.as_ref(), b"0"],
            &self.pda.program_id,
        );
        let (vault_1, _) = Pubkey::find_program_address(
            &[b"vault", feelssol_mint.as_ref(), other_mint.as_ref(), b"1"],
            &self.pda.program_id,
        );

        Ok(FeelsInstructionBuilder::new()
            .add_signer(position_owner)
            .add_writable(position)
            .add_readonly(position_token_account)
            .add_writable(token_owner_account_0)
            .add_writable(token_owner_account_1)
            .add_readonly(market)
            .add_writable(vault_0)
            .add_writable(vault_1)
            .add_readonly(vault_authority)
            .add_readonly(spl_token::id())
            .with_data(CollectFeesParams {}.build_data()?)
            .build())
    }

    /// Build mint token instruction
    pub fn mint_token(
        &self,
        creator: Pubkey,
        ticker: String,
        name: String,
        uri: String,
    ) -> SdkResult<Instruction> {
        let params = MintTokenParams { ticker, name, uri };

        // This would need more accounts based on actual implementation
        Ok(FeelsInstructionBuilder::new()
            .add_signer(creator)
            .add_readonly(solana_program::system_program::id())
            .add_readonly(spl_token::id())
            .with_data(params.build_data()?)
            .build())
    }

    /// Build deploy initial liquidity instruction  
    pub fn deploy_initial_liquidity(
        &self,
        creator: Pubkey,
        market: Pubkey,
        buy_feelssol_amount: u64,
        feelssol_mint: Pubkey,
        other_mint: Pubkey,
    ) -> SdkResult<Instruction> {
        let params = DeployInitialLiquidityParams {
            buy_feelssol_amount,
        };
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
            .add_writable(vault_0)
            .add_writable(vault_1)
            .add_readonly(vault_authority)
            .add_readonly(spl_token::id())
            .with_data(params.build_data()?)
            .build())
    }

    fn get_tick_array_for_tick(&self, market: &Pubkey, tick: i32) -> Pubkey {
        // Simplified - would need tick spacing to calculate properly
        let start_index =
            (tick / (crate::core::TICK_ARRAY_SIZE * 10)) * (crate::core::TICK_ARRAY_SIZE * 10);
        let (tick_array, _) = self.pda.tick_array(market, start_index);
        tick_array
    }
}

/// Build initialize market instruction
pub fn initialize_market(
    creator: Pubkey,
    token_0: Pubkey,
    token_1: Pubkey,
    feelssol_mint: Pubkey,
    base_fee_bps: u16,
    tick_spacing: u16,
    initial_sqrt_price: u128,
    initial_buy_feelssol_amount: u64,
    creator_feelssol: Option<Pubkey>,
    creator_token_out: Option<Pubkey>,
) -> SdkResult<Instruction> {
    use crate::core::program_id;

    // Validate token order
    if token_0 >= token_1 {
        return Err(SdkError::InvalidParameters(
            "Invalid token order: token_0 must be < token_1".to_string(),
        ));
    }

    // Validate that at least one token is FeelsSOL
    if token_0 != feelssol_mint && token_1 != feelssol_mint {
        return Err(SdkError::InvalidParameters(
            "At least one token must be FeelsSOL".to_string(),
        ));
    }

    // Validate that FeelsSOL is token_0 (required by program)
    if token_0 != feelssol_mint {
        return Err(SdkError::InvalidParameters(
            "FeelsSOL must be token_0 (lower pubkey)".to_string(),
        ));
    }

    let pda = PdaBuilder::new(program_id());

    // Derive PDAs
    let (market, _) = pda.market(&token_0, &token_1);
    let (buffer, _) = pda.buffer(&market);
    let (oracle, _) = pda.oracle(&market);
    let (vault_0, _) = Pubkey::find_program_address(
        &[b"vault", token_0.as_ref(), token_1.as_ref(), b"0"],
        &program_id(),
    );
    let (vault_1, _) = Pubkey::find_program_address(
        &[b"vault", token_0.as_ref(), token_1.as_ref(), b"1"],
        &program_id(),
    );
    let (market_authority, _) =
        Pubkey::find_program_address(&[b"market_authority", market.as_ref()], &program_id());

    // For protocol tokens, derive the expected PDA addresses
    // If token is FeelsSOL, use a dummy PDA to avoid system program
    let protocol_token_0 = if token_0 == feelssol_mint {
        // Create a dummy PDA that's not the system program
        Pubkey::find_program_address(&[b"dummy_protocol_token_0"], &program_id()).0
    } else {
        Pubkey::find_program_address(&[b"protocol_token", token_0.as_ref()], &program_id()).0
    };

    let protocol_token_1 = if token_1 == feelssol_mint {
        // Create a dummy PDA that's not the system program
        Pubkey::find_program_address(&[b"dummy_protocol_token_1"], &program_id()).0
    } else {
        Pubkey::find_program_address(&[b"protocol_token", token_1.as_ref()], &program_id()).0
    };

    // Derive escrow PDA - for the protocol-minted token
    let escrow = if token_0 != feelssol_mint {
        Pubkey::find_program_address(&[b"escrow", token_0.as_ref()], &program_id()).0
    } else {
        Pubkey::find_program_address(&[b"escrow", token_1.as_ref()], &program_id()).0
    };

    let escrow_authority =
        Pubkey::find_program_address(&[b"escrow_authority", escrow.as_ref()], &program_id()).0;

    let params = InitializeMarketParams {
        base_fee_bps,
        tick_spacing,
        initial_sqrt_price,
        initial_buy_feelssol_amount,
    };

    Ok(FeelsInstructionBuilder::new()
        .add_signer(creator) // 0: creator
        .add_writable(token_0) // 1: token_0
        .add_writable(token_1) // 2: token_1
        .add_writable(market) // 3: market
        .add_writable(buffer) // 4: buffer
        .add_writable(oracle) // 5: oracle
        .add_writable(vault_0) // 6: vault_0
        .add_writable(vault_1) // 7: vault_1
        .add_readonly(market_authority) // 8: market_authority
        .add_readonly(feelssol_mint) // 9: feelssol_mint
        .add_readonly(protocol_token_0) // 10: protocol_token_0
        .add_readonly(protocol_token_1) // 11: protocol_token_1
        .add_writable(escrow) // 12: escrow
        .add_readonly(creator_feelssol.unwrap_or_else(|| {
            Pubkey::find_program_address(&[b"dummy_creator_feelssol"], &program_id()).0
        })) // 13: creator_feelssol
        .add_readonly(creator_token_out.unwrap_or_else(|| {
            Pubkey::find_program_address(&[b"dummy_creator_token_out"], &program_id()).0
        })) // 14: creator_token_out
        .add_writable(escrow_authority) // 15: escrow_authority
        .add_readonly(solana_program::system_program::id()) // 16: system_program
        .add_readonly(spl_token::id()) // 17: token_program
        .add_readonly(solana_program::sysvar::rent::id()) // 18: rent
        .with_data(params.build_data()?)
        .build())
}
