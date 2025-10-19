//! Position management and liquidity operations helper

use super::super::*;
use crate::common::sdk_compat;
use feels::state::{Market, Position};
use solana_sdk::instruction::{AccountMeta, Instruction};

/// Helper for position operations
pub struct PositionHelper {
    ctx: TestContext,
}

impl PositionHelper {
    pub fn new(ctx: TestContext) -> Self {
        Self { ctx }
    }

    /// Open a new position
    pub async fn open_position(
        &self,
        market: &Pubkey,
        owner: &Keypair,
        lower_tick: i32,
        upper_tick: i32,
        liquidity: u128,
    ) -> TestResult<Pubkey> {
        // Get market state to find token vaults
        let market_state = self.ctx.get_account::<Market>(market).await?.unwrap();

        // Create position mint and token account
        let position_mint = Keypair::new();
        let position_token_account = self
            .ctx
            .create_ata(&owner.pubkey(), &position_mint.pubkey())
            .await?;

        // Create provider token accounts if they don't exist
        let provider_token_0 = self
            .ctx
            .create_ata(&owner.pubkey(), &market_state.token_0)
            .await?;
        let provider_token_1 = self
            .ctx
            .create_ata(&owner.pubkey(), &market_state.token_1)
            .await?;

        // Derive vaults
        let (vault_0, _) = sdk_compat::find_vault_address(market, &market_state.token_0);
        let (vault_1, _) = sdk_compat::find_vault_address(market, &market_state.token_1);

        // Calculate tick array addresses
        let lower_array_start =
            utils::get_tick_array_start_index(lower_tick, market_state.tick_spacing);
        let upper_array_start =
            utils::get_tick_array_start_index(upper_tick, market_state.tick_spacing);
        let (lower_tick_array, _) = utils::find_tick_array_address(market, lower_array_start);
        let (upper_tick_array, _) = utils::find_tick_array_address(market, upper_array_start);

        // Use SDK to build instruction
        let position_pda = position_mint.pubkey(); // Position PDA would be derived from position mint
        let ix = sdk_compat::instructions::open_position(
            owner.pubkey(),
            *market,
            position_pda,
            lower_tick,
            upper_tick,
            liquidity,
        )?;

        self.ctx
            .process_instruction(ix, &[owner, &position_mint])
            .await?;

        Ok(position_mint.pubkey())
    }

    /// Close a position
    pub async fn close_position(&self, position_mint: &Pubkey, owner: &Keypair) -> TestResult<()> {
        // Get position state
        let (position_pda, _) = sdk_compat::utils::find_position_address(position_mint);
        let position = self
            .ctx
            .get_account::<Position>(&position_pda)
            .await?
            .unwrap();

        // Get market state
        let market_state = self
            .ctx
            .get_account::<Market>(&position.market)
            .await?
            .unwrap();

        // Get owner's position token account
        let position_token_account = spl_associated_token_account::get_associated_token_address(
            &owner.pubkey(),
            position_mint,
        );

        // Create owner token accounts if they don't exist
        let owner_token_0 = self
            .ctx
            .create_ata(&owner.pubkey(), &market_state.token_0)
            .await?;
        let owner_token_1 = self
            .ctx
            .create_ata(&owner.pubkey(), &market_state.token_1)
            .await?;

        // Derive vaults
        let (vault_0, _) = sdk_compat::find_vault_address(&position.market, &market_state.token_0);
        let (vault_1, _) = sdk_compat::find_vault_address(&position.market, &market_state.token_1);

        // Calculate tick array addresses
        let lower_array_start =
            utils::get_tick_array_start_index(position.tick_lower, market_state.tick_spacing);
        let upper_array_start =
            utils::get_tick_array_start_index(position.tick_upper, market_state.tick_spacing);
        let (lower_tick_array, _) =
            utils::find_tick_array_address(&position.market, lower_array_start);
        let (upper_tick_array, _) =
            utils::find_tick_array_address(&position.market, upper_array_start);

        // Build close position params
        let params = sdk_compat::instructions::ClosePositionParams {
            amount_0_min: 0,
            amount_1_min: 0,
            close_account: true,
        };

        // Use SDK to build instruction
        let ix = sdk_compat::instructions::close_position(
            owner.pubkey(),
            position.market,
            *position_mint,
            params,
        )?;

        self.ctx.process_instruction(ix, &[owner]).await?;

        Ok(())
    }

    /// Open a position with metadata
    pub async fn open_position_with_metadata(
        &self,
        market: &Pubkey,
        owner: &Keypair,
        lower_tick: i32,
        upper_tick: i32,
        liquidity: u128,
    ) -> TestResult<PositionInfo> {
        // Get market state to find token vaults
        let market_state = self.ctx.get_account::<Market>(market).await?.unwrap();

        // Create position mint
        let position_mint = Keypair::new();
        let position_token_account = self
            .ctx
            .create_ata(&owner.pubkey(), &position_mint.pubkey())
            .await?;

        // Create provider token accounts if they don't exist
        let provider_token_0 = self
            .ctx
            .create_ata(&owner.pubkey(), &market_state.token_0)
            .await?;
        let provider_token_1 = self
            .ctx
            .create_ata(&owner.pubkey(), &market_state.token_1)
            .await?;

        // Derive position PDA
        let (position_pda, _) = sdk_compat::utils::find_position_address(&position_mint.pubkey());

        // Derive vaults
        let (vault_0, _) = sdk_compat::find_vault_address(market, &market_state.token_0);
        let (vault_1, _) = sdk_compat::find_vault_address(market, &market_state.token_1);

        // Calculate tick array addresses
        let lower_array_start =
            utils::get_tick_array_start_index(lower_tick, market_state.tick_spacing);
        let upper_array_start =
            utils::get_tick_array_start_index(upper_tick, market_state.tick_spacing);
        let (lower_tick_array, _) = utils::find_tick_array_address(market, lower_array_start);
        let (upper_tick_array, _) = utils::find_tick_array_address(market, upper_array_start);

        // Derive metadata account
        let position_mint_pubkey = position_mint.pubkey();
        let metadata_seeds = &[
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            position_mint_pubkey.as_ref(),
        ];
        let (metadata_account, _) =
            Pubkey::find_program_address(metadata_seeds, &mpl_token_metadata::ID);

        // Build instruction manually since SDK doesn't have this yet
        let discriminator: [u8; 8] =
            anchor_lang::solana_program::hash::hash(b"global:open_position_with_metadata")
                .to_bytes()[..8]
                .try_into()
                .unwrap();
        let mut data: Vec<u8> = discriminator.to_vec();
        data.extend_from_slice(&lower_tick.to_le_bytes());
        data.extend_from_slice(&upper_tick.to_le_bytes());
        data.extend_from_slice(&liquidity.to_le_bytes());

        let accounts = vec![
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new(*market, false),
            AccountMeta::new(position_mint.pubkey(), true),
            AccountMeta::new(position_token_account, false),
            AccountMeta::new(position_pda, false),
            AccountMeta::new(provider_token_0, false),
            AccountMeta::new(provider_token_1, false),
            AccountMeta::new(vault_0, false),
            AccountMeta::new(vault_1, false),
            AccountMeta::new(lower_tick_array, false),
            AccountMeta::new(upper_tick_array, false),
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(mpl_token_metadata::ID, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
        ];

        let ix = Instruction {
            program_id: sdk_compat::program_id(),
            accounts,
            data,
        };

        self.ctx
            .process_instruction(ix, &[owner, &position_mint])
            .await?;

        Ok(PositionInfo {
            pubkey: position_pda,
            mint: position_mint.pubkey(),
            token_account: position_token_account,
        })
    }

    /// Close a position with metadata
    pub async fn close_position_with_metadata(
        &self,
        position_info: &PositionInfo,
        owner: &Keypair,
    ) -> TestResult<()> {
        // Get position state
        let position = self
            .ctx
            .get_account::<Position>(&position_info.pubkey)
            .await?
            .unwrap();

        // Get market state
        let market_state = self
            .ctx
            .get_account::<Market>(&position.market)
            .await?
            .unwrap();

        // Create owner token accounts if they don't exist
        let owner_token_0 = self
            .ctx
            .create_ata(&owner.pubkey(), &market_state.token_0)
            .await?;
        let owner_token_1 = self
            .ctx
            .create_ata(&owner.pubkey(), &market_state.token_1)
            .await?;

        // Derive vaults
        let (vault_0, _) = sdk_compat::find_vault_address(&position.market, &market_state.token_0);
        let (vault_1, _) = sdk_compat::find_vault_address(&position.market, &market_state.token_1);

        // Calculate tick array addresses
        let lower_array_start =
            utils::get_tick_array_start_index(position.tick_lower, market_state.tick_spacing);
        let upper_array_start =
            utils::get_tick_array_start_index(position.tick_upper, market_state.tick_spacing);
        let (lower_tick_array, _) =
            utils::find_tick_array_address(&position.market, lower_array_start);
        let (upper_tick_array, _) =
            utils::find_tick_array_address(&position.market, upper_array_start);

        // Derive metadata account
        let metadata_seeds = &[
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            position_info.mint.as_ref(),
        ];
        let (metadata_account, _) =
            Pubkey::find_program_address(metadata_seeds, &mpl_token_metadata::ID);

        // Build instruction manually since SDK doesn't have this yet
        let discriminator: [u8; 8] =
            anchor_lang::solana_program::hash::hash(b"global:close_position_with_metadata")
                .to_bytes()[..8]
                .try_into()
                .unwrap();
        let mut data: Vec<u8> = discriminator.to_vec();
        data.extend_from_slice(&0u64.to_le_bytes()); // amount_0_min
        data.extend_from_slice(&0u64.to_le_bytes()); // amount_1_min

        let accounts = vec![
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new(position.market, false),
            AccountMeta::new(position_info.mint, false),
            AccountMeta::new(position_info.token_account, false),
            AccountMeta::new(position_info.pubkey, false),
            AccountMeta::new(owner_token_0, false),
            AccountMeta::new(owner_token_1, false),
            AccountMeta::new(vault_0, false),
            AccountMeta::new(vault_1, false),
            AccountMeta::new(lower_tick_array, false),
            AccountMeta::new(upper_tick_array, false),
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ];

        let ix = Instruction {
            program_id: sdk_compat::program_id(),
            accounts,
            data,
        };

        self.ctx.process_instruction(ix, &[owner]).await?;

        Ok(())
    }

    /// Get position state
    pub async fn get_position(&self, position_id: &Pubkey) -> TestResult<Option<Position>> {
        self.ctx.get_account(position_id).await
    }

    /// Collect fees from a position
    pub async fn collect_fees(
        &self,
        position_mint: &Pubkey,
        owner: &Keypair,
    ) -> TestResult<CollectFeesResult> {
        // Get position state
        let (position_pda, _) = sdk_compat::utils::find_position_address(position_mint);
        let position = self
            .ctx
            .get_account::<Position>(&position_pda)
            .await?
            .unwrap();

        // Get market state
        let market_state = self
            .ctx
            .get_account::<Market>(&position.market)
            .await?
            .unwrap();

        // Get owner's position token account
        let position_token_account = spl_associated_token_account::get_associated_token_address(
            &owner.pubkey(),
            position_mint,
        );

        // Create owner token accounts if they don't exist
        let owner_token_0 = self
            .ctx
            .create_ata(&owner.pubkey(), &market_state.token_0)
            .await?;
        let owner_token_1 = self
            .ctx
            .create_ata(&owner.pubkey(), &market_state.token_1)
            .await?;

        // Derive vaults
        let (vault_0, _) = sdk_compat::find_vault_address(&position.market, &market_state.token_0);
        let (vault_1, _) = sdk_compat::find_vault_address(&position.market, &market_state.token_1);

        // Calculate tick array addresses for fee calculation
        let lower_array_start =
            utils::get_tick_array_start_index(position.tick_lower, market_state.tick_spacing);
        let upper_array_start =
            utils::get_tick_array_start_index(position.tick_upper, market_state.tick_spacing);
        let (lower_tick_array, _) =
            utils::find_tick_array_address(&position.market, lower_array_start);
        let (upper_tick_array, _) =
            utils::find_tick_array_address(&position.market, upper_array_start);

        // Get initial balances
        let initial_balance_0 = self.ctx.get_token_balance(&owner_token_0).await?;
        let initial_balance_1 = self.ctx.get_token_balance(&owner_token_1).await?;

        // Use SDK to build instruction with tick arrays for fee calculation
        let ix = sdk_compat::instructions::collect_fees(
            owner.pubkey(),
            *position_mint,
            position_token_account,
            owner_token_0,
        )?;

        self.ctx.process_instruction(ix, &[owner]).await?;

        // Get final balances
        let final_balance_0 = self.ctx.get_token_balance(&owner_token_0).await?;
        let final_balance_1 = self.ctx.get_token_balance(&owner_token_1).await?;

        Ok(CollectFeesResult {
            fee_a_collected: final_balance_0 - initial_balance_0,
            fee_b_collected: final_balance_1 - initial_balance_1,
        })
    }

    /// Add liquidity to existing position
    pub async fn add_liquidity(
        &self,
        _position_id: &Pubkey,
        _owner: &Keypair,
        _liquidity_delta: u128,
    ) -> TestResult<()> {
        // The Feels protocol currently does not support modifying liquidity in existing positions.
        // This is a deliberate design choice to keep the protocol simpler and more efficient.
        //
        // To add liquidity at the same price range:
        // 1. Open a new position with the additional liquidity
        // 2. Keep track of multiple position NFTs
        //
        // This approach:
        // - Simplifies the protocol implementation
        // - Makes fee accounting cleaner (each position tracks its own fees)
        // - Allows for more granular position management
        // - Enables easier position transfers (NFT-based)

        Err("Adding liquidity to existing positions is not supported. Please open a new position instead.".into())
    }

    /// Remove liquidity from position
    pub async fn remove_liquidity(
        &self,
        _position_id: &Pubkey,
        _owner: &Keypair,
        _liquidity_delta: u128,
    ) -> TestResult<()> {
        // The Feels protocol currently does not support partial liquidity removal.
        // Positions must be closed entirely to remove liquidity.
        //
        // To partially remove liquidity:
        // 1. Close the entire position
        // 2. Open a new position with the remaining liquidity
        //
        // This design choice:
        // - Keeps the protocol simpler and more gas-efficient
        // - Avoids complex partial fee calculations
        // - Maintains clear position lifecycle (open -> collect fees -> close)
        //
        // For testing partial liquidity scenarios, use multiple smaller positions
        // instead of one large position.

        Err("Partial liquidity removal is not supported. Please close the entire position and open a new one with desired liquidity.".into())
    }
}