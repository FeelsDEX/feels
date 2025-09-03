use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
    instruction::Instruction,
    pubkey::Pubkey,
};
use std::sync::Arc;

use crate::{SdkConfig, SdkResult, SdkError};

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

    /// Send a transaction
    pub async fn send_transaction(&self, instructions: &[Instruction]) -> SdkResult<Signature> {
        let recent_blockhash = self.rpc_client.get_latest_blockhash()
            .map_err(|e| SdkError::RpcError(e.to_string()))?;
        
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&self.payer.pubkey()),
            &[&*self.payer],
            recent_blockhash,
        );

        let sig = self.rpc_client.send_and_confirm_transaction(&tx)
            .map_err(|e| SdkError::TransactionFailed(e.to_string()))?;
        Ok(sig)
    }

    /// Get the program ID
    pub fn program_id(&self) -> &Pubkey {
        &self.program_id
    }

    /// Get the payer's public key
    pub fn payer(&self) -> Pubkey {
        self.payer.pubkey()
    }

    /// Check RPC connection health
    pub async fn health_check(&self) -> SdkResult<()> {
        self.rpc_client.get_health()
            .map_err(|e| SdkError::RpcError(e.to_string()))?;
        Ok(())
    }
}