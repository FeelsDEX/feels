#![cfg(test)]

mod tests {
    use anchor_lang::prelude::*;
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Signer};
    use solana_program_test::{tokio, ProgramTest};
    use feels::instructions::InitializeProtocolParams;

    #[tokio::test]
    async fn test_initialize_protocol_directly() {
        // Set BPF_OUT_DIR if not already set
        if std::env::var("BPF_OUT_DIR").is_err() {
            let possible_paths = vec![
                "target/deploy",
                "../target/deploy",
                "../../target/deploy",
                "../../../target/deploy",
            ];
            
            for path in possible_paths {
                if std::path::Path::new(path).exists() {
                    std::env::set_var("BPF_OUT_DIR", path);
                    break;
                }
            }
        }
        
        // Create a fresh program test
        let mut pt = ProgramTest::new(
            "feels",
            feels::id(),
            None, // Load from BPF
        );
        
        let (mut banks_client, payer, recent_blockhash) = pt.start().await;
        
        // Build initialize_protocol instruction manually
        let (protocol_config, _) = Pubkey::find_program_address(
            &[b"protocol_config"],
            &feels::id(),
        );
        
        // Create params
        let params = InitializeProtocolParams {
            mint_fee: 0,
            treasury: payer.pubkey(),
        };
        
        // Create instruction data manually
        let discriminator: [u8; 8] = [0xbc, 0xe9, 0xfc, 0x6a, 0x86, 0x92, 0xca, 0x5b];
        let mut data = discriminator.to_vec();
        data.extend_from_slice(&params.try_to_vec().unwrap());
        
        let instruction = Instruction {
            program_id: feels::id(),
            accounts: vec![
                solana_sdk::instruction::AccountMeta::new(payer.pubkey(), true),
                solana_sdk::instruction::AccountMeta::new(protocol_config, false),
                solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
            data,
        };
        
        // Create and send transaction
        let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        
        match banks_client.process_transaction(tx).await {
            Ok(()) => println!("Success!"),
            Err(e) => println!("Error: {:?}", e),
        }
    }
}