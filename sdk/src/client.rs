use anchor_lang::prelude::*;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use std::sync::Arc;

use crate::{
    config::SdkConfig,
    errors::{SdkError, SdkResult},
    instructions,
    types::{AddLiquidityResult, CreatePoolResult, PoolInfo, PositionInfo, SwapResult},
};

/// Main SDK client for interacting with the Feels Protocol
pub struct FeelsClient {
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
    payer: Arc<Keypair>,
}

impl FeelsClient {
    /// Create a new Feels client
    pub fn new(config: SdkConfig) -> Self {
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        ));

        Self {
            rpc_client,
            program_id: config.program_id,
            payer: config.payer,
        }
    }

    /// Initialize the protocol
    pub async fn initialize_protocol(
        &self,
        protocol_state: &Pubkey,
        authority: &Pubkey,
        treasury: &Pubkey,
    ) -> SdkResult<Signature> {
        let ix = instructions::initialize_protocol(
            &self.program_id,
            protocol_state,
            authority,
            treasury,
        );

        self.send_transaction(&[ix]).await
    }

    /// Initialize FeelsSOL
    pub async fn initialize_feelssol(
        &self,
        feelssol: &Pubkey,
        feels_mint: &Pubkey,
        authority: &Pubkey,
        underlying_mint: &Pubkey,
    ) -> SdkResult<Signature> {
        let ix = instructions::initialize_feelssol(
            &self.program_id,
            feelssol,
            feels_mint,
            authority,
            underlying_mint,
        );

        self.send_transaction(&[ix]).await
    }

    /// Create a new pool
    #[allow(clippy::too_many_arguments)]
    pub async fn create_pool(
        &self,
        pool: &Pubkey,
        token_a_mint: &Pubkey,
        token_b_mint: &Pubkey,
        feelssol: &Pubkey,
        token_a_vault: &Pubkey,
        token_b_vault: &Pubkey,
        protocol_state: &Pubkey,
        authority: &Pubkey,
        fee_rate: u16,
        initial_sqrt_price: u128,
        base_rate: u16,
        protocol_share: u16,
    ) -> SdkResult<CreatePoolResult> {
        let ix = instructions::initialize_pool(
            &self.program_id,
            pool,
            token_a_mint,
            token_b_mint,
            feelssol,
            token_a_vault,
            token_b_vault,
            protocol_state,
            authority,
            fee_rate,
            initial_sqrt_price,
            base_rate,
            protocol_share,
        );

        let signature = self.send_transaction(&[ix]).await?;

        Ok(CreatePoolResult {
            pool_pubkey: *pool,
            vault_a: *token_a_vault,
            vault_b: *token_b_vault,
            signature,
        })
    }

    /// Add liquidity to a pool
    #[allow(clippy::too_many_arguments)]
    pub async fn add_liquidity(
        &self,
        pool: &Pubkey,
        user: &Pubkey,
        user_token_0: &Pubkey,
        user_token_1: &Pubkey,
        pool_token_0: &Pubkey,
        pool_token_1: &Pubkey,
        tick_lower: i32,
        tick_upper: i32,
        liquidity_amount: u128,
        leverage: Option<u64>,
        amount_0_max: u64,
        amount_1_max: u64,
    ) -> SdkResult<AddLiquidityResult> {
        let ix = instructions::add_liquidity(
            &self.program_id,
            pool,
            user,
            user_token_0,
            user_token_1,
            pool_token_0,
            pool_token_1,
            tick_lower,
            tick_upper,
            liquidity_amount,
            leverage,
            amount_0_max,
            amount_1_max,
        );

        let signature = self.send_transaction(&[ix]).await?;

        Ok(AddLiquidityResult {
            position_pubkey: Pubkey::default(), // In unified system, positions tracked differently
            position_mint: Pubkey::default(), // In production, get from position metadata
            liquidity_amount,
            amount_0: amount_0_max, // In production, get actual amounts from logs
            amount_1: amount_1_max,
            signature,
        })
    }

    /// Execute a swap
    #[allow(clippy::too_many_arguments)]
    pub async fn swap(
        &self,
        pool: &Pubkey,
        _oracle_state: &Pubkey, // No longer used - oracle accessed via remaining_accounts
        user: &Pubkey,
        user_token_a: &Pubkey,
        user_token_b: &Pubkey,
        pool_token_a: &Pubkey,
        pool_token_b: &Pubkey,
        amount_in: u64,
        amount_out_minimum: u64,
        sqrt_price_limit: u128,
        is_token_0_to_1: bool,
    ) -> SdkResult<SwapResult> {
        let ix = instructions::swap_execute(
            &self.program_id,
            pool,
            user,
            user_token_a,
            user_token_b,
            pool_token_a,
            pool_token_b,
            amount_in,
            amount_out_minimum,
            sqrt_price_limit,
            is_token_0_to_1,
        );

        let signature = self.send_transaction(&[ix]).await?;

        Ok(SwapResult {
            amount_in,
            amount_out: 0,  // In production, get from transaction logs
            fee_amount: 0,  // In production, get from transaction logs
            price_after: 0, // In production, get from transaction logs
            signature,
        })
    }

    /// Get pool information
    pub async fn get_pool_info(&self, pool_address: &Pubkey) -> SdkResult<PoolInfo> {
        let _account = self
            .rpc_client
            .get_account(pool_address)
            .map_err(|e| SdkError::RpcError(e.to_string()))?;

        // In production, deserialize the pool account data
        Ok(PoolInfo {
            pubkey: *pool_address,
            token_a_mint: Pubkey::default(),
            token_b_mint: Pubkey::default(),
            token_a_vault: Pubkey::default(),
            token_b_vault: Pubkey::default(),
            fee_rate: 0,
            protocol_fee_rate: 0,
            liquidity: 0,
            sqrt_price: 0,
            current_tick: 0,
            tick_spacing: 0,
        })
    }

    /// Get position information
    pub async fn get_position_info(&self, position_address: &Pubkey) -> SdkResult<PositionInfo> {
        let _account = self
            .rpc_client
            .get_account(position_address)
            .map_err(|e| SdkError::RpcError(e.to_string()))?;

        // In production, deserialize the position account data
        Ok(PositionInfo {
            pubkey: *position_address,
            mint: Pubkey::default(),
            pool: Pubkey::default(),
            owner: Pubkey::default(),
            liquidity: 0,
            tick_lower: 0,
            tick_upper: 0,
            fee_growth_0_checkpoint: 0,
            fee_growth_1_checkpoint: 0,
            tokens_owed_0: 0,
            tokens_owed_1: 0,
        })
    }

    /// Send a transaction
    async fn send_transaction(
        &self,
        instructions: &[solana_sdk::instruction::Instruction],
    ) -> SdkResult<Signature> {
        let recent_blockhash = self
            .rpc_client
            .get_latest_blockhash()
            .map_err(|e| SdkError::RpcError(e.to_string()))?;

        let transaction = Transaction::new_signed_with_payer(
            instructions,
            Some(&self.payer.pubkey()),
            &[&*self.payer],
            recent_blockhash,
        );

        self.rpc_client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| SdkError::TransactionFailed(e.to_string()))
    }
}
