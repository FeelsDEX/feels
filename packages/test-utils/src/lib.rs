use anchor_lang::prelude::*;
use solana_address::Address;
use solana_program_test::*;
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction as SdkInstruction},
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use std::fs;

pub struct TestContext {
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub recent_blockhash: Hash,
}

impl TestContext {
    /// Create a new test context with a program loaded from the specified path
    pub async fn new_with_program(program_id: Pubkey, program_path: &str) -> Self {
        let program_id = Address::new_from_array(program_id.to_bytes());
        let program_data = fs::read(program_path)
            .expect("Failed to read program file. Make sure to run 'anchor build' first");

        let mut program_test = ProgramTest::default();
        program_test.add_account(
            program_id,
            solana_sdk::account::Account {
                lamports: 1_000_000,
                data: program_data,
                owner: solana_sdk::bpf_loader::id(),
                executable: true,
                rent_epoch: 0,
            },
        );

        let (banks_client, payer, recent_blockhash) = program_test.start().await;
        Self {
            banks_client,
            payer,
            recent_blockhash,
        }
    }

    /// Create a new test context with multiple programs
    pub async fn new_with_programs(programs: Vec<(Pubkey, &str)>) -> Self {
        let mut program_test = ProgramTest::default();

        for (program_id, program_path) in programs {
            let program_id = Address::new_from_array(program_id.to_bytes());
            let program_data = fs::read(program_path)
                .unwrap_or_else(|_| panic!("Failed to read program file: {program_path}"));

            program_test.add_account(
                program_id,
                solana_sdk::account::Account {
                    lamports: 1_000_000,
                    data: program_data,
                    owner: solana_sdk::bpf_loader::id(),
                    executable: true,
                    rent_epoch: 0,
                },
            );
        }

        let (banks_client, payer, recent_blockhash) = program_test.start().await;
        Self {
            banks_client,
            payer,
            recent_blockhash,
        }
    }

    /// Create a basic test context without any programs (for testing with deployed programs)
    pub async fn new_basic() -> Self {
        let program_test = ProgramTest::default();
        let (banks_client, payer, recent_blockhash) = program_test.start().await;
        Self {
            banks_client,
            payer,
            recent_blockhash,
        }
    }

    pub fn payer_pubkey(&self) -> Pubkey {
        Pubkey::new_from_array(self.payer.pubkey().to_bytes())
    }

    pub async fn process_instruction(
        &mut self,
        instruction: SdkInstruction,
    ) -> std::result::Result<(), BanksClientError> {
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.recent_blockhash,
        );

        self.banks_client.process_transaction(transaction).await
    }

    pub async fn process_instructions(
        &mut self,
        instructions: Vec<SdkInstruction>,
    ) -> std::result::Result<(), BanksClientError> {
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.recent_blockhash,
        );

        self.banks_client.process_transaction(transaction).await
    }

    pub async fn get_account_data<T: anchor_lang::AccountDeserialize>(
        &mut self,
        address: Pubkey,
    ) -> Result<T> {
        let address = Address::new_from_array(address.to_bytes());
        let account = self
            .banks_client
            .get_account(address)
            .await
            .unwrap()
            .unwrap();
        T::try_deserialize(&mut account.data.as_slice())
    }

    pub async fn get_account(&mut self, address: Pubkey) -> Option<solana_sdk::account::Account> {
        let address = Address::new_from_array(address.to_bytes());
        self.banks_client.get_account(address).await.unwrap()
    }
}

/// Utility to convert Anchor Instruction to SDK Instruction
pub fn to_sdk_instruction(
    instruction: anchor_lang::solana_program::instruction::Instruction,
) -> SdkInstruction {
    SdkInstruction {
        program_id: solana_sdk::pubkey::Pubkey::new_from_array(instruction.program_id.to_bytes()),
        accounts: instruction
            .accounts
            .iter()
            .map(|acc| AccountMeta {
                pubkey: solana_sdk::pubkey::Pubkey::new_from_array(acc.pubkey.to_bytes()),
                is_signer: acc.is_signer,
                is_writable: acc.is_writable,
            })
            .collect(),
        data: instruction.data,
    }
}

/// Convert multiple Anchor instructions to SDK instructions
pub fn to_sdk_instructions(
    instructions: Vec<anchor_lang::solana_program::instruction::Instruction>,
) -> Vec<SdkInstruction> {
    instructions.into_iter().map(to_sdk_instruction).collect()
}
