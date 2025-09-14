//! Tests for stair pattern liquidity deployment

use anchor_lang::prelude::*;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use feels::{
    state::*,
    instructions::{
        MintTokenParams, InitializeMarketParams, DeployInitialLiquidityParams,
    },
};

use crate::helpers::*;

#[tokio::test]
#[ignore = "Test needs updating for new architecture"]
async fn test_pool_stair_pattern_deployment() {
    let test = ProgramTest::new("feels", feels::id(), None);
    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    
    // Create token mints
    let feelssol_mint = create_mint(&mut banks_client, &payer, 9).await;
    let token_mint = Keypair::new();
    
    // Step 1: Mint a protocol token
    let mint_params = MintTokenParams {
        name: "Test Token".to_string(),
        ticker: "TEST".to_string(),
        uri: "https://test.com".to_string(),
    };
    
    // Create creator's FeelsSOL account for paying mint fee
    let creator_feelssol = create_token_account(&mut banks_client, &payer, feelssol_mint, payer.pubkey()).await;
    mint_to(&mut banks_client, &payer, feelssol_mint, creator_feelssol, 1_000_000_000).await; // Mint fee amount
    
    // Use SDK to create mint token instruction
    let mint_ix = feels_sdk::instructions::mint_token(
        payer.pubkey(),
        creator_feelssol,
        token_mint.pubkey(),
        feelssol_mint,
        mint_params,
    ).unwrap();
    
    let mut tx = Transaction::new_with_payer(&[mint_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &token_mint], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();
    
    // Step 2: Initialize market
    let initial_sqrt_price = 1u128 << 64; // Price = 1.0
    
    let init_market_params = InitializeMarketParams {
        base_fee_bps: 30,
        tick_spacing: 10,
        initial_sqrt_price,
        initial_buy_feelssol_amount: 0,
    };
    
    // Token order: feelssol < token_mint (feelssol is token_0)
    let (token_0, token_1) = if feelssol_mint < token_mint.pubkey() {
        (feelssol_mint, token_mint.pubkey())
    } else {
        (token_mint.pubkey(), feelssol_mint)
    };
    
    let init_market_ix = feels_sdk::instructions::initialize_market(
        payer.pubkey(),
        token_0,
        token_1,
        feelssol_mint,
        init_market_params.base_fee_bps,
        init_market_params.tick_spacing,
        init_market_params.initial_sqrt_price,
        init_market_params.initial_buy_feelssol_amount,
        None, // No creator feelssol account needed
        None, // No creator token out account needed
    ).unwrap();
    
    let mut tx = Transaction::new_with_payer(&[init_market_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();
    
    // Step 3: Deploy pool liquidity with stair pattern
    let (market_pubkey, _) = find_market_address(&token_0, &token_1);
    
    let deploy_params = DeployInitialLiquidityParams {
        tick_step_size: 100, // 100 ticks between steps
        initial_buy_feelssol_amount: 0, // No initial buy
    };
    
    let deploy_ix = feels_sdk::instructions::deploy_initial_liquidity(
        payer.pubkey(),
        market_pubkey,
        token_0,
        token_1,
        feelssol_mint,
        deploy_params.tick_step_size,
        deploy_params.initial_buy_feelssol_amount,
        None, // No deployer feelssol account 
        None, // No deployer token out account
    ).unwrap();
    
    let mut tx = Transaction::new_with_payer(&[deploy_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    let result = banks_client.process_transaction(tx).await;
    
    match result {
        Ok(_) => {
            println!("Pool stair pattern liquidity deployed successfully!");
            
            // Verify market state
            let market_account = banks_client
                .get_account(market_pubkey)
                .await
                .unwrap()
                .unwrap();
            
            let market = Market::try_deserialize(&mut market_account.data.as_ref()).unwrap();
            
            assert!(market.initial_liquidity_deployed);
            assert!(market.liquidity > 0);
            
            println!("Market liquidity after deployment: {}", market.liquidity);
        }
        Err(e) => {
            panic!("Failed to deploy stair pattern liquidity: {:?}", e);
        }
    }
}

#[tokio::test]
#[ignore = "Test needs updating for new architecture"]
async fn test_user_commitment_deployment() {
    let test = ProgramTest::new("feels", feels::id(), None);
    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    
    // Create token mints
    let feelssol_mint = create_mint(&mut banks_client, &payer, 9).await;
    let token_mint = create_mint(&mut banks_client, &payer, 6).await;
    
    // Create user token accounts and mint some tokens
    let user_feelssol = create_token_account(&mut banks_client, &payer, feelssol_mint, payer.pubkey()).await;
    let user_token = create_token_account(&mut banks_client, &payer, token_mint, payer.pubkey()).await;
    
    mint_to(&mut banks_client, &payer, feelssol_mint, user_feelssol, 1_000_000_000_000).await; // 1000 FeelsSOL
    mint_to(&mut banks_client, &payer, token_mint, user_token, 1_000_000_000).await; // 1000 tokens
    
    // Token order
    let (token_0, token_1) = if feelssol_mint < token_mint {
        (feelssol_mint, token_mint)
    } else {
        (token_mint, feelssol_mint)
    };
    
    // Initialize market
    let initial_sqrt_price = 1u128 << 64;
    
    let init_market_params = InitializeMarketParams {
        base_fee_bps: 30,
        tick_spacing: 10,
        initial_sqrt_price,
        initial_buy_feelssol_amount: 0,
    };
    
    let init_market_ix = feels_sdk::instructions::initialize_market(
        payer.pubkey(),
        token_0,
        token_1,
        feelssol_mint,
        init_market_params.base_fee_bps,
        init_market_params.tick_spacing,
        init_market_params.initial_sqrt_price,
        init_market_params.initial_buy_feelssol_amount,
        None, // No creator feelssol account needed
        None, // No creator token out account needed
    ).unwrap();
    
    let mut tx = Transaction::new_with_payer(&[init_market_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();
    
    // Deploy user liquidity
    let (market_pubkey, _) = find_market_address(&token_0, &token_1);
    
    let deploy_params = DeployInitialLiquidityParams {
        tick_step_size: 100,
        initial_buy_feelssol_amount: 0, // No initial buy
    };
    
    let deploy_ix = feels_sdk::instructions::deploy_initial_liquidity(
        payer.pubkey(),
        market_pubkey,
        token_0,
        token_1,
        feelssol_mint,
        deploy_params.tick_step_size,
        deploy_params.initial_buy_feelssol_amount,
        None, // No deployer feelssol account 
        None, // No deployer token out account
    ).unwrap();
    
    let mut tx = Transaction::new_with_payer(&[deploy_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    let result = banks_client.process_transaction(tx).await;
    
    match result {
        Ok(_) => {
            println!("User commitment liquidity deployed successfully!");
            
            // Verify market state
            let market_account = banks_client
                .get_account(market_pubkey)
                .await
                .unwrap()
                .unwrap();
            
            let market = Market::try_deserialize(&mut market_account.data.as_ref()).unwrap();
            
            assert!(market.initial_liquidity_deployed);
            assert_eq!(market.liquidity, 1000000); // The liquidity from the position
            
            println!("Market liquidity after deployment: {}", market.liquidity);
        }
        Err(e) => {
            panic!("Failed to deploy user commitment liquidity: {:?}", e);
        }
    }
}