/// Test for V136: Missing Mint Authority Transfer Validation
/// Verifies that the mint authority and freeze authority are properly
/// validated after initialization to ensure protocol control.

#[cfg(test)]
mod test_v136_mint_authority_validation {
    use anchor_lang::prelude::*;
    use anchor_spl::token_2022::{self, Token2022};
    use solana_program_test::*;
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use crate::instructions::initialize_feelssol;
    
    #[tokio::test]
    async fn test_mint_authority_validation_success() {
        let program_test = ProgramTest::new(
            "feels",
            crate::ID,
            processor!(crate::entry),
        );
        
        let mut context = program_test.start_with_context().await;
        
        // Create mint account
        let mint_keypair = Keypair::new();
        let feelssol_pda = Pubkey::find_program_address(
            &[b"feelssol"],
            &crate::ID,
        ).0;
        
        // Create mint with feelssol PDA as authority
        let rent = context.banks_client.get_rent().await.unwrap();
        let mint_rent = rent.minimum_balance(token_2022::Mint::LEN);
        
        let create_mint_ix = solana_program::system_instruction::create_account(
            &context.payer.pubkey(),
            &mint_keypair.pubkey(),
            mint_rent,
            token_2022::Mint::LEN as u64,
            &token_2022::ID,
        );
        
        let init_mint_ix = token_2022::instruction::initialize_mint2(
            &token_2022::ID,
            &mint_keypair.pubkey(),
            &feelssol_pda, // Correct authority
            Some(&feelssol_pda), // Correct freeze authority
            9,
        ).unwrap();
        
        let mut transaction = Transaction::new_with_payer(
            &[create_mint_ix, init_mint_ix],
            Some(&context.payer.pubkey()),
        );
        transaction.sign(&[&context.payer, &mint_keypair], context.last_blockhash);
        context.banks_client.process_transaction(transaction).await.unwrap();
        
        // Now test initialize_feelssol - should succeed
        let initialize_ix = crate::instruction::InitializeFeelssol {
            underlying_mint: Pubkey::new_unique(), // Some SOL-like mint
        };
        
        let accounts = crate::accounts::InitializeFeelssol {
            feelssol: feelssol_pda,
            feels_mint: mint_keypair.pubkey(),
            authority: context.payer.pubkey(),
            system_program: solana_program::system_program::ID,
            token_program: token_2022::ID,
        };
        
        // This should succeed because mint authorities are correct
        let result = initialize_feelssol::handler(
            Context::new(
                &crate::ID,
                &mut accounts.to_account_metas(None),
                &[],
                context.remaining_accounts,
            ),
            initialize_ix.underlying_mint,
        );
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_mint_authority_validation_fails_wrong_authority() {
        let program_test = ProgramTest::new(
            "feels",
            crate::ID,
            processor!(crate::entry),
        );
        
        let mut context = program_test.start_with_context().await;
        
        // Create mint account
        let mint_keypair = Keypair::new();
        let feelssol_pda = Pubkey::find_program_address(
            &[b"feelssol"],
            &crate::ID,
        ).0;
        
        // Create mint with WRONG authority (not feelssol PDA)
        let wrong_authority = Keypair::new();
        let rent = context.banks_client.get_rent().await.unwrap();
        let mint_rent = rent.minimum_balance(token_2022::Mint::LEN);
        
        let create_mint_ix = solana_program::system_instruction::create_account(
            &context.payer.pubkey(),
            &mint_keypair.pubkey(),
            mint_rent,
            token_2022::Mint::LEN as u64,
            &token_2022::ID,
        );
        
        let init_mint_ix = token_2022::instruction::initialize_mint2(
            &token_2022::ID,
            &mint_keypair.pubkey(),
            &wrong_authority.pubkey(), // Wrong mint authority!
            Some(&feelssol_pda), // Correct freeze authority
            9,
        ).unwrap();
        
        let mut transaction = Transaction::new_with_payer(
            &[create_mint_ix, init_mint_ix],
            Some(&context.payer.pubkey()),
        );
        transaction.sign(&[&context.payer, &mint_keypair], context.last_blockhash);
        context.banks_client.process_transaction(transaction).await.unwrap();
        
        // Now test initialize_feelssol - should fail
        let initialize_ix = crate::instruction::InitializeFeelssol {
            underlying_mint: Pubkey::new_unique(),
        };
        
        let accounts = crate::accounts::InitializeFeelssol {
            feelssol: feelssol_pda,
            feels_mint: mint_keypair.pubkey(),
            authority: context.payer.pubkey(),
            system_program: solana_program::system_program::ID,
            token_program: token_2022::ID,
        };
        
        // This should fail because mint authority is wrong
        let result = initialize_feelssol::handler(
            Context::new(
                &crate::ID,
                &mut accounts.to_account_metas(None),
                &[],
                context.remaining_accounts,
            ),
            initialize_ix.underlying_mint,
        );
        
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            crate::state::FeelsError::Unauthorized.into()
        );
    }
    
    #[tokio::test]
    async fn test_freeze_authority_validation_fails() {
        let program_test = ProgramTest::new(
            "feels",
            crate::ID,
            processor!(crate::entry),
        );
        
        let mut context = program_test.start_with_context().await;
        
        // Create mint account
        let mint_keypair = Keypair::new();
        let feelssol_pda = Pubkey::find_program_address(
            &[b"feelssol"],
            &crate::ID,
        ).0;
        
        // Create mint with correct mint authority but WRONG freeze authority
        let wrong_freeze_authority = Keypair::new();
        let rent = context.banks_client.get_rent().await.unwrap();
        let mint_rent = rent.minimum_balance(token_2022::Mint::LEN);
        
        let create_mint_ix = solana_program::system_instruction::create_account(
            &context.payer.pubkey(),
            &mint_keypair.pubkey(),
            mint_rent,
            token_2022::Mint::LEN as u64,
            &token_2022::ID,
        );
        
        let init_mint_ix = token_2022::instruction::initialize_mint2(
            &token_2022::ID,
            &mint_keypair.pubkey(),
            &feelssol_pda, // Correct mint authority
            Some(&wrong_freeze_authority.pubkey()), // Wrong freeze authority!
            9,
        ).unwrap();
        
        let mut transaction = Transaction::new_with_payer(
            &[create_mint_ix, init_mint_ix],
            Some(&context.payer.pubkey()),
        );
        transaction.sign(&[&context.payer, &mint_keypair], context.last_blockhash);
        context.banks_client.process_transaction(transaction).await.unwrap();
        
        // Now test initialize_feelssol - should fail
        let initialize_ix = crate::instruction::InitializeFeelssol {
            underlying_mint: Pubkey::new_unique(),
        };
        
        let accounts = crate::accounts::InitializeFeelssol {
            feelssol: feelssol_pda,
            feels_mint: mint_keypair.pubkey(),
            authority: context.payer.pubkey(),
            system_program: solana_program::system_program::ID,
            token_program: token_2022::ID,
        };
        
        // This should fail because freeze authority is wrong
        let result = initialize_feelssol::handler(
            Context::new(
                &crate::ID,
                &mut accounts.to_account_metas(None),
                &[],
                context.remaining_accounts,
            ),
            initialize_ix.underlying_mint,
        );
        
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            crate::state::FeelsError::Unauthorized.into()
        );
    }
    
    #[test]
    fn test_validation_logic_unit() {
        // Unit test for the validation logic itself
        use anchor_spl::token_2022::spl_token_2022::state::Mint;
        
        let feelssol_pda = Pubkey::find_program_address(
            &[b"feelssol"],
            &crate::ID,
        ).0;
        
        // Create mock mint data with correct authorities
        let mut mint_data = vec![0u8; Mint::LEN];
        let mut mint = Mint::unpack_unchecked(&mint_data).unwrap();
        mint.mint_authority = Some(feelssol_pda).into();
        mint.freeze_authority = Some(feelssol_pda).into();
        Mint::pack(mint, &mut mint_data).unwrap();
        
        // This represents the validation in initialize_feelssol
        let mint = Mint::unpack(&mint_data).unwrap();
        assert_eq!(mint.mint_authority.unwrap(), feelssol_pda);
        assert_eq!(mint.freeze_authority.unwrap(), feelssol_pda);
        
        // Test with wrong mint authority
        let mut wrong_mint_data = vec![0u8; Mint::LEN];
        let mut wrong_mint = Mint::unpack_unchecked(&wrong_mint_data).unwrap();
        wrong_mint.mint_authority = Some(Pubkey::new_unique()).into();
        wrong_mint.freeze_authority = Some(feelssol_pda).into();
        Mint::pack(wrong_mint, &mut wrong_mint_data).unwrap();
        
        let wrong_mint = Mint::unpack(&wrong_mint_data).unwrap();
        assert_ne!(wrong_mint.mint_authority.unwrap(), feelssol_pda);
        
        // Test with wrong freeze authority
        let mut wrong_freeze_data = vec![0u8; Mint::LEN];
        let mut wrong_freeze = Mint::unpack_unchecked(&wrong_freeze_data).unwrap();
        wrong_freeze.mint_authority = Some(feelssol_pda).into();
        wrong_freeze.freeze_authority = Some(Pubkey::new_unique()).into();
        Mint::pack(wrong_freeze, &mut wrong_freeze_data).unwrap();
        
        let wrong_freeze = Mint::unpack(&wrong_freeze_data).unwrap();
        assert_ne!(wrong_freeze.freeze_authority.unwrap(), feelssol_pda);
    }
}