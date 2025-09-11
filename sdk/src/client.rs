//! Feels SDK client

use crate::{
    config::SdkConfig,
    error::{SdkError, SdkResult},
    types::{MarketInfo, BufferInfo, SwapQuote, Route},
    instructions,
};
use anchor_client::{Client, Cluster, Program};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use std::sync::Arc;
use std::str::FromStr;
use feels::state::{Market, Buffer};

/// Main SDK client for interacting with Feels protocol
pub struct FeelsClient {
    /// Anchor client
    pub client: Client<Arc<Keypair>>,
    
    /// Anchor program handle
    pub program: Program<Arc<Keypair>>,
    
    /// SDK configuration
    pub config: SdkConfig,
}

impl FeelsClient {
    /// Create a new client instance
    pub fn new(config: SdkConfig) -> SdkResult<Self> {
        let cluster = Cluster::Custom(config.rpc_url.clone(), config.ws_url.clone());
        let client = Client::new_with_options(
            cluster,
            config.payer.clone(),
            CommitmentConfig::from_str(&config.commitment).unwrap(),
        );
        
        let program = client.program(config.program_id)?;
        
        Ok(Self {
            client,
            program,
            config,
        })
    }
    
    /// Get market info
    pub fn get_market_info(&self, market: &Pubkey) -> SdkResult<MarketInfo> {
        let account: Market = self.program.account(*market)
            .map_err(|e| SdkError::AccountNotFound(format!("Market {}: {}", market, e)))?;
        
        Ok(MarketInfo {
            address: *market,
            token_0: account.token_0,
            token_1: account.token_1,
            sqrt_price: account.sqrt_price,
            liquidity: account.liquidity,
            base_fee_bps: account.base_fee_bps,
            is_paused: account.is_paused,
        })
    }
    
    /// Get buffer info  
    pub fn get_buffer_info(&self, buffer: &Pubkey) -> SdkResult<BufferInfo> {
        let account: Buffer = self.program.account(*buffer)
            .map_err(|e| SdkError::AccountNotFound(format!("Buffer {}: {}", buffer, e)))?;
        
        Ok(BufferInfo {
            address: *buffer,
            market: account.market,
            tau_spot: account.tau_spot,
            tau_time: account.tau_time,
            tau_leverage: account.tau_leverage,
            fees_token_0: account.fees_token_0,
            fees_token_1: account.fees_token_1,
            floor_threshold: account.floor_placement_threshold,
        })
    }
    
    /// Quote a swap
    pub fn quote_swap(
        &self,
        from_mint: &Pubkey,
        to_mint: &Pubkey,
        amount_in: u64,
    ) -> SdkResult<SwapQuote> {
        // Determine route
        let route = self.find_route(from_mint, to_mint)?;
        
        // For MVP, just estimate with fixed fee
        let fee_bps = 30; // 0.3%
        let fee_amount = (amount_in as u128 * fee_bps as u128 / 10_000) as u64;
        let amount_out = amount_in - fee_amount; // Simplified calculation
        
        Ok(SwapQuote {
            amount_in,
            amount_out,
            fee_amount,
            fee_bps,
            price_impact_bps: 0, // Simplified for now
            route,
        })
    }
    
    /// Execute a swap
    pub fn swap(
        &self,
        user_token_in: &Pubkey,
        user_token_out: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        tick_arrays: Vec<Pubkey>,
    ) -> SdkResult<Signature> {
        // Get market from tokens (simplified - assumes direct market exists)
        let market = self.find_market(user_token_in, user_token_out)?;
        
        let ix = instructions::swap(
            self.config.payer.pubkey(),
            market,
            *user_token_in,
            *user_token_out,
            Pubkey::default(), // token_0_mint - would need to fetch market to get
            Pubkey::default(), // token_1_mint - would need to fetch market to get
            tick_arrays,
            amount_in,
            minimum_amount_out,
            0, // max_ticks_crossed
        )?;
        
        let sig = self.program.request()
            .instruction(ix)
            .send()?;
        
        Ok(sig)
    }
    
    /// Enter FeelsSOL
    pub fn enter_feelssol(
        &self,
        user_jitosol: &Pubkey,
        user_feelssol: &Pubkey,
        jitosol_mint: &Pubkey,
        feelssol_mint: &Pubkey,
        amount: u64,
    ) -> SdkResult<Signature> {
        let ix = instructions::enter_feelssol(
            self.config.payer.pubkey(),
            *user_jitosol,
            *user_feelssol,
            *jitosol_mint,
            *feelssol_mint,
            amount,
        );
        
        let sig = self.program.request()
            .instruction(ix)
            .send()?;
        
        Ok(sig)
    }
    
    /// Exit FeelsSOL
    pub fn exit_feelssol(
        &self,
        user_feelssol: &Pubkey,
        user_jitosol: &Pubkey,
        feelssol_mint: &Pubkey,
        jitosol_mint: &Pubkey,
        amount: u64,
    ) -> SdkResult<Signature> {
        let ix = instructions::exit_feelssol(
            self.config.payer.pubkey(),
            *user_feelssol,
            *user_jitosol,
            *feelssol_mint,
            *jitosol_mint,
            amount,
        );
        
        let sig = self.program.request()
            .instruction(ix)
            .send()?;
        
        Ok(sig)
    }
    
    /// Find route between two tokens
    fn find_route(&self, from: &Pubkey, to: &Pubkey) -> SdkResult<Route> {
        // For MVP, assume FeelsSOL is the hub
        // This is simplified - would need to check if direct market exists
        let feelssol_mint = Pubkey::default(); // Would need actual FeelsSOL mint
        
        if from == to {
            return Err(SdkError::InvalidRoute("Cannot swap same token".to_string()));
        }
        
        // Check if one of them is FeelsSOL
        if from == &feelssol_mint || to == &feelssol_mint {
            Ok(Route::Direct {
                from: *from,
                to: *to,
            })
        } else {
            // Two-hop through FeelsSOL
            Ok(Route::TwoHop {
                from: *from,
                intermediate: feelssol_mint,
                to: *to,
            })
        }
    }
    
    /// Find market for token pair (placeholder)
    fn find_market(&self, _token_0: &Pubkey, _token_1: &Pubkey) -> SdkResult<Pubkey> {
        // This would derive the market PDA
        // For now, return a placeholder
        Ok(Pubkey::default())
    }
}