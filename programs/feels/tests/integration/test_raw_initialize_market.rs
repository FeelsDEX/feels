//! Raw test for initialize_market to bypass Anchor validation
use crate::common::*;
use feels::state::{ProtocolToken, PreLaunchEscrow};
use anchor_lang::InstructionData;
use solana_sdk::instruction::Instruction;
use solana_program::instruction::AccountMeta;

test_in_memory!(test_raw_initialize_market, |ctx: TestContext| async move {
    println!("\n=== Raw Initialize Market Test ===");
    
    // Create token
    let creator = Keypair::new();
    ctx.airdrop(&creator.pubkey(), 1_000_000_000).await?;
    
    let creator_feelssol = ctx.create_ata(&creator.pubkey(), &ctx.feelssol_mint).await?;
    
    let token_mint = Keypair::new();
    let params = feels::instructions::MintTokenParams {
        ticker: "RAW".to_string(),
        name: "Raw".to_string(),
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
    
    println!("\nTokens:");
    println!("  token_0: {} ({})", token_0, if token_0 == ctx.feelssol_mint { "FeelsSOL" } else { "Project" });
    println!("  token_1: {} ({})", token_1, if token_1 == ctx.feelssol_mint { "FeelsSOL" } else { "Project" });
    
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
        // Use a dummy PDA for FeelsSOL
        Pubkey::find_program_address(&[b"dummy_protocol_0"], &PROGRAM_ID)
    } else {
        Pubkey::find_program_address(&[b"protocol_token", token_0.as_ref()], &PROGRAM_ID)
    };
    
    let (protocol_token_1, _) = if token_1 == ctx.feelssol_mint {
        // Use a dummy PDA for FeelsSOL
        Pubkey::find_program_address(&[b"dummy_protocol_1"], &PROGRAM_ID)
    } else {
        Pubkey::find_program_address(&[b"protocol_token", token_1.as_ref()], &PROGRAM_ID)
    };
    
    // Dummy accounts for optional fields
    let (dummy_feelssol, _) = Pubkey::find_program_address(&[b"dummy_feelssol"], &PROGRAM_ID);
    let (dummy_token_out, _) = Pubkey::find_program_address(&[b"dummy_token_out"], &PROGRAM_ID);
    
    // Build raw instruction data
    let params = feels::instructions::InitializeMarketParams {
        base_fee_bps: 30,
        tick_spacing: 10,
        initial_sqrt_price: 79228162514264337593543950336u128,
        initial_buy_feelssol_amount: 0,
    };
    
    let data = feels::instruction::InitializeMarket { params }.data();
    
    // Build accounts array manually
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
        AccountMeta::new_readonly(dummy_feelssol, false),       // 13: creator_feelssol
        AccountMeta::new_readonly(dummy_token_out, false),      // 14: creator_token_out
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // 15: system_program
        AccountMeta::new_readonly(spl_token::id(), false),      // 16: token_program
        AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),   // 17: rent
    ];
    
    println!("\nAccounts:");
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