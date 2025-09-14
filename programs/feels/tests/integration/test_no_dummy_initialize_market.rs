//! Test without dummy accounts
use crate::common::*;
use feels::state::{ProtocolToken, PreLaunchEscrow};
use anchor_lang::InstructionData;
use solana_sdk::instruction::Instruction;

test_in_memory!(test_no_dummy_initialize_market, |ctx: TestContext| async move {
    println!("\n=== No Dummy Initialize Market Test ===");
    
    // Create token
    let creator = Keypair::new();
    ctx.airdrop(&creator.pubkey(), 1_000_000_000).await?;
    
    let creator_feelssol = ctx.create_ata(&creator.pubkey(), &ctx.feelssol_mint).await?;
    
    let token_mint = Keypair::new();
    let params = feels::instructions::MintTokenParams {
        ticker: "NDM".to_string(),
        name: "NoDummy".to_string(),
        uri: "https://test.com".to_string(),
    };
    
    let ix = feels_sdk::mint_token(
        creator.pubkey(),
        creator_feelssol,
        token_mint.pubkey(),
        ctx.feelssol_mint,
        params,
    )?;
    
    ctx.process_instruction(ix, &[&creator, &token_mint]).await?;
    println!("✓ Token minted");
    
    // Order tokens
    let (token_0, token_1) = if ctx.feelssol_mint < token_mint.pubkey() {
        (ctx.feelssol_mint, token_mint.pubkey())
    } else {
        (token_mint.pubkey(), ctx.feelssol_mint)
    };
    
    // Derive all PDAs
    let (market, _) = feels_sdk::find_market_address(&token_0, &token_1);
    let (buffer, _) = feels_sdk::find_buffer_address(&market);
    let (oracle, _) = Pubkey::find_program_address(
        &[b"oracle", market.as_ref()],
        &PROGRAM_ID,
    );
    let (vault_0, _) = feels_sdk::find_vault_0_address(&token_0, &token_1);
    let (vault_1, _) = feels_sdk::find_vault_1_address(&token_0, &token_1);
    let (market_authority, _) = Pubkey::find_program_address(
        &[b"authority", market.as_ref()],
        &PROGRAM_ID,
    );
    
    let project_token_mint = if token_0 != ctx.feelssol_mint { token_0 } else { token_1 };
    let (escrow, _) = Pubkey::find_program_address(
        &[b"escrow", project_token_mint.as_ref()],
        &PROGRAM_ID,
    );
    
    // Protocol token PDAs
    let (protocol_token_0, _) = if token_0 == ctx.feelssol_mint {
        Pubkey::find_program_address(&[b"dummy_protocol_0"], &PROGRAM_ID)
    } else {
        Pubkey::find_program_address(&[b"protocol_token", token_0.as_ref()], &PROGRAM_ID)
    };
    
    let (protocol_token_1, _) = if token_1 == ctx.feelssol_mint {
        Pubkey::find_program_address(&[b"dummy_protocol_1"], &PROGRAM_ID)
    } else {
        Pubkey::find_program_address(&[b"protocol_token", token_1.as_ref()], &PROGRAM_ID)
    };
    
    // Build instruction data
    let params = feels::instructions::InitializeMarketParams {
        base_fee_bps: 30,
        tick_spacing: 10,
        initial_sqrt_price: 79228162514264337593543950336u128,
        initial_buy_feelssol_amount: 0,
    };
    
    let data = feels::instruction::InitializeMarket { params }.data();
    
    // Try with only the essential accounts first
    let accounts = vec![
        AccountMeta::new(creator.pubkey(), true),              // 0: creator
        AccountMeta::new(token_0, false),                       // 1: token_0
        AccountMeta::new(token_1, false),                       // 2: token_1
        AccountMeta::new(market, false),                        // 3: market
        AccountMeta::new(buffer, false),                        // 4: buffer
        AccountMeta::new(oracle, false),                        // 5: oracle
        AccountMeta::new(vault_0, false),                       // 6: vault_0
        AccountMeta::new(vault_1, false),                       // 7: vault_1
        AccountMeta::new_readonly(market_authority, false),     // 8: market_authority
        AccountMeta::new_readonly(ctx.feelssol_mint, false),   // 9: feelssol_mint
        AccountMeta::new_readonly(protocol_token_0, false),     // 10: protocol_token_0
        AccountMeta::new_readonly(protocol_token_1, false),     // 11: protocol_token_1
        AccountMeta::new(escrow, false),                        // 12: escrow
        // Skip dummy accounts entirely
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // 13: system_program
        AccountMeta::new_readonly(spl_token::id(), false),      // 14: token_program
        AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),   // 15: rent
    ];
    
    println!("\nAccounts ({}  total):", accounts.len());
    for (i, account) in accounts.iter().enumerate() {
        println!("  {}: {}", i, account.pubkey);
    }
    
    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts,
        data,
    };
    
    // Process
    match ctx.process_instruction(ix, &[&creator]).await {
        Ok(_) => println!("\n✓ Market initialized successfully!"),
        Err(e) => println!("\n✗ Failed with error: {:?}", e),
    }
    
    Ok::<(), Box<dyn std::error::Error>>(())
});