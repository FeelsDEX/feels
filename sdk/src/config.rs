//! SDK configuration module

use anchor_lang::prelude::*;
use solana_sdk::signature::Keypair;
use std::sync::Arc;

/// SDK Configuration
#[derive(Clone, Debug)]
pub struct SdkConfig {
    /// RPC URL for Solana cluster
    pub rpc_url: String,
    
    /// WebSocket URL for subscriptions
    pub ws_url: String,
    
    /// Program ID for Feels protocol
    pub program_id: Pubkey,
    
    /// Payer keypair
    pub payer: Arc<Keypair>,
    
    /// Commitment level
    pub commitment: String,
    
    /// Skip preflight checks
    pub skip_preflight: bool,
    
    /// Preflight commitment level
    pub preflight_commitment: String,
}

impl SdkConfig {
    /// Create a new SDK configuration
    pub fn new(rpc_url: String, payer: Keypair) -> Self {
        Self {
            rpc_url: rpc_url.clone(),
            ws_url: rpc_url.replace("http", "ws").replace("8899", "8900"),
            program_id: crate::program_id(),
            payer: Arc::new(payer),
            commitment: "confirmed".to_string(),
            skip_preflight: false,
            preflight_commitment: "confirmed".to_string(),
        }
    }
    
    /// Create configuration for localnet
    pub fn localnet(payer: Keypair) -> Self {
        Self::new("http://localhost:8899".to_string(), payer)
    }
    
    /// Create configuration for devnet
    pub fn devnet(payer: Keypair) -> Self {
        Self::new("https://api.devnet.solana.com".to_string(), payer)
    }
    
    /// Create configuration for mainnet
    pub fn mainnet(payer: Keypair) -> Self {
        Self::new("https://api.mainnet-beta.solana.com".to_string(), payer)
    }
    
    /// Set custom program ID
    pub fn with_program_id(mut self, program_id: Pubkey) -> Self {
        self.program_id = program_id;
        self
    }
    
    /// Set commitment level
    pub fn with_commitment(mut self, commitment: String) -> Self {
        self.commitment = commitment.clone();
        self.preflight_commitment = commitment;
        self
    }
    
    /// Skip preflight simulation
    pub fn skip_preflight(mut self, skip: bool) -> Self {
        self.skip_preflight = skip;
        self
    }
}