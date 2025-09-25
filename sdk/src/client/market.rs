use std::sync::Arc;

use crate::prelude::*;
use solana_sdk::{account::Account, instruction::Instruction};

use crate::{
    client::BaseClient,
    core::{MarketInfo, SdkError, SdkResult},
    instructions::MarketInstructionBuilder,
    protocol::PdaBuilder,
};

/// Service for market-related operations
pub struct MarketService {
    base: Arc<BaseClient>,
    pda: Arc<PdaBuilder>,
    builder: MarketInstructionBuilder,
}

impl MarketService {
    pub fn new(base: Arc<BaseClient>, pda: Arc<PdaBuilder>) -> Self {
        Self { 
            builder: MarketInstructionBuilder::new(pda.program_id),
            base,
            pda,
        }
    }

    /// Get market info by tokens
    pub async fn get_market_by_tokens(
        &self,
        token_0: &Pubkey,
        token_1: &Pubkey,
    ) -> SdkResult<MarketInfo> {
        // Ensure proper token ordering
        let (ordered_token_0, ordered_token_1) = if token_0 < token_1 {
            (token_0, token_1)
        } else {
            (token_1, token_0)
        };

        let (market_address, _) = self.pda.market(ordered_token_0, ordered_token_1);
        self.get_market(&market_address).await
    }

    /// Get market info by address
    pub async fn get_market(&self, market_address: &Pubkey) -> SdkResult<MarketInfo> {
        let account = self.base.get_account(market_address).await?;
        
        // Parse market account (simplified - would need actual deserialization)
        self.parse_market_account(&account, market_address)
    }

    /// Get all markets (paginated)
    pub async fn get_all_markets(&self, _page: u32, _page_size: u32) -> SdkResult<Vec<MarketInfo>> {
        // In a real implementation, this would use getProgramAccounts with filters
        // For now, return empty vec
        Ok(Vec::new())
    }

    /// Find best market for a token pair
    pub async fn find_best_market(
        &self,
        token_a: &Pubkey,
        token_b: &Pubkey,
    ) -> SdkResult<MarketInfo> {
        // In hub-and-spoke model, there's only one possible market per pair
        self.get_market_by_tokens(token_a, token_b).await
    }

    /// Check if a market exists
    pub async fn market_exists(&self, token_0: &Pubkey, token_1: &Pubkey) -> bool {
        let (market_address, _) = self.pda.market(token_0, token_1);
        self.base.get_account(&market_address).await.is_ok()
    }

    /// Get market oracle data
    pub async fn get_market_oracle(&self, market: &Pubkey) -> SdkResult<OracleData> {
        let (oracle_address, _) = self.pda.oracle(market);
        let account = self.base.get_account(&oracle_address).await?;
        
        // Parse oracle account
        self.parse_oracle_account(&account)
    }

    /// Get market buffer data
    pub async fn get_market_buffer(&self, market: &Pubkey) -> SdkResult<BufferData> {
        let (buffer_address, _) = self.pda.buffer(market);
        let account = self.base.get_account(&buffer_address).await?;
        
        // Parse buffer account
        self.parse_buffer_account(&account)
    }

    // Helper methods for parsing accounts
    fn parse_market_account(&self, account: &Account, address: &Pubkey) -> SdkResult<MarketInfo> {
        // Simplified parsing - would need actual struct deserialization
        if account.data.len() < 200 {
            return Err(SdkError::InvalidParameters(
                "Invalid market account data".to_string(),
            ));
        }

        // Mock data for now
        Ok(MarketInfo {
            address: *address,
            token_0: Pubkey::default(),
            token_1: Pubkey::default(),
            sqrt_price: 18446744073709551616, // sqrt price at tick 0
            liquidity: 0,
            current_tick: 0,
            base_fee_bps: 30,
            tick_spacing: 10,
            is_paused: false,
        })
    }

    fn parse_oracle_account(&self, _account: &Account) -> SdkResult<OracleData> {
        Ok(OracleData {
            last_update_slot: 0,
            observations: Vec::new(),
        })
    }

    fn parse_buffer_account(&self, _account: &Account) -> SdkResult<BufferData> {
        Ok(BufferData {
            collected_fees_0: 0,
            collected_fees_1: 0,
            pomm_liquidity: 0,
        })
    }

    // Market management instructions

    /// Build transition market phase instruction
    pub fn transition_market_phase_ix(
        &self,
        authority: Pubkey,
        market: Pubkey,
        new_phase: u8,
    ) -> SdkResult<Instruction> {
        self.builder
            .transition_market_phase(authority, market, new_phase)
    }

    /// Build graduate pool instruction
    pub fn graduate_pool_ix(
        &self,
        creator: Pubkey,
        market: Pubkey,
        target_pool: Pubkey,
        feelssol_mint: Pubkey,
        other_mint: Pubkey,
    ) -> SdkResult<Instruction> {
        self.builder
            .graduate_pool(creator, market, target_pool, feelssol_mint, other_mint)
    }

    /// Build cleanup bonding curve instruction
    pub fn cleanup_bonding_curve_ix(
        &self,
        authority: Pubkey,
        market: Pubkey,
        feelssol_mint: Pubkey,
        other_mint: Pubkey,
    ) -> SdkResult<Instruction> {
        self.builder
            .cleanup_bonding_curve(authority, market, feelssol_mint, other_mint)
    }

    /// Build destroy expired token instruction
    pub fn destroy_expired_token_ix(
        &self,
        anyone: Pubkey,
        market: Pubkey,
        expired_mint: Pubkey,
        refund_recipient: Pubkey,
    ) -> SdkResult<Instruction> {
        self.builder
            .destroy_expired_token(anyone, market, expired_mint, refund_recipient)
    }

    /// Build initialize tranche ticks instruction
    pub fn initialize_tranche_ticks_ix(
        &self,
        payer: Pubkey,
        market: Pubkey,
        start_tick_index: i32,
    ) -> SdkResult<Instruction> {
        self.builder
            .initialize_tranche_ticks(payer, market, start_tick_index)
    }

    /// Build update DEX TWAP instruction
    pub fn update_dex_twap_ix(
        &self,
        market: Pubkey,
        target_dex_pool: Pubkey,
    ) -> SdkResult<Instruction> {
        self.builder.update_dex_twap(market, target_dex_pool)
    }
}

/// Oracle data for a market
#[derive(Debug, Clone)]
pub struct OracleData {
    pub last_update_slot: u64,
    pub observations: Vec<(u64, u128)>, // (slot, sqrt_price)
}

/// Buffer data for a market
#[derive(Debug, Clone)]
pub struct BufferData {
    pub collected_fees_0: u64,
    pub collected_fees_1: u64,
    pub pomm_liquidity: u128,
}