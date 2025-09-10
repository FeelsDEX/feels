//! Test for initialize vaults instruction

#[cfg(test)]
mod tests {
    use anchor_lang::prelude::*;
    use anchor_lang::{InstructionData, ToAccountMetas};
    use solana_sdk::{
        program_pack::Pack,
        instruction::Instruction,
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use solana_program_test::{tokio, BanksClient, ProgramTest};
    use spl_token::state::Account as TokenAccount;
    
    use feels::{
        constants::{VAULT_SEED, MARKET_AUTHORITY_SEED},
        state::Market,
    };

    async fn setup() -> (BanksClient, Keypair, Pubkey) {
        let program_test = ProgramTest::new("feels", feels::ID, None);
        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        
        (banks_client, payer, feels::ID)
    }

    #[tokio::test]
    async fn test_initialize_vaults() {
        let (mut banks_client, payer, program_id) = setup().await;
        
        // Create token mints
        let token_0 = Keypair::new();
        let token_1 = Keypair::new();
        
        // Ensure proper ordering
        let (token_0, token_1) = if token_0.pubkey() < token_1.pubkey() {
            (token_0, token_1)
        } else {
            (token_1, token_0)
        };
        
        // Create token mints
        let mint_ix_0 = spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &token_0.pubkey(),
            &payer.pubkey(),
            None,
            6,
        ).unwrap();
        
        let mint_ix_1 = spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &token_1.pubkey(),
            &payer.pubkey(),
            None,
            6,
        ).unwrap();
        
        // Create token mint accounts
        let rent = banks_client.get_rent().await.unwrap();
        let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);
        
        let create_token_0_ix = solana_sdk::system_instruction::create_account(
            &payer.pubkey(),
            &token_0.pubkey(),
            mint_rent,
            spl_token::state::Mint::LEN as u64,
            &spl_token::id(),
        );
        
        let create_token_1_ix = solana_sdk::system_instruction::create_account(
            &payer.pubkey(),
            &token_1.pubkey(),
            mint_rent,
            spl_token::state::Mint::LEN as u64,
            &spl_token::id(),
        );
        
        // Initialize mints
        let tx = Transaction::new_signed_with_payer(
            &[create_token_0_ix, mint_ix_0, create_token_1_ix, mint_ix_1],
            Some(&payer.pubkey()),
            &[&payer, &token_0, &token_1],
            banks_client.get_recent_blockhash().await.unwrap(),
        );
        
        banks_client.process_transaction(tx).await.unwrap();
        
        // Derive market PDA
        let (market_pda, _) = Pubkey::find_program_address(
            &[b"market", token_0.pubkey().as_ref(), token_1.pubkey().as_ref()],
            &program_id,
        );
        
        // Initialize market first
        let market_accounts = feels::accounts::InitializeMarket {
            authority: payer.pubkey(),
            market: market_pda,
            token_0: token_0.pubkey(),
            token_1: token_1.pubkey(),
            system_program: solana_sdk::system_program::id(),
            token_program: spl_token::id(),
        };
        
        let init_market_ix = Instruction {
            program_id,
            accounts: market_accounts.to_account_metas(None),
            data: feels::instruction::InitializeMarket {
                base_fee_bps: 25,
                tick_spacing: 64,
                initial_sqrt_price: 79228162514264337593543950336u128, // 1:1 price
            }.data(),
        };
        
        let tx = Transaction::new_signed_with_payer(
            &[init_market_ix],
            Some(&payer.pubkey()),
            &[&payer],
            banks_client.get_recent_blockhash().await.unwrap(),
        );
        
        banks_client.process_transaction(tx).await.unwrap();
        
        // Derive vault PDAs
        let (vault_0, _) = Pubkey::find_program_address(
            &[VAULT_SEED, market_pda.as_ref(), token_0.pubkey().as_ref()],
            &program_id,
        );
        
        let (vault_1, _) = Pubkey::find_program_address(
            &[VAULT_SEED, market_pda.as_ref(), token_1.pubkey().as_ref()],
            &program_id,
        );
        
        // Derive market authority
        let (market_authority, _) = Pubkey::find_program_address(
            &[MARKET_AUTHORITY_SEED, market_pda.as_ref()],
            &program_id,
        );
        
        // Create initialize vaults instruction
        let accounts = feels::accounts::InitializeVaults {
            market: market_pda,
            authority: payer.pubkey(),
            base_mint: token_0.pubkey(),
            quote_mint: token_1.pubkey(),
            base_vault: vault_0,
            quote_vault: vault_1,
            market_authority,
            system_program: solana_sdk::system_program::id(),
            token_program: spl_token::id(),
        };
        
        let ix = Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: feels::instruction::InitializeVaults {}.data(),
        };
        
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            banks_client.get_recent_blockhash().await.unwrap(),
        );
        
        // Execute transaction
        banks_client.process_transaction(tx).await.unwrap();
        
        // Verify vaults were created
        let vault_0_account = banks_client.get_account(vault_0).await.unwrap().unwrap();
        let vault_1_account = banks_client.get_account(vault_1).await.unwrap().unwrap();
        
        // Vaults should exist and be owned by the token program
        assert_eq!(vault_0_account.owner, spl_token::id());
        assert_eq!(vault_1_account.owner, spl_token::id());
        
        // Verify vault authorities are set to market authority PDA
        let vault_0_data = TokenAccount::unpack(&vault_0_account.data).unwrap();
        let vault_1_data = TokenAccount::unpack(&vault_1_account.data).unwrap();
        
        assert_eq!(vault_0_data.owner, market_authority);
        assert_eq!(vault_1_data.owner, market_authority);
    }
}