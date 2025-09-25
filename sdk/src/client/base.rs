use std::sync::Arc;

use crate::prelude::*;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

use crate::core::{program_id, SdkError, SdkResult};

/// Base RPC client wrapper for common operations
pub struct BaseClient {
    rpc: Arc<RpcClient>,
    program_id: Pubkey,
}

impl BaseClient {
    pub fn new(rpc: Arc<RpcClient>) -> Self {
        Self {
            rpc,
            program_id: program_id(),
        }
    }

    pub fn with_program_id(rpc: Arc<RpcClient>, program_id: Pubkey) -> Self {
        Self { rpc, program_id }
    }

    /// Get the RPC client
    pub fn rpc(&self) -> &RpcClient {
        &self.rpc
    }

    /// Get the program ID
    pub fn program_id(&self) -> Pubkey {
        self.program_id
    }

    /// Get the RPC endpoint URL
    pub fn rpc_url(&self) -> String {
        self.rpc.url()
    }

    /// Fetch an account
    pub async fn get_account(&self, address: &Pubkey) -> SdkResult<Account> {
        self.rpc
            .get_account(address)
            .await
            .map_err(|e| SdkError::RpcError(e))
    }

    /// Fetch multiple accounts
    pub async fn get_multiple_accounts(
        &self,
        addresses: &[Pubkey],
    ) -> SdkResult<Vec<Option<Account>>> {
        self.rpc
            .get_multiple_accounts(addresses)
            .await
            .map_err(|e| SdkError::RpcError(e))
    }

    /// Send a transaction
    pub async fn send_transaction(
        &self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> SdkResult<Signature> {
        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&signers[0].pubkey()),
            signers,
            recent_blockhash,
        );

        self.rpc
            .send_and_confirm_transaction(&tx)
            .await
            .map_err(|e| SdkError::RpcError(e))
    }

    /// Send a transaction with custom options
    pub async fn send_transaction_with_config(
        &self,
        instructions: &[Instruction],
        signers: &[&Keypair],
        commitment: CommitmentConfig,
    ) -> SdkResult<Signature> {
        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&signers[0].pubkey()),
            signers,
            recent_blockhash,
        );

        self.rpc
            .send_and_confirm_transaction_with_spinner_and_commitment(&tx, commitment)
            .await
            .map_err(|e| SdkError::RpcError(e))
    }

    /// Simulate a transaction
    pub async fn simulate_transaction(
        &self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> SdkResult<()> {
        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&signers[0].pubkey()),
            signers,
            recent_blockhash,
        );

        let result = self.rpc.simulate_transaction(&tx).await?;
        
        if let Some(err) = result.value.err {
            return Err(SdkError::SimulationFailed(format!("{:?}", err)));
        }

        Ok(())
    }

    /// Get current slot
    pub async fn get_slot(&self) -> SdkResult<u64> {
        self.rpc.get_slot().await.map_err(|e| SdkError::RpcError(e))
    }

    /// Get account balance
    pub async fn get_balance(&self, pubkey: &Pubkey) -> SdkResult<u64> {
        self.rpc
            .get_balance(pubkey)
            .await
            .map_err(|e| SdkError::RpcError(e))
    }
}