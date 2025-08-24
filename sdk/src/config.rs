use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::{str::FromStr, sync::Arc};

/// SDK configuration for connecting to the Feels Protocol
#[derive(Clone)]
pub struct SdkConfig {
    /// RPC endpoint URL
    pub rpc_url: String,
    
    /// WebSocket URL for subscriptions
    pub ws_url: Option<String>,
    
    /// Feels program ID
    pub program_id: Pubkey,
    
    /// Transaction commitment level
    pub commitment: solana_sdk::commitment_config::CommitmentLevel,
    
    /// Request timeout in seconds
    pub timeout: u64,
    
    /// Payer keypair for transactions
    pub payer: Arc<Keypair>,
}

impl SdkConfig {
    pub fn localnet(payer: Keypair) -> Self {
        Self {
            rpc_url: "http://localhost:8899".to_string(),
            ws_url: Some("ws://localhost:8900".to_string()),
            program_id: Pubkey::from_str("Fee1sProtoco11111111111111111111111111111111").unwrap(),
            commitment: solana_sdk::commitment_config::CommitmentLevel::Confirmed,
            timeout: 30,
            payer: Arc::new(payer),
        }
    }
    
    pub fn devnet(payer: Keypair) -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            ws_url: Some("wss://api.devnet.solana.com".to_string()),
            program_id: Pubkey::from_str("Fee1sProtoco11111111111111111111111111111111").unwrap(),
            commitment: solana_sdk::commitment_config::CommitmentLevel::Confirmed,
            timeout: 30,
            payer: Arc::new(payer),
        }
    }
    
    pub fn mainnet(payer: Keypair) -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            ws_url: Some("wss://api.mainnet-beta.solana.com".to_string()),
            program_id: Pubkey::from_str("Fee1sProtoco11111111111111111111111111111111").unwrap(),
            commitment: solana_sdk::commitment_config::CommitmentLevel::Confirmed,
            timeout: 30,
            payer: Arc::new(payer),
        }
    }
    
    pub fn with_rpc_url(mut self, url: String) -> Self {
        self.rpc_url = url;
        self
    }
    
    pub fn with_program_id(mut self, program_id: Pubkey) -> Self {
        self.program_id = program_id;
        self
    }
}