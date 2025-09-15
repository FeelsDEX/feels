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
use feels::state::{Market, Buffer, ProtocolOracle, SafetyController};

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

    /// Recommended default max fee (bps) for MVP
    pub fn default_max_fee_bps() -> u16 { 150 }

    /// Get protocol oracle rates (native, dex_twap, and min)
    pub fn get_protocol_rate(&self) -> SdkResult<(u128, u128, u128)> {
        let (oracle_pda, _) = Pubkey::find_program_address(&[b"protocol_oracle"], &self.config.program_id);
        let oracle: ProtocolOracle = self.program.account(oracle_pda)
            .map_err(|e| SdkError::AccountNotFound(format!("ProtocolOracle {}: {}", oracle_pda, e)))?;
        let native = oracle.native_rate_q64;
        let dex = oracle.dex_twap_rate_q64;
        let min = if native == 0 { dex } else if dex == 0 { native } else { native.min(dex) };
        Ok((native, dex, min))
    }

    /// Get redemption pause status (true = paused)
    pub fn get_redemption_status(&self) -> SdkResult<bool> {
        let (safety_pda, _) = Pubkey::find_program_address(&[b"safety_controller"], &self.config.program_id);
        let safety: SafetyController = self.program.account(safety_pda)
            .map_err(|e| SdkError::AccountNotFound(format!("SafetyController {}: {}", safety_pda, e)))?;
        Ok(safety.redemptions_paused)
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
        // Derive market from sorted mints and fetch true market state
        let (token_0, token_1) = if user_token_in < user_token_out {
            (*user_token_in, *user_token_out)
        } else {
            (*user_token_out, *user_token_in)
        };
        let (market, _) = crate::find_market_address(&token_0, &token_1);
        let market_state: feels::state::Market = self.program.account(market)
            .map_err(|e| SdkError::AccountNotFound(format!("Market {}: {}", market, e)))?;
        
        let ix = instructions::swap(
            self.config.payer.pubkey(),
            market,
            *user_token_in,
            *user_token_out,
            market_state.token_0,
            market_state.token_1,
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

    /// Execute a swap with a max fee cap (client-side estimate)
    pub fn swap_with_fee_cap(
        &self,
        user_token_in: &Pubkey,
        user_token_out: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        max_fee_bps: u16,
        tick_arrays: Vec<Pubkey>,
    ) -> SdkResult<Signature> {
        // Basic estimate using base fee; in production, use an estimator
        let est_fee_bps: u16 = 30; // fallback base fee
        if est_fee_bps > max_fee_bps {
            return Err(SdkError::InvalidParameters(format!("Estimated fee {}bps exceeds cap {}bps", est_fee_bps, max_fee_bps)));
        }
        self.swap(user_token_in, user_token_out, amount_in, minimum_amount_out, tick_arrays)
    }

    /// Launch Factory: initialize_market then deploy_initial_liquidity (optionally with initial buy)
    #[allow(clippy::too_many_arguments)]
    pub fn launch_pool(
        &self,
        token_0: &Pubkey,
        token_1: &Pubkey,
        feelssol_mint: &Pubkey,
        base_fee_bps: u16,
        tick_spacing: u16,
        initial_sqrt_price: u128,
        initial_buy_feelssol_amount: u64,
        creator_feelssol: Option<Pubkey>,
        creator_token_out: Option<Pubkey>,
        tick_step_size: i32,
    ) -> SdkResult<(Signature, Signature)> {
        let ix_init = instructions::initialize_market(
            self.config.payer.pubkey(),
            *token_0,
            *token_1,
            *feelssol_mint,
            base_fee_bps,
            tick_spacing,
            initial_sqrt_price,
            initial_buy_feelssol_amount,
            creator_feelssol,
            creator_token_out,
        )?;
        let sig_init = self.program.request().instruction(ix_init).send()?;

        // Build deploy_initial_liquidity instruction
        let (market, _) = crate::find_market_address(token_0, token_1);
        // Dummy accounts for optional fields; resolved by builder
        let deployer_feelssol = creator_feelssol.unwrap_or_default();
        let deployer_token_out = creator_token_out.unwrap_or_default();
        let ix_deploy = instructions::deploy_initial_liquidity(
            self.config.payer.pubkey(),
            market,
            *token_0,
            *token_1,
            *feelssol_mint,
            tick_step_size,
            initial_buy_feelssol_amount,
            Some(deployer_feelssol),
            Some(deployer_token_out),
        )?;
        let sig_deploy = self.program.request().instruction(ix_deploy).send()?;
        Ok((sig_init, sig_deploy))
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
    
    // Deprecated placeholder removed: derive market via sorted mints instead
}
