//! Minimal test for initialize_market
use crate::common::*;
use feels::state::{ProtocolToken, PreLaunchEscrow};
use anchor_lang::InstructionData;
use solana_sdk::instruction::Instruction;

test_in_memory!(test_minimal_initialize_market, |ctx: TestContext| async move {
    println!("\n=== Minimal Initialize Market Test ===");
    
    // Create token first
    let creator = Keypair::new();
    ctx.airdrop(&creator.pubkey(), 1_000_000_000).await?;
    
    let creator_feelssol = ctx.create_ata(&creator.pubkey(), &ctx.feelssol_mint).await?;
    
    let token_mint = Keypair::new();
    let params = feels::instructions::MintTokenParams {
        ticker: "MIN".to_string(),
        name: "Minimal".to_string(),
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
    
    // Get all PDAs
    let (token_0, token_1) = if ctx.feelssol_mint < token_mint.pubkey() {
        (ctx.feelssol_mint, token_mint.pubkey())
    } else {
        (token_mint.pubkey(), ctx.feelssol_mint)
    };
    
    let (market, _) = feels_sdk::find_market_address(&token_0, &token_1);
    let (buffer, _) = feels_sdk::find_buffer_address(&market);
    let (oracle, _) = Pubkey::find_program_address(
        &[b"oracle", market.as_ref()],
        &PROGRAM_ID,
    );
    let (vault_0, _) = feels_sdk::find_vault_address(&market, &token_0);
    let (vault_1, _) = feels_sdk::find_vault_address(&market, &token_1);
    let (market_authority, _) = Pubkey::find_program_address(
        &[b"authority", market.as_ref()],
        &PROGRAM_ID,
    );
    
    let project_token_mint = if token_0 != ctx.feelssol_mint { token_0 } else { token_1 };
    let (escrow, _) = Pubkey::find_program_address(
        &[b"escrow", project_token_mint.as_ref()],
        &PROGRAM_ID,
    );
    
    // Verify escrow exists
    let escrow_account: PreLaunchEscrow = ctx.get_account(&escrow).await?
        .ok_or("Escrow not found")?;
    println!("✓ Escrow exists: {}", escrow);
    
    // Build instruction manually
    let params = feels::instructions::InitializeMarketParams {
        base_fee_bps: 30,
        tick_spacing: 10,
        initial_sqrt_price: 79228162514264337593543950336u128,
        initial_buy_feelssol_amount: 0,
    };
    
    let data = feels::instruction::InitializeMarket { params }.data();
    
    let accounts = vec![
        AccountMeta::new(creator.pubkey(), true),
        AccountMeta::new(token_0, false),
        AccountMeta::new(token_1, false),
        AccountMeta::new(market, false),
        AccountMeta::new(buffer, false),
        AccountMeta::new(oracle, false),
        AccountMeta::new(vault_0, false),
        AccountMeta::new(vault_1, false),
        AccountMeta::new_readonly(market_authority, false),
        AccountMeta::new_readonly(ctx.feelssol_mint, false),
        // Protocol tokens - use unique dummy PDAs for FeelsSOL
        if token_0 == ctx.feelssol_mint {
            let (dummy_protocol_0, _) = Pubkey::find_program_address(
                &[b"dummy_protocol_0"],
                &PROGRAM_ID,
            );
            AccountMeta::new_readonly(dummy_protocol_0, false)
        } else {
            let (pda, _) = Pubkey::find_program_address(
                &[b"protocol_token", token_0.as_ref()],
                &PROGRAM_ID,
            );
            AccountMeta::new(pda, false)
        },
        if token_1 == ctx.feelssol_mint {
            let (dummy_protocol_1, _) = Pubkey::find_program_address(
                &[b"dummy_protocol_1"],
                &PROGRAM_ID,
            );
            AccountMeta::new_readonly(dummy_protocol_1, false)
        } else {
            let (pda, _) = Pubkey::find_program_address(
                &[b"protocol_token", token_1.as_ref()],
                &PROGRAM_ID,
            );
            AccountMeta::new(pda, false)
        },
        AccountMeta::new(escrow, false),
        // Use unique dummy PDAs for creator accounts
        {
            let (dummy_feelssol, _) = Pubkey::find_program_address(
                &[b"dummy_feelssol"],
                &PROGRAM_ID,
            );
            AccountMeta::new(dummy_feelssol, false)
        },
        {
            let (dummy_token_out, _) = Pubkey::find_program_address(
                &[b"dummy_token_out"],
                &PROGRAM_ID,
            );
            AccountMeta::new(dummy_token_out, false)
        },
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // system_program
        AccountMeta::new_readonly(spl_token::id(), false), // token_program
        AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false), // rent
    ];
    
    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts,
        data,
    };
    
    // Try to process
    match ctx.process_instruction(ix, &[&creator]).await {
        Ok(_) => println!("✓ Market initialized successfully!"),
        Err(e) => println!("✗ Failed: {:?}", e),
    }
    
    Ok::<(), Box<dyn std::error::Error>>(())
});