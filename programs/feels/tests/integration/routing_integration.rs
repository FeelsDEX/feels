/// Integration tests for hub-constrained routing flows
use anchor_lang::prelude::*;
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token_2022::{
    extension::ExtensionType,
    state::{Account as TokenAccount, Mint},
};

use feels::{
    accounts as feels_accounts,
    instruction as feels_instruction,
    state::FeelsSOL,
    error::FeelsError,
};

#[tokio::test]
async fn test_entry_exit_flow() {
    let program_id = feels::ID;
    let mut program_test = ProgramTest::new("feels", program_id, None);

    // Add accounts
    let authority = Keypair::new();
    let user = Keypair::new();
    
    // Create mints
    let jitosol_mint = Keypair::new();
    let feelssol_mint = Keypair::new();
    
    // Start test
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Initialize JitoSOL mint
    // ... mint initialization code ...
    
    // Initialize FeelsSOL
    let (feelssol_pda, _) = Pubkey::find_program_address(&[b"feelssol"], &program_id);
    let (vault_pda, _) = Pubkey::find_program_address(
        &[b"feelssol_vault", jitosol_mint.pubkey().as_ref()],
        &program_id
    );
    
    // Test entry (JitoSOL -> FeelsSOL)
    let entry_amount = 1_000_000_000; // 1 JitoSOL
    
    // Create user token accounts
    // ... token account creation ...
    
    // Execute entry
    let entry_ix = feels_instruction::enter_system(
        feels_accounts::EntryExit {
            user: user.pubkey(),
            user_jitosol: user_jitosol_ata,
            user_feelssol: user_feelssol_ata,
            jitosol_mint: jitosol_mint.pubkey(),
            feelssol: feelssol_pda,
            feelssol_vault: vault_pda,
            feelssol_mint: feelssol_mint.pubkey(),
            token_program: spl_token_2022::ID,
            system_program: solana_program::system_program::ID,
        },
        feels::EntryParams {
            amount_in: entry_amount,
            min_amount_out: entry_amount * 99 / 100, // 1% slippage
        },
    );
    
    let mut transaction = Transaction::new_with_payer(&[entry_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &user], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok(), "Entry transaction failed: {:?}", result);
    
    // Verify balances
    // ... balance verification ...
    
    // Test exit (FeelsSOL -> JitoSOL)
    let exit_ix = feels_instruction::exit_system(
        feels_accounts::EntryExit {
            user: user.pubkey(),
            user_jitosol: user_jitosol_ata,
            user_feelssol: user_feelssol_ata,
            jitosol_mint: jitosol_mint.pubkey(),
            feelssol: feelssol_pda,
            feelssol_vault: vault_pda,
            feelssol_mint: feelssol_mint.pubkey(),
            token_program: spl_token_2022::ID,
            system_program: solana_program::system_program::ID,
        },
        feels::ExitParams {
            amount_in: entry_amount,
            min_amount_out: entry_amount * 99 / 100,
        },
    );
    
    let mut transaction = Transaction::new_with_payer(&[exit_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &user], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok(), "Exit transaction failed: {:?}", result);
}

#[tokio::test]
async fn test_pool_initialization_hub_constraint() {
    let program_id = feels::ID;
    let mut program_test = ProgramTest::new("feels", program_id, None);
    
    let authority = Keypair::new();
    let feelssol_mint = Keypair::new();
    let token_a = Keypair::new();
    let token_b = Keypair::new();
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Initialize FeelsSOL
    let (feelssol_pda, _) = Pubkey::find_program_address(&[b"feelssol"], &program_id);
    
    // Test 1: Valid pool with FeelsSOL
    let valid_pool_ix = feels_instruction::initialize_market(
        feels_accounts::InitializeMarket {
            market_field: Keypair::new().pubkey(),
            buffer_account: Keypair::new().pubkey(),
            token_0_mint: feelssol_mint.pubkey(),
            token_1_mint: token_a.pubkey(),
            feelssol: feelssol_pda,
            token_0_vault: Keypair::new().pubkey(),
            token_1_vault: Keypair::new().pubkey(),
            twap_oracle: Keypair::new().pubkey(),
            market_data_source: Keypair::new().pubkey(),
            protocol_state: Keypair::new().pubkey(),
            authority: authority.pubkey(),
            token_program: spl_token_2022::ID,
            associated_token_program: spl_associated_token_account::ID,
            system_program: solana_program::system_program::ID,
            rent: solana_program::sysvar::rent::ID,
        },
        feels::InitializeMarketParams::default(),
    );
    
    let mut transaction = Transaction::new_with_payer(&[valid_pool_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &authority], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok(), "Valid pool initialization failed");
    
    // Test 2: Invalid pool without FeelsSOL (should fail)
    let invalid_pool_ix = feels_instruction::initialize_market(
        feels_accounts::InitializeMarket {
            market_field: Keypair::new().pubkey(),
            buffer_account: Keypair::new().pubkey(),
            token_0_mint: token_a.pubkey(),
            token_1_mint: token_b.pubkey(),
            feelssol: feelssol_pda,
            token_0_vault: Keypair::new().pubkey(),
            token_1_vault: Keypair::new().pubkey(),
            twap_oracle: Keypair::new().pubkey(),
            market_data_source: Keypair::new().pubkey(),
            protocol_state: Keypair::new().pubkey(),
            authority: authority.pubkey(),
            token_program: spl_token_2022::ID,
            associated_token_program: spl_associated_token_account::ID,
            system_program: solana_program::system_program::ID,
            rent: solana_program::sysvar::rent::ID,
        },
        feels::InitializeMarketParams::default(),
    );
    
    let mut transaction = Transaction::new_with_payer(&[invalid_pool_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &authority], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err(), "Invalid pool initialization should have failed");
}

#[tokio::test]
async fn test_two_hop_swap_flow() {
    let program_id = feels::ID;
    let mut program_test = ProgramTest::new("feels", program_id, None);
    
    let user = Keypair::new();
    let feelssol_mint = Keypair::new();
    let usdc_mint = Keypair::new();
    let sol_mint = Keypair::new();
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Setup: Create two pools
    // Pool 1: USDC-FeelsSOL
    // Pool 2: FeelsSOL-SOL
    
    // Execute two-hop swap: USDC -> FeelsSOL -> SOL
    let amount_in = 1_000_000; // 1 USDC
    
    // First hop: USDC -> FeelsSOL
    let swap1_ix = feels_instruction::order_unified(
        feels_accounts::Order {
            market_field: usdc_feelssol_pool,
            market_manager: usdc_feelssol_manager,
            buffer_account: usdc_feelssol_buffer,
            user: user.pubkey(),
            user_token_0: user_usdc_ata,
            user_token_1: user_feelssol_ata,
            market_token_0: pool_usdc_vault,
            market_token_1: pool_feelssol_vault,
            token_program: spl_token_2022::ID,
            system_program: solana_program::system_program::ID,
            tick_array_router: None,
        },
        feels::OrderParams::Create(feels::CreateOrderParams {
            order_type: feels::OrderType::Immediate,
            amount: amount_in,
            rate_params: feels::RateParams::TargetRate {
                sqrt_rate_limit: u128::MAX,
                direction: feels::SwapDirection::BuyExactIn,
            },
            duration: feels::Duration::Swap,
            leverage: 1_000_000, // 1.0x
            max_slippage_bps: 100,
        }),
    );
    
    // Second hop: FeelsSOL -> SOL
    let swap2_ix = feels_instruction::order_unified(
        feels_accounts::Order {
            market_field: feelssol_sol_pool,
            market_manager: feelssol_sol_manager,
            buffer_account: feelssol_sol_buffer,
            user: user.pubkey(),
            user_token_0: user_feelssol_ata,
            user_token_1: user_sol_ata,
            market_token_0: pool_feelssol_vault2,
            market_token_1: pool_sol_vault,
            token_program: spl_token_2022::ID,
            system_program: solana_program::system_program::ID,
            tick_array_router: None,
        },
        feels::OrderParams::Create(feels::CreateOrderParams {
            order_type: feels::OrderType::Immediate,
            amount: 0, // Will be determined by first hop
            rate_params: feels::RateParams::TargetRate {
                sqrt_rate_limit: u128::MAX,
                direction: feels::SwapDirection::BuyExactIn,
            },
            duration: feels::Duration::Swap,
            leverage: 1_000_000,
            max_slippage_bps: 100,
        }),
    );
    
    // Execute both in single transaction
    let mut transaction = Transaction::new_with_payer(
        &[swap1_ix, swap2_ix], 
        Some(&payer.pubkey())
    );
    transaction.sign(&[&payer, &user], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok(), "Two-hop swap failed: {:?}", result);
    
    // Verify final balances
    // User should have less USDC and more SOL
    // FeelsSOL balance should be unchanged (intermediate)
}

#[tokio::test]
async fn test_position_flow_integration() {
    let program_id = feels::ID;
    let mut program_test = ProgramTest::new("feels", program_id, None);
    
    let user = Keypair::new();
    let feelssol_mint = Keypair::new();
    let time_position_mint = Keypair::new();
    let leverage_position_mint = Keypair::new();
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Test 1: Enter position (FeelsSOL -> Time Position)
    let enter_ix = feels_instruction::enter_position(
        feels_accounts::EnterPosition {
            user: user.pubkey(),
            user_feelssol: user_feelssol_ata,
            user_position: user_time_position_ata,
            feelssol: feelssol_pda,
            market_field: time_market_field,
            position_mint: time_position_mint.pubkey(),
            market_vault: time_market_vault,
            token_program: spl_token_2022::ID,
            associated_token_program: spl_associated_token_account::ID,
            system_program: solana_program::system_program::ID,
        },
        feels::EnterPositionParams {
            amount_in: 1_000_000_000,
            position_type: feels::PositionType::Time { 
                duration: feels::Duration::Weekly 
            },
            min_position_tokens: 990_000_000,
        },
    );
    
    let mut transaction = Transaction::new_with_payer(&[enter_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &user], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok(), "Enter position failed");
    
    // Test 2: Convert position (Time -> Leverage via FeelsSOL)
    let convert_ix = feels_instruction::convert_position(
        feels_accounts::ConvertPosition {
            user: user.pubkey(),
            user_source_position: user_time_position_ata,
            user_dest_position: user_leverage_position_ata,
            user_feelssol: user_feelssol_ata,
            feelssol: feelssol_pda,
            source_market_field: time_market_field,
            dest_market_field: leverage_market_field,
            source_position_mint: time_position_mint.pubkey(),
            dest_position_mint: leverage_position_mint.pubkey(),
            source_market_vault: time_market_vault,
            dest_market_vault: leverage_market_vault,
            token_program: spl_token_2022::ID,
            associated_token_program: spl_associated_token_account::ID,
            system_program: solana_program::system_program::ID,
        },
        feels::ConvertPositionParams {
            amount_in: 500_000_000,
            target_position_type: feels::PositionType::Leverage {
                risk_profile: feels::RiskProfile::default(),
            },
            min_position_tokens_out: 490_000_000,
        },
    );
    
    let mut transaction = Transaction::new_with_payer(&[convert_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &user], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok(), "Convert position failed");
    
    // Test 3: Exit position (Leverage -> FeelsSOL)
    let exit_ix = feels_instruction::exit_position(
        feels_accounts::ExitPosition {
            user: user.pubkey(),
            user_position: user_leverage_position_ata,
            user_feelssol: user_feelssol_ata,
            feelssol: feelssol_pda,
            market_field: leverage_market_field,
            position_mint: leverage_position_mint.pubkey(),
            market_vault: leverage_market_vault,
            token_program: spl_token_2022::ID,
            system_program: solana_program::system_program::ID,
        },
        feels::ExitPositionParams {
            amount_in: 490_000_000,
            min_feelssol_out: 480_000_000,
        },
    );
    
    let mut transaction = Transaction::new_with_payer(&[exit_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &user], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok(), "Exit position failed");
}