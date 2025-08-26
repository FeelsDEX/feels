use anchor_lang::prelude::*;
use solana_address::Address;
use solana_program_test::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction as SdkInstruction},
    signer::Signer,
    transaction::Transaction,
};
use std::fs;

pub struct TestApp {
    pub context: ProgramTestContext,
}

impl TestApp {
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

        let context = program_test.start_with_context().await;
        Self { context }
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

        let context = program_test.start_with_context().await;
        Self { context }
    }

    /// Create a basic test context without any programs (for testing with deployed programs)
    pub async fn new_basic() -> Self {
        let program_test = ProgramTest::default();
        let context = program_test.start_with_context().await;
        Self { context }
    }

    pub fn payer_pubkey(&self) -> Pubkey {
        Pubkey::new_from_array(self.context.payer.pubkey().to_bytes())
    }

    pub async fn process_instruction(
        &mut self,
        instruction: SdkInstruction,
    ) -> std::result::Result<(), BanksClientError> {
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.context.payer.pubkey()),
            &[&self.context.payer],
            self.context.last_blockhash,
        );

        self.context
            .banks_client
            .process_transaction(transaction)
            .await
    }

    pub async fn process_instructions(
        &mut self,
        instructions: Vec<SdkInstruction>,
    ) -> std::result::Result<(), BanksClientError> {
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.context.payer.pubkey()),
            &[&self.context.payer],
            self.context.last_blockhash,
        );

        self.context
            .banks_client
            .process_transaction(transaction)
            .await
    }

    pub async fn process_instruction_as_signer(
        &mut self,
        instruction: SdkInstruction,
        signer: &solana_sdk::signer::keypair::Keypair,
    ) -> std::result::Result<(), BanksClientError> {
        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&signer.pubkey()));
        transaction.sign(&[signer], self.context.last_blockhash);
        self.context
            .banks_client
            .process_transaction(transaction)
            .await
    }

    pub async fn get_account_data<T: anchor_lang::AccountDeserialize>(
        &mut self,
        address: Pubkey,
    ) -> Result<T> {
        let address = Address::new_from_array(address.to_bytes());
        let account = self
            .context
            .banks_client
            .get_account(address)
            .await
            .unwrap()
            .unwrap();
        T::try_deserialize(&mut account.data.as_slice())
    }

    pub async fn get_account(&mut self, address: Pubkey) -> Option<solana_sdk::account::Account> {
        let address = Address::new_from_array(address.to_bytes());
        self.context
            .banks_client
            .get_account(address)
            .await
            .unwrap()
    }

    pub async fn warp_forward_seconds(&mut self, seconds: i64) {
        // Get the current clock
        let mut clock: solana_clock::Clock = self.context.banks_client.get_sysvar().await.unwrap();

        // Advance the timestamp
        clock.unix_timestamp += seconds;

        // Set the updated clock back
        self.context.set_sysvar(&clock);

        // Also advance by at least one slot to ensure block progression
        let current_slot = self.context.banks_client.get_root_slot().await.unwrap();
        self.context.warp_to_slot(current_slot + 1).unwrap();
    }

    /// Warp to a specific slot
    pub async fn warp_to_slot(&mut self, slot: u64) {
        self.context.warp_to_slot(slot).unwrap();
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
