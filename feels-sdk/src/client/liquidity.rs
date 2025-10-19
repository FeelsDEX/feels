use std::sync::Arc;

use crate::prelude::*;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signature},
    signer::Signer,
};

use crate::{
    client::BaseClient,
    core::{PositionInfo, SdkResult},
    instructions::{InitializeMarketParams, LiquidityInstructionBuilder, OpenPositionParams},
    protocol::PdaBuilder,
};

/// Service for liquidity management operations
pub struct LiquidityService {
    base: Arc<BaseClient>,
    pda: Arc<PdaBuilder>,
    liquidity_builder: LiquidityInstructionBuilder,
}

impl LiquidityService {
    pub fn new(base: Arc<BaseClient>, pda: Arc<PdaBuilder>, program_id: Pubkey) -> Self {
        Self {
            base,
            pda,
            liquidity_builder: LiquidityInstructionBuilder::new(program_id),
        }
    }

    /// Enter FeelsSOL by converting JitoSOL
    pub async fn enter_feelssol(
        &self,
        signer: &Keypair,
        user_jitosol: Pubkey,
        user_feelssol: Pubkey,
        amount: u64,
    ) -> SdkResult<Signature> {
        let ix = self.liquidity_builder.enter_feelssol(
            signer.pubkey(),
            user_jitosol,
            user_feelssol,
            amount,
        )?;

        self.base.send_transaction(&[ix], &[signer]).await
    }

    /// Exit FeelsSOL to receive JitoSOL
    pub async fn exit_feelssol(
        &self,
        signer: &Keypair,
        user_jitosol: Pubkey,
        user_feelssol: Pubkey,
        amount: u64,
    ) -> SdkResult<Signature> {
        let ix = self.liquidity_builder.exit_feelssol(
            signer.pubkey(),
            user_jitosol,
            user_feelssol,
            amount,
        )?;

        self.base.send_transaction(&[ix], &[signer]).await
    }

    /// Initialize a new market
    pub async fn initialize_market(
        &self,
        deployer: &Keypair,
        token_0: Pubkey,
        token_1: Pubkey,
        base_fee_bps: u16,
        tick_spacing: u16,
        initial_sqrt_price: u128,
        initial_buy_feelssol_amount: u64,
    ) -> SdkResult<InitializeMarketResult> {
        let params = InitializeMarketParams {
            base_fee_bps,
            tick_spacing,
            initial_sqrt_price,
            initial_buy_feelssol_amount,
        };

        let ix = self.liquidity_builder.initialize_market(
            deployer.pubkey(),
            token_0,
            token_1,
            params,
        )?;

        let signature = self.base.send_transaction(&[ix], &[deployer]).await?;
        let (market, _) = self.pda.market(&token_0, &token_1);

        Ok(InitializeMarketResult { signature, market })
    }

    /// Open a new liquidity position
    pub async fn open_position(
        &self,
        owner: &Keypair,
        market: Pubkey,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
    ) -> SdkResult<OpenPositionResult> {
        let params = OpenPositionParams {
            tick_lower,
            tick_upper,
            liquidity,
        };

        let ix = self
            .liquidity_builder
            .open_position(owner.pubkey(), market, params)?;

        let signature = self.base.send_transaction(&[ix], &[owner]).await?;
        let (position, _) = self.pda.position(&owner.pubkey(), tick_lower, tick_upper);

        Ok(OpenPositionResult {
            signature,
            position,
        })
    }

    /// Close a liquidity position
    pub async fn close_position(
        &self,
        owner: &Keypair,
        market: Pubkey,
        position: Pubkey,
        tick_lower: i32,
        tick_upper: i32,
        amount_0_min: u64,
        amount_1_min: u64,
        close_account: bool,
    ) -> SdkResult<Signature> {
        let ix = self.liquidity_builder.close_position(
            owner.pubkey(),
            market,
            position,
            tick_lower,
            tick_upper,
            amount_0_min,
            amount_1_min,
            close_account,
        )?;

        self.base.send_transaction(&[ix], &[owner]).await
    }

    /// Get position info
    pub async fn get_position(
        &self,
        owner: &Pubkey,
        tick_lower: i32,
        tick_upper: i32,
    ) -> SdkResult<PositionInfo> {
        let (position_address, _) = self.pda.position(owner, tick_lower, tick_upper);
        let account = self.base.get_account(&position_address).await?;

        self.parse_position_account(&account, owner)
    }

    /// Get all positions for an owner
    pub async fn get_positions_by_owner(&self, _owner: &Pubkey) -> SdkResult<Vec<PositionInfo>> {
        // Would use getProgramAccounts with filters in real implementation
        Ok(Vec::new())
    }

    /// Collect fees from a position
    pub async fn collect_fees(
        &self,
        _owner: &Keypair,
        _position: Pubkey,
    ) -> SdkResult<CollectFeesResult> {
        // Simplified - would build actual collect fees instruction
        Ok(CollectFeesResult {
            signature: Signature::default(),
            fees_0: 0,
            fees_1: 0,
        })
    }

    // Helper methods
    fn parse_position_account(
        &self,
        _account: &Account,
        owner: &Pubkey,
    ) -> SdkResult<PositionInfo> {
        // Simplified parsing
        Ok(PositionInfo {
            owner: *owner,
            liquidity: 0,
            tick_lower: -887220,
            tick_upper: 887220,
            fee_growth_inside_0: 0,
            fee_growth_inside_1: 0,
            tokens_owed_0: 0,
            tokens_owed_1: 0,
        })
    }
}

/// Result of initializing a market
#[derive(Debug, Clone)]
pub struct InitializeMarketResult {
    pub signature: Signature,
    pub market: Pubkey,
}

/// Result of opening a position
#[derive(Debug, Clone)]
pub struct OpenPositionResult {
    pub signature: Signature,
    pub position: Pubkey,
}

/// Result of collecting fees
#[derive(Debug, Clone)]
pub struct CollectFeesResult {
    pub signature: Signature,
    pub fees_0: u64,
    pub fees_1: u64,
}
