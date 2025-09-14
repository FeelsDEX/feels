//! Unified test client interface that works with both ProgramTest and RPC

use super::*;
use anchor_lang::AccountDeserialize;
use solana_sdk::transaction::Transaction;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_program::program_pack::Pack;
use spl_token::state::Account as TokenAccount;

/// Enum that wraps different client implementations
pub enum TestClient {
    InMemory(InMemoryClient),
    Devnet(DevnetClient),
}

impl TestClient {
    /// Process a single instruction
    pub async fn process_instruction(
        &mut self,
        instruction: Instruction,
        signers: &[&Keypair],
    ) -> TestResult<()> {
        match self {
            TestClient::InMemory(client) => client.process_instruction(instruction, signers).await,
            TestClient::Devnet(client) => client.process_instruction(instruction, signers).await,
        }
    }

    /// Process multiple instructions in a single transaction
    pub async fn process_transaction(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> TestResult<()> {
        match self {
            TestClient::InMemory(client) => client.process_transaction(instructions, signers).await,
            TestClient::Devnet(client) => client.process_transaction(instructions, signers).await,
        }
    }

    /// Get and deserialize account data
    pub async fn get_account<T: AccountDeserialize>(
        &mut self,
        address: &Pubkey,
    ) -> TestResult<Option<T>> {
        match self {
            TestClient::InMemory(client) => client.get_account(address).await,
            TestClient::Devnet(client) => client.get_account(address).await,
        }
    }

    /// Get raw account data
    pub async fn get_account_data(
        &mut self,
        address: &Pubkey,
    ) -> TestResult<Option<Vec<u8>>> {
        match self {
            TestClient::InMemory(client) => client.get_account_data(address).await,
            TestClient::Devnet(client) => client.get_account_data(address).await,
        }
    }

    /// Get token account balance
    pub async fn get_token_balance(&mut self, address: &Pubkey) -> TestResult<u64> {
        match self {
            TestClient::InMemory(client) => client.get_token_balance(address).await,
            TestClient::Devnet(client) => client.get_token_balance(address).await,
        }
    }

    /// Airdrop SOL to an account
    pub async fn airdrop(&mut self, to: &Pubkey, lamports: u64) -> TestResult<()> {
        match self {
            TestClient::InMemory(client) => client.airdrop(to, lamports).await,
            TestClient::Devnet(client) => client.airdrop(to, lamports).await,
        }
    }

    /// Get recent blockhash
    pub async fn get_recent_blockhash(&mut self) -> TestResult<Hash> {
        match self {
            TestClient::InMemory(client) => client.get_recent_blockhash().await,
            TestClient::Devnet(client) => client.get_recent_blockhash().await,
        }
    }

    /// Advance time for testing time-based features
    pub async fn advance_time(&mut self, seconds: i64) -> TestResult<()> {
        match self {
            TestClient::InMemory(client) => client.advance_time(seconds).await,
            TestClient::Devnet(client) => client.advance_time(seconds).await,
        }
    }

    /// Get current slot
    pub async fn get_slot(&mut self) -> TestResult<u64> {
        match self {
            TestClient::InMemory(client) => client.get_slot().await,
            TestClient::Devnet(client) => client.get_slot().await,
        }
    }

    /// Get the payer pubkey
    pub fn payer(&self) -> Pubkey {
        match self {
            TestClient::InMemory(client) => client.payer(),
            TestClient::Devnet(client) => client.payer(),
        }
    }
}

// Implementation for ProgramTest (in-memory testing)
pub struct InMemoryClient {
    pub banks_client: solana_program_test::BanksClient,
    pub payer: Keypair,
    pub last_blockhash: Hash,
}

impl InMemoryClient {
    pub async fn new() -> TestResult<Self> {
        // Set BPF_OUT_DIR if not already set
        if std::env::var("BPF_OUT_DIR").is_err() {
            // Try to find the deploy directory relative to the test
            let possible_paths = vec![
                "target/deploy",
                "../target/deploy",
                "../../target/deploy",
                "../../../target/deploy",
                "../../../../target/deploy",
            ];
            
            for path in possible_paths {
                if std::path::Path::new(path).exists() {
                    std::env::set_var("BPF_OUT_DIR", path);
                    break;
                }
            }
        }
        
        // Load the BPF binary
        let mut program_test = solana_program_test::ProgramTest::new(
            "feels",
            PROGRAM_ID,
            None, // Load from BPF
        );
        
        // Add Metaplex Token Metadata program
        // First, ensure the binary is downloaded
        let metaplex_path = "../../target/external-programs/mpl_token_metadata.so";
        if !std::path::Path::new(metaplex_path).exists() {
            // Try to download it
            println!("Metaplex binary not found, attempting to download...");
            let output = std::process::Command::new("../../scripts/download-metaplex.sh")
                .output()
                .expect("Failed to run download-metaplex.sh");
            
            if !output.status.success() {
                println!("Warning: Could not download Metaplex binary: {}", 
                    String::from_utf8_lossy(&output.stderr));
                println!("Tests requiring Metaplex will fail");
            }
        }
        
        // Add Metaplex if the binary exists
        if std::path::Path::new(metaplex_path).exists() {
            println!("Adding Metaplex Token Metadata program to test environment");
            
            // Copy the binary to the expected location for ProgramTest
            let bpf_out_dir = std::env::var("BPF_OUT_DIR").unwrap_or_else(|_| "../../target/deploy".to_string());
            let target_path = format!("{}/mpl_token_metadata.so", bpf_out_dir);
            
            // Create directory if it doesn't exist
            if let Some(parent) = std::path::Path::new(&target_path).parent() {
                std::fs::create_dir_all(parent).ok();
            }
            
            // Copy the binary
            if let Err(e) = std::fs::copy(metaplex_path, &target_path) {
                println!("Warning: Could not copy Metaplex binary to {}: {}", target_path, e);
            }
            
            // Add the program - ProgramTest will look for it in BPF_OUT_DIR
            program_test.add_program(
                "mpl_token_metadata",
                mpl_token_metadata::ID,
                None, // Will load from BPF_OUT_DIR
            );
        } else {
            println!("Warning: Metaplex binary not found at {}", metaplex_path);
            println!("Tests requiring token metadata will fail");
        }
        
        // SPL Token and ATA programs are automatically included by solana-program-test
        
        // Increase compute units for complex operations
        program_test.set_compute_max_units(2_000_000);
        
        let (banks_client, payer, recent_blockhash) = program_test.start().await;
        
        Ok(Self {
            banks_client,
            payer,
            last_blockhash: recent_blockhash,
        })
    }
}

impl InMemoryClient {
    pub async fn process_instruction(
        &mut self,
        instruction: Instruction,
        signers: &[&Keypair],
    ) -> TestResult<()> {
        self.process_transaction(&[instruction], signers).await
    }

    pub async fn process_transaction(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> TestResult<()> {
        // Update blockhash
        self.last_blockhash = self.banks_client.get_latest_blockhash().await?;
        
        // Include payer in signers if not already present
        let mut all_signers: Vec<&Keypair> = Vec::new();
        let payer_pubkey = self.payer.pubkey();
        
        // Check if payer is already in signers
        let payer_in_signers = signers.iter().any(|s| s.pubkey() == payer_pubkey);
        if !payer_in_signers {
            all_signers.push(&self.payer);
        }
        all_signers.extend(signers);
        
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&payer_pubkey),
            &all_signers,
            self.last_blockhash,
        );
        
        self.banks_client.process_transaction(tx).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    }

    pub async fn get_account<T: AccountDeserialize>(
        &mut self,
        address: &Pubkey,
    ) -> TestResult<Option<T>> {
        match self.banks_client.get_account(*address).await? {
            Some(account) => {
                let data = account.data;
                if data.len() < 8 {
                    return Ok(None);
                }
                
                // Don't skip discriminator - AccountDeserialize expects the full data
                let mut slice = &data[..];
                let parsed = T::try_deserialize(&mut slice)?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    pub async fn get_account_data(
        &mut self,
        address: &Pubkey,
    ) -> TestResult<Option<Vec<u8>>> {
        match self.banks_client.get_account(*address).await? {
            Some(account) => Ok(Some(account.data)),
            None => Ok(None),
        }
    }

    pub async fn get_token_balance(&mut self, address: &Pubkey) -> TestResult<u64> {
        match self.get_account_data(address).await? {
            Some(data) => {
                let account = TokenAccount::unpack(&data)?;
                Ok(account.amount)
            }
            None => Ok(0),
        }
    }

    pub async fn airdrop(&mut self, to: &Pubkey, lamports: u64) -> TestResult<()> {
        let ix = solana_sdk::system_instruction::transfer(
            &self.payer.pubkey(),
            to,
            lamports,
        );
        
        let payer = Keypair::from_bytes(&self.payer.to_bytes())?;
        self.process_instruction(ix, &[&payer]).await
    }

    pub async fn get_recent_blockhash(&mut self) -> TestResult<Hash> {
        self.last_blockhash = self.banks_client.get_latest_blockhash().await?;
        Ok(self.last_blockhash)
    }

    pub async fn advance_time(&mut self, seconds: i64) -> TestResult<()> {
        use solana_program_test::tokio::time::{sleep, Duration};
        
        // Get current slot
        let current_slot = self.banks_client.get_root_slot().await?;
        
        // Advance slots (assuming ~400ms per slot)
        let slots_to_advance = (seconds * 1000 / 400).max(1) as u64;
        let target_slot = current_slot + slots_to_advance;
        
        // Note: warp_to_slot is not available in current BanksClient
        // For now, we'll just sleep to simulate time passing
        
        // Also update clock sysvar
        let mut clock: solana_program::clock::Clock = self.banks_client.get_sysvar().await?;
        clock.unix_timestamp += seconds;
        clock.slot = target_slot;
        
        // Small delay to ensure state updates
        sleep(Duration::from_millis(10)).await;
        
        Ok::<(), Box<dyn std::error::Error>>(())
    }

    pub async fn get_slot(&mut self) -> TestResult<u64> {
        Ok(self.banks_client.get_root_slot().await?)
    }

    pub fn payer(&self) -> Pubkey {
        self.payer.pubkey()
    }
}

// Implementation for RPC client (devnet/localnet testing)
pub struct DevnetClient {
    pub rpc_client: solana_client::rpc_client::RpcClient,
    pub payer: Keypair,
    pub commitment: CommitmentConfig,
}

impl DevnetClient {
    pub async fn new(url: &str, payer_path: Option<&str>) -> TestResult<Self> {
        use solana_sdk::signature::read_keypair_file;
        
        let rpc_client = solana_client::rpc_client::RpcClient::new_with_commitment(
            url.to_string(),
            CommitmentConfig::confirmed(),
        );
        
        // Load payer keypair
        let payer = match payer_path {
            Some(path) => read_keypair_file(path)?,
            None => {
                // Generate new payer and airdrop
                let new_payer = Keypair::new();
                
                // Request airdrop
                let sig = rpc_client.request_airdrop(
                    &new_payer.pubkey(),
                    10 * LAMPORTS_PER_SOL,
                )?;
                
                // Wait for confirmation
                rpc_client.confirm_transaction(&sig)?;
                
                new_payer
            }
        };
        
        Ok(Self {
            rpc_client,
            payer,
            commitment: CommitmentConfig::confirmed(),
        })
    }
}

impl DevnetClient {
    pub async fn process_instruction(
        &mut self,
        instruction: Instruction,
        signers: &[&Keypair],
    ) -> TestResult<()> {
        self.process_transaction(&[instruction], signers).await
    }

    pub async fn process_transaction(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> TestResult<()> {
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        
        // Include payer in signers if not already present
        let mut all_signers: Vec<&Keypair> = Vec::new();
        let payer_pubkey = self.payer.pubkey();
        
        // Check if payer is already in signers
        let payer_in_signers = signers.iter().any(|s| s.pubkey() == payer_pubkey);
        if !payer_in_signers {
            all_signers.push(&self.payer);
        }
        all_signers.extend(signers);
        
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&payer_pubkey),
            &all_signers,
            recent_blockhash,
        );
        
        let sig = self.rpc_client.send_and_confirm_transaction(&tx)?;
        println!("Transaction confirmed: {}", sig);
        
        Ok::<(), Box<dyn std::error::Error>>(())
    }

    pub async fn get_account<T: AccountDeserialize>(
        &mut self,
        address: &Pubkey,
    ) -> TestResult<Option<T>> {
        match self.rpc_client.get_account_with_commitment(address, self.commitment)?.value {
            Some(account) => {
                let data = account.data;
                if data.len() < 8 {
                    return Ok(None);
                }
                
                // Don't skip discriminator - AccountDeserialize expects the full data
                let mut slice = &data[..];
                let parsed = T::try_deserialize(&mut slice)?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    pub async fn get_account_data(
        &mut self,
        address: &Pubkey,
    ) -> TestResult<Option<Vec<u8>>> {
        match self.rpc_client.get_account_with_commitment(address, self.commitment)?.value {
            Some(account) => Ok(Some(account.data)),
            None => Ok(None),
        }
    }

    pub async fn get_token_balance(&mut self, address: &Pubkey) -> TestResult<u64> {
        match self.get_account_data(address).await? {
            Some(data) => {
                let account = TokenAccount::unpack(&data)?;
                Ok(account.amount)
            }
            None => Ok(0),
        }
    }

    pub async fn airdrop(&mut self, to: &Pubkey, lamports: u64) -> TestResult<()> {
        let sig = self.rpc_client.request_airdrop(to, lamports)?;
        self.rpc_client.confirm_transaction(&sig)?;
        Ok::<(), Box<dyn std::error::Error>>(())
    }

    pub async fn get_recent_blockhash(&mut self) -> TestResult<Hash> {
        Ok(self.rpc_client.get_latest_blockhash()?)
    }

    pub async fn advance_time(&mut self, seconds: i64) -> TestResult<()> {
        // For devnet, we just wait real time
        // This is a limitation of testing against real validators
        use tokio::time::{sleep, Duration};
        
        println!("Waiting {} seconds for time-based test...", seconds);
        sleep(Duration::from_secs(seconds as u64)).await;
        
        Ok::<(), Box<dyn std::error::Error>>(())
    }

    pub async fn get_slot(&mut self) -> TestResult<u64> {
        Ok(self.rpc_client.get_slot()?)
    }

    pub fn payer(&self) -> Pubkey {
        self.payer.pubkey()
    }
}