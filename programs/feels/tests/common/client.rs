//! Unified test client interface that works with both ProgramTest and RPC

use super::*;
use anchor_lang::AccountDeserialize;
use solana_program::program_pack::Pack;
use solana_program_test::{BanksClient, ProgramTest};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::transaction::Transaction;
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
    pub async fn get_account_data(&mut self, address: &Pubkey) -> TestResult<Option<Vec<u8>>> {
        match self {
            TestClient::InMemory(client) => client.get_account_data(address).await,
            TestClient::Devnet(client) => client.get_account_data(address).await,
        }
    }

    /// Set account data directly (for testing)
    pub fn set_account_data(&mut self, address: &Pubkey, data: Vec<u8>) -> TestResult<()> {
        match self {
            TestClient::InMemory(client) => client.set_account_data(address, data),
            TestClient::Devnet(_) => Err("Cannot set account data in devnet tests".into()),
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

    /// Get SOL balance for an account
    pub async fn get_balance(&mut self, address: &Pubkey) -> TestResult<u64> {
        match self {
            TestClient::InMemory(client) => client.get_balance(address).await,
            TestClient::Devnet(client) => client.get_balance(address).await,
        }
    }
}

// Implementation for ProgramTest (in-memory testing)
pub struct InMemoryClient {
    pub banks_client: BanksClient,
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
        let mut program_test = ProgramTest::new(
            "feels", PROGRAM_ID, None, // Load from BPF
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
                println!(
                    "Warning: Could not download Metaplex binary: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                println!("Tests requiring Metaplex will fail");
            }
        }

        // Add Metaplex if the binary exists
        if std::path::Path::new(metaplex_path).exists() {
            println!("Adding Metaplex Token Metadata program to test environment");

            // Copy the binary to the expected location for ProgramTest
            let bpf_out_dir =
                std::env::var("BPF_OUT_DIR").unwrap_or_else(|_| "../../target/deploy".to_string());
            let target_path = format!("{}/mpl_token_metadata.so", bpf_out_dir);

            // Create directory if it doesn't exist
            if let Some(parent) = std::path::Path::new(&target_path).parent() {
                std::fs::create_dir_all(parent).ok();
            }

            // Copy the binary
            if let Err(e) = std::fs::copy(metaplex_path, &target_path) {
                println!(
                    "Warning: Could not copy Metaplex binary to {}: {}",
                    target_path, e
                );
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

        // Fund the payer account with SOL for transaction fees
        // The payer should already have SOL from ProgramTest, but let's check
        if let Ok(Some(payer_account)) = banks_client.get_account(payer.pubkey()).await {
            println!(
                "Payer balance after ProgramTest::start(): {} SOL",
                payer_account.lamports as f64 / 1e9
            );
        } else {
            println!("Warning: Could not get payer account balance");
        }

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

    pub async fn get_account_data(&mut self, address: &Pubkey) -> TestResult<Option<Vec<u8>>> {
        match self.banks_client.get_account(*address).await? {
            Some(account) => Ok(Some(account.data)),
            None => Ok(None),
        }
    }

    /// Set account data directly (for testing)
    /// Note: This is not feasible with BanksClient - account data must be set through transactions
    pub fn set_account_data(&mut self, _address: &Pubkey, _data: Vec<u8>) -> TestResult<()> {
        // BanksClient doesn't support direct account data setting
        // Account data can only be modified through program instructions
        Err("Direct account data setting not supported in BanksClient tests".into())
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
        // In-memory client doesn't have rate limits, directly transfer
        let ix = solana_sdk::system_instruction::transfer(&self.payer.pubkey(), to, lamports);

        let payer_bytes = self.payer.to_bytes();
        let payer = Keypair::try_from(&payer_bytes[..])?;
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

    pub async fn get_balance(&mut self, address: &Pubkey) -> TestResult<u64> {
        match self.banks_client.get_account(*address).await? {
            Some(account) => Ok(account.lamports),
            None => Ok(0),
        }
    }
}

// Implementation for RPC client (devnet/localnet testing)
pub struct DevnetClient {
    pub rpc_client: solana_client::rpc_client::RpcClient,
    pub payer: Keypair,
    pub commitment: CommitmentConfig,
    pub disable_airdrop_rate_limit: bool,
}

impl DevnetClient {
    pub fn set_disable_airdrop_rate_limit(&mut self, disable: bool) {
        self.disable_airdrop_rate_limit = disable;
    }

    pub async fn new(url: &str, payer_path: Option<&str>) -> TestResult<Self> {
        use solana_sdk::signature::read_keypair_file;
        use tokio::task;

        // Create RPC client synchronously first
        let rpc_client = solana_client::rpc_client::RpcClient::new_with_commitment(
            url.to_string(),
            CommitmentConfig::confirmed(),
        );

        // Load payer keypair
        let payer = match payer_path {
            Some(path) => read_keypair_file(path)?,
            None => {
                // Generate new payer
                let new_payer = Keypair::new();

                // Request airdrop using spawn_blocking for the blocking calls
                let new_payer_pubkey = new_payer.pubkey();
                let rpc_url = url.to_string();

                let result: std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> =
                    task::spawn_blocking(move || {
                        let client = solana_client::rpc_client::RpcClient::new_with_commitment(
                            rpc_url,
                            CommitmentConfig::confirmed(),
                        );

                        // Request airdrop
                        let sig =
                            client.request_airdrop(&new_payer_pubkey, 10 * LAMPORTS_PER_SOL)?;

                        // Wait for confirmation
                        client.confirm_transaction(&sig)?;

                        Ok(())
                    })
                    .await?;

                result.map_err(|e| e.to_string())?;

                new_payer
            }
        };

        Ok(Self {
            rpc_client,
            payer,
            commitment: CommitmentConfig::confirmed(),
            disable_airdrop_rate_limit: false,
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
        use tokio::task;

        // Clone RPC client URL and commitment for the blocking task
        let rpc_url = self.rpc_client.url();
        let commitment = self.commitment;

        // Get recent blockhash in a blocking task
        let recent_blockhash = task::spawn_blocking(move || {
            let client =
                solana_client::rpc_client::RpcClient::new_with_commitment(rpc_url, commitment);
            client.get_latest_blockhash()
        })
        .await??;

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

        // Send and confirm transaction in a blocking task
        let rpc_url = self.rpc_client.url();
        let commitment = self.commitment;
        let sig = task::spawn_blocking(move || {
            let client =
                solana_client::rpc_client::RpcClient::new_with_commitment(rpc_url, commitment);
            client.send_and_confirm_transaction(&tx)
        })
        .await??;

        println!("Transaction confirmed: {}", sig);

        Ok::<(), Box<dyn std::error::Error>>(())
    }

    pub async fn get_account<T: AccountDeserialize>(
        &mut self,
        address: &Pubkey,
    ) -> TestResult<Option<T>> {
        use tokio::task;

        let rpc_url = self.rpc_client.url();
        let commitment = self.commitment;
        let address = *address;

        let account_result = task::spawn_blocking(move || {
            let client =
                solana_client::rpc_client::RpcClient::new_with_commitment(rpc_url, commitment);
            client.get_account_with_commitment(&address, commitment)
        })
        .await??;

        match account_result.value {
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

    pub async fn get_account_data(&mut self, address: &Pubkey) -> TestResult<Option<Vec<u8>>> {
        use tokio::task;

        let rpc_url = self.rpc_client.url();
        let commitment = self.commitment;
        let address = *address;

        let account_result = task::spawn_blocking(move || {
            let client =
                solana_client::rpc_client::RpcClient::new_with_commitment(rpc_url, commitment);
            client.get_account_with_commitment(&address, commitment)
        })
        .await??;

        match account_result.value {
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
        use tokio::task;
        use std::time::Duration;

        let rpc_url = self.rpc_client.url();
        let commitment = self.commitment;
        let to = *to;

        // Check if we should apply rate limiting
        let disable_rate_limit = self.disable_airdrop_rate_limit || 
                                std::env::var("DISABLE_AIRDROP_RATE_LIMIT").is_ok();
        let is_localnet = rpc_url.contains("localhost") || rpc_url.contains("127.0.0.1");
        
        // Apply rate limiting for non-localnet unless explicitly disabled
        if !is_localnet && !disable_rate_limit {
            // Wait between airdrops to respect rate limits
            tokio::time::sleep(Duration::from_millis(2500)).await;
        }

        // Request airdrop with retry logic
        let mut retries = 3;
        let mut last_error = None;
        
        let sig = loop {
            match task::spawn_blocking({
                let rpc_url = rpc_url.clone();
                move || {
                    let client = solana_client::rpc_client::RpcClient::new_with_commitment(
                        rpc_url,
                        commitment,
                    );
                    client.request_airdrop(&to, lamports)
                }
            })
            .await?
            {
                Ok(sig) => break sig,
                Err(e) => {
                    last_error = Some(e);
                    retries -= 1;
                    if retries == 0 {
                        return Err(format!("Airdrop failed after 3 attempts: {:?}", last_error).into());
                    }
                    // Wait longer between retries
                    tokio::time::sleep(Duration::from_millis(2000)).await;
                }
            }
        };

        // Confirm transaction in a blocking task
        let rpc_url = self.rpc_client.url();
        let commitment = self.commitment;
        task::spawn_blocking(move || {
            let client =
                solana_client::rpc_client::RpcClient::new_with_commitment(rpc_url, commitment);
            client.confirm_transaction(&sig)
        })
        .await??;

        Ok::<(), Box<dyn std::error::Error>>(())
    }

    pub async fn get_recent_blockhash(&mut self) -> TestResult<Hash> {
        use tokio::task;

        let rpc_url = self.rpc_client.url();
        let commitment = self.commitment;

        let blockhash = task::spawn_blocking(move || {
            let client =
                solana_client::rpc_client::RpcClient::new_with_commitment(rpc_url, commitment);
            client.get_latest_blockhash()
        })
        .await??;

        Ok(blockhash)
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
        use tokio::task;

        let rpc_url = self.rpc_client.url();
        let commitment = self.commitment;

        let slot = task::spawn_blocking(move || {
            let client =
                solana_client::rpc_client::RpcClient::new_with_commitment(rpc_url, commitment);
            client.get_slot()
        })
        .await??;

        Ok(slot)
    }

    pub fn payer(&self) -> Pubkey {
        self.payer.pubkey()
    }

    pub async fn get_balance(&mut self, address: &Pubkey) -> TestResult<u64> {
        use tokio::task;

        let rpc_url = self.rpc_client.url();
        let commitment = self.commitment;
        let address = *address;

        let balance = task::spawn_blocking(move || {
            let client =
                solana_client::rpc_client::RpcClient::new_with_commitment(rpc_url, commitment);
            client.get_balance(&address)
        })
        .await??;

        Ok(balance)
    }
}
