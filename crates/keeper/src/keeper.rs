use std::sync::Arc;
use std::collections::HashMap;
use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::{
    pubkey::Pubkey, 
    signature::Keypair,
    transaction::Transaction,
    instruction::Instruction,
    system_program,
    sysvar::clock,
};
use anchor_client::{
    Client, Cluster, Program,
    anchor_lang::{prelude::*, InstructionData, ToAccountMetas},
};

use crate::field_computation::FieldComputer;
use crate::config::{KeeperConfig, MarketConfig};
use crate::hysteresis_controller::{HysteresisController, DomainWeights};

// Use shared types
use feels_types::{MarketState, FieldCommitmentData, FeelsResult, FeelsProtocolError};

/// Main keeper service that manages off-chain field computation and updates
pub struct Keeper {
    /// RPC client for Solana interaction
    rpc_client: Arc<RpcClient>,
    
    /// Keeper authority keypair
    keypair: Arc<Keypair>,
    
    /// Keeper configuration
    config: KeeperConfig,
    
    /// Field computation engine
    field_computer: FieldComputer,
    
    /// Anchor client for program interaction
    anchor_client: Client,
    
    /// Program handle
    feels_program: Program,
    
    /// Last update timestamps per market
    last_updates: HashMap<Pubkey, i64>,
    
    /// Hysteresis controllers per market
    hysteresis_controllers: HashMap<Pubkey, HysteresisController>,
    
    /// Dry run mode flag
    dry_run: bool,
}

impl Keeper {
    /// Create a new keeper instance
    pub fn new(
        rpc_client: Arc<RpcClient>,
        keypair: Arc<Keypair>,
        config: KeeperConfig,
        dry_run: bool,
    ) -> FeelsResult<Self> {
        // Initialize Anchor client
        let cluster = match config.cluster.as_str() {
            "mainnet" => Cluster::Mainnet,
            "devnet" => Cluster::Devnet,
            "testnet" => Cluster::Testnet,
            _ => Cluster::Custom(config.cluster.clone(), config.cluster.clone()),
        };
        
        let anchor_client = Client::new_with_options(
            cluster,
            keypair.clone(),
            anchor_client::ClientConfig::default(),
        );
        
        let feels_program = anchor_client.program(config.program_id)
            .map_err(|e| FeelsProtocolError::rpc_error(&e.to_string(), None))?;
        
        Ok(Self {
            rpc_client,
            keypair,
            config,
            field_computer: FieldComputer::new(),
            anchor_client,
            feels_program,
            last_updates: HashMap::new(),
            hysteresis_controllers: HashMap::new(),
            dry_run,
        })
    }

    /// Update all configured markets that need updates
    pub async fn update_all_markets(&mut self) -> FeelsResult<usize> {
        let mut updates = 0;
        
        // Clone markets config to avoid borrow checker issues
        let markets = self.config.markets.clone();
        for market_config in &markets {
            match self.update_market(market_config).await {
                Ok(updated) => {
                    if updated {
                        updates += 1;
                    }
                }
                Err(e) => {
                    log::error!("Failed to update market {}: {}", market_config.market_pubkey, e);
                    // Continue with other markets
                }
            }
        }
        
        Ok(updates)
    }

    /// Update a specific market if needed
    pub async fn update_market(&mut self, market_config: &MarketConfig) -> FeelsResult<bool> {
        let current_time = chrono::Utc::now().timestamp();
        let market_key = market_config.market_pubkey;
        
        // Check if update is needed based on staleness threshold
        if let Some(&last_update) = self.last_updates.get(&market_key) {
            let time_since_update = current_time - last_update;
            if time_since_update < market_config.min_update_interval {
                log::debug!("Market {} updated {}s ago, skipping", market_key, time_since_update);
                return Ok(false);
            }
        }

        log::info!("Updating market: {}", market_key);

        // Fetch current market state
        let mut market_state = self.fetch_market_state(&market_key).await
            .map_err(|e| FeelsProtocolError::rpc_error(&format!("Failed to fetch market state for {}: {}", market_key, e), None))?;

        // Get or create hysteresis controller for this market
        let hysteresis_controller = self.hysteresis_controllers.entry(market_key)
            .or_insert_with(|| {
                // Use domain weights from field commitment if available
                let weights = if let Some(ref field) = market_state.field_commitment {
                    DomainWeights {
                        w_s: field.w_s,
                        w_t: field.w_t,
                        w_l: field.w_l,
                    }
                } else {
                    // Default weights
                    DomainWeights {
                        w_s: 7000,  // 70% spot
                        w_t: 2000,  // 20% time
                        w_l: 1000,  // 10% leverage
                    }
                };
                HysteresisController::new(weights).expect("Failed to create hysteresis controller")
            });

        // Compute stress components
        let stress_components = self.field_computer.compute_stress_components(&market_state)
            .map_err(|e| FeelsProtocolError::field_commitment_error(&format!("Failed to compute stress components for {}: {}", market_key, e), None))?;

        // Update base fee using hysteresis controller
        let base_fee_bps = hysteresis_controller.update(&stress_components, current_time)
            .map_err(|e| FeelsProtocolError::field_commitment_error(&format!("Failed to update base fee for {}: {}", market_key, e), None))?;
        
        let controller_state = hysteresis_controller.get_state();
        log::info!("Hysteresis controller for {}: base_fee={} bps, stress={} bps, direction={:?}, in_dead_zone={}",
                   market_key, controller_state.current_fee, controller_state.stress_ewma, 
                   controller_state.last_direction, controller_state.in_dead_zone);
        
        // Add base fee to market state for field computation
        market_state.base_fee_bps = Some(base_fee_bps);

        // Compute field commitment  
        let field_commitment = {
            let mut field_computer = FieldComputer::new();
            field_computer.compute_field_commitment(&market_state)
                .map_err(|e| FeelsProtocolError::field_commitment_error(&format!("Failed to compute field commitment for {}: {}", market_key, e), None))?
        };

        log::debug!("Computed field commitment for {}: S={}, T={}, L={}", 
                   market_key, field_commitment.S, field_commitment.T, field_commitment.L);

        // Check if update is significant enough
        if !self.should_update_field(&market_key, &field_commitment).await? {
            log::debug!("Field commitment change not significant enough for {}", market_key);
            return Ok(false);
        }

        // Submit field commitment update
        if self.dry_run {
            log::info!("DRY RUN: Would update field commitment for {}", market_key);
            return Ok(true);
        }

        match self.submit_field_update(&market_key, &field_commitment).await {
            Ok(signature) => {
                log::info!("Successfully updated field commitment for {}, tx: {}", market_key, signature);
                // Update last update timestamp
                self.last_updates.insert(market_key, current_time);
                Ok(true)
            }
            Err(e) => {
                log::error!("Failed to submit field update for {}: {}", market_key, e);
                Err(e)
            }
        }
    }

    /// Fetch current market state from on-chain data
    async fn fetch_market_state(&self, market_pubkey: &Pubkey) -> FeelsResult<MarketState> {
        // Fetch market field account
        let market_account = self.rpc_client.get_account(market_pubkey)
            .map_err(|e| FeelsProtocolError::rpc_error(&e.to_string(), None))?;
        
        // Parse market field data (simplified - would use proper Anchor deserialization)
        let market_data = self.parse_market_field_account(&market_account.data)?;
        
        // Fetch buffer account
        let buffer_seeds = &[b"buffer", market_pubkey.as_ref()];
        let (buffer_pubkey, _) = Pubkey::find_program_address(buffer_seeds, &self.config.program_id);
        let buffer_account = self.rpc_client.get_account(&buffer_pubkey)
            .map_err(|e| FeelsProtocolError::rpc_error(&e.to_string(), None))?;
        let buffer_data = self.parse_buffer_account(&buffer_account.data)?;

        // Fetch TWAP data
        let twap_seeds = &[b"twap", market_pubkey.as_ref()];
        let (twap_pubkey, _) = Pubkey::find_program_address(twap_seeds, &self.config.program_id);
        let twap_data = match self.rpc_client.get_account(&twap_pubkey) {
            Ok(account) => Some(self.parse_twap_account(&account.data)?),
            Err(_) => None,
        };

        Ok(MarketState {
            market_pubkey: *market_pubkey,
            current_sqrt_price: market_data.current_sqrt_rate,
            liquidity: market_data.liquidity,
            tick_current: market_data.current_tick,
            fee_growth_global_0: market_data.fee_growth_global_0,
            fee_growth_global_1: market_data.fee_growth_global_1,
            protocol_fees_0: buffer_data.protocol_fees_0,
            protocol_fees_1: buffer_data.protocol_fees_1,
            twap_0: twap_data.as_ref().map(|t| t.price_0).unwrap_or(market_data.current_sqrt_rate),
            twap_1: twap_data.as_ref().map(|t| t.price_1).unwrap_or(market_data.current_sqrt_rate),
            last_update_ts: chrono::Utc::now().timestamp(),
            // Additional required fields
            total_volume_0: 0, // Would fetch from market data
            total_volume_1: 0, // Would fetch from market data
            swap_count: 0, // Would fetch from market data
            token_0_mint: Pubkey::default(), // Would fetch from market config
            token_1_mint: Pubkey::default(), // Would fetch from market config
            token_0_decimals: 6, // Would fetch from token metadata
            token_1_decimals: 6, // Would fetch from token metadata
            field_commitment: None, // Will be computed later
            base_fee_bps: None, // Will be set by hysteresis controller
        })
    }

    /// Check if field commitment update is significant enough
    async fn should_update_field(&self, market_pubkey: &Pubkey, new_field: &FieldCommitmentData) -> FeelsResult<bool> {
        // Get current field commitment if exists
        let field_seeds = &[b"field_commitment", market_pubkey.as_ref()];
        let (field_pubkey, _) = Pubkey::find_program_address(field_seeds, &self.config.program_id);
        
        let current_field = match self.rpc_client.get_account(&field_pubkey) {
            Ok(account) => Some(self.parse_field_commitment(&account.data)?),
            Err(_) => None,
        };

        // Always update if no current field exists
        let Some(current) = current_field else {
            return Ok(true);
        };

        // Check staleness
        let current_time = chrono::Utc::now().timestamp();
        let staleness = current_time - current.snapshot_ts;
        if staleness > current.max_staleness {
            log::debug!("Field commitment is stale ({}s old), updating", staleness);
            return Ok(true);
        }

        // Check for significant changes (>1% in any major scalar)
        const CHANGE_THRESHOLD: u128 = 100; // 1% in basis points

        let s_change = calculate_change_bps(new_field.S, current.S);
        let t_change = calculate_change_bps(new_field.T, current.T);
        let l_change = calculate_change_bps(new_field.L, current.L);

        let significant_change = s_change > CHANGE_THRESHOLD 
            || t_change > CHANGE_THRESHOLD 
            || l_change > CHANGE_THRESHOLD;

        if significant_change {
            log::debug!("Significant field change detected: S={}bps, T={}bps, L={}bps", 
                       s_change, t_change, l_change);
        }

        Ok(significant_change)
    }

    /// Submit field commitment update transaction
    async fn submit_field_update(
        &self,
        market_pubkey: &Pubkey,
        field_commitment: &FieldCommitmentData,
    ) -> FeelsResult<String> {
        // Build update instruction
        let instruction = self.build_update_instruction(market_pubkey, field_commitment)?;
        
        // Create and send transaction
        let recent_blockhash = self.rpc_client.get_latest_blockhash()
            .map_err(|e| FeelsProtocolError::rpc_error(&e.to_string(), None))?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.keypair.pubkey()),
            &[&*self.keypair],
            recent_blockhash,
        );

        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)
            .map_err(|e| FeelsProtocolError::rpc_error(&e.to_string(), None))?;
        
        Ok(signature.to_string())
    }

    /// Build the update_field_commitment instruction
    fn build_update_instruction(
        &self,
        market_pubkey: &Pubkey,
        field_commitment: &FieldCommitmentData,
    ) -> FeelsResult<Instruction> {
        // Calculate PDAs
        let field_seeds = &[b"field_commitment", market_pubkey.as_ref()];
        let (field_commitment_pubkey, _) = Pubkey::find_program_address(field_seeds, &self.config.program_id);
        
        let protocol_seeds = &[b"protocol"];
        let (protocol_state_pubkey, _) = Pubkey::find_program_address(protocol_seeds, &self.config.program_id);
        
        let buffer_seeds = &[b"buffer", market_pubkey.as_ref()];
        let (buffer_pubkey, _) = Pubkey::find_program_address(buffer_seeds, &self.config.program_id);

        // Convert field commitment to keeper update params
        let params = field_commitment.to_keeper_update_params();

        // Use anchor client to build instruction - simplified approach
        let accounts = vec![
            AccountMeta::new(field_commitment_pubkey, false),
            AccountMeta::new(*market_pubkey, false),
            AccountMeta::new_readonly(protocol_state_pubkey, false),
            AccountMeta::new(buffer_pubkey, false),
            AccountMeta::new_readonly(self.keypair.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let instruction = Instruction {
            program_id: self.config.program_id,
            accounts,
            data: params.to_instruction_data()?,
        };

        Ok(instruction)
    }

    /// Health check for keeper service
    pub async fn health_check(&self) -> FeelsResult<()> {
        // Check RPC connection
        let _ = self.rpc_client.get_health()
            .map_err(|e| FeelsProtocolError::rpc_error(&e.to_string(), None))?;
        
        // Check balance
        let balance = self.rpc_client.get_balance(&self.keypair.pubkey())
            .map_err(|e| FeelsProtocolError::rpc_error(&e.to_string(), None))?;
        if balance < self.config.min_balance_lamports {
            return Err(FeelsProtocolError::insufficient_balance(balance, self.config.min_balance_lamports));
        }
        
        log::debug!("Health check passed - balance: {} lamports", balance);
        Ok(())
    }

    // Helper parsing functions (simplified - would use proper Anchor deserialization)
    fn parse_market_field_account(&self, data: &[u8]) -> FeelsResult<MarketFieldData> {
        // Simplified parsing - in practice would use proper Anchor deserialization
        if data.len() < 8 + 128 { // Account discriminator + minimum data
            return Err(FeelsProtocolError::parse_error("Invalid market field account data", None));
        }
        
        // Skip discriminator and parse basic fields (simplified)
        let current_sqrt_rate = u128::from_le_bytes(
            data[8..24].try_into().map_err(|_| FeelsProtocolError::parse_error("Invalid sqrt rate", None))?
        );
        let current_tick = i32::from_le_bytes(
            data[24..28].try_into().map_err(|_| FeelsProtocolError::parse_error("Invalid tick", None))?
        );
        let liquidity = u128::from_le_bytes(
            data[28..44].try_into().map_err(|_| FeelsProtocolError::parse_error("Invalid liquidity", None))?
        );
        
        Ok(MarketFieldData {
            current_sqrt_rate,
            current_tick,
            liquidity,
            fee_growth_global_0: 0, // Would parse actual values
            fee_growth_global_1: 0,
        })
    }

    fn parse_buffer_account(&self, data: &[u8]) -> FeelsResult<BufferData> {
        if data.len() < 8 + 64 {
            return Err(FeelsProtocolError::parse_error("Invalid buffer account data", None));
        }
        
        Ok(BufferData {
            protocol_fees_0: 0, // Would parse actual values
            protocol_fees_1: 0,
        })
    }

    fn parse_twap_account(&self, data: &[u8]) -> FeelsResult<TwapData> {
        if data.len() < 8 + 32 {
            return Err(FeelsProtocolError::parse_error("Invalid TWAP account data", None));
        }
        
        Ok(TwapData {
            price_0: u128::from_le_bytes(data[8..24].try_into().unwrap()),
            price_1: u128::from_le_bytes(data[24..40].try_into().unwrap()),
        })
    }

    fn parse_field_commitment(&self, data: &[u8]) -> FeelsResult<FieldCommitmentData> {
        if data.len() < 8 + 200 {
            return Err(FeelsProtocolError::parse_error("Invalid field commitment data", None));
        }
        
        // Simplified parsing
        Ok(FieldCommitmentData {
            S: u128::from_le_bytes(data[8..24].try_into().unwrap()),
            T: u128::from_le_bytes(data[24..40].try_into().unwrap()),
            L: u128::from_le_bytes(data[40..56].try_into().unwrap()),
            w_s: u32::from_le_bytes(data[56..60].try_into().unwrap()),
            w_t: u32::from_le_bytes(data[60..64].try_into().unwrap()),
            w_l: u32::from_le_bytes(data[64..68].try_into().unwrap()),
            w_tau: u32::from_le_bytes(data[68..72].try_into().unwrap()),
            omega_0: u32::from_le_bytes(data[72..76].try_into().unwrap()),
            omega_1: u32::from_le_bytes(data[76..80].try_into().unwrap()),
            sigma_price: 0, // Would parse from data[80..]
            sigma_rate: 0,  // Would parse from data[88..]
            sigma_leverage: 0, // Would parse from data[96..]
            twap_0: u128::from_le_bytes(data[80..96].try_into().unwrap()),
            twap_1: u128::from_le_bytes(data[96..112].try_into().unwrap()),
            snapshot_ts: i64::from_le_bytes(data[112..120].try_into().unwrap()),
            max_staleness: i64::from_le_bytes(data[120..128].try_into().unwrap()),
            sequence: u64::from_le_bytes(data[128..136].try_into().unwrap()),
            base_fee_bps: 25, // Default value
            local_coefficients: None,
            commitment_hash: None,
            lipschitz_L: None,
            gap_bps: None,
        })
    }
}

// Helper data structures
#[derive(Debug)]
struct MarketFieldData {
    current_sqrt_rate: u128,
    current_tick: i32,
    liquidity: u128,
    fee_growth_global_0: u128,
    fee_growth_global_1: u128,
}

#[derive(Debug)]
struct BufferData {
    protocol_fees_0: u64,
    protocol_fees_1: u64,
}

#[derive(Debug)]
struct TwapData {
    price_0: u128,
    price_1: u128,
}

/// Calculate change in basis points between two values
fn calculate_change_bps(new_value: u128, old_value: u128) -> u128 {
    if old_value == 0 {
        return if new_value == 0 { 0 } else { 10000 }; // 100% change
    }
    
    let diff = if new_value > old_value {
        new_value - old_value
    } else {
        old_value - new_value
    };
    
    (diff * 10000) / old_value
}