//! Debug test for initialize_market issue
use crate::common::*;
use feels::state::{ProtocolToken, PreLaunchEscrow};
use anchor_lang::InstructionData;
use solana_sdk::instruction::Instruction;
use solana_program::{program_option::COption, program_pack::Pack};

test_in_memory!(test_debug_initialize_market, |ctx: TestContext| async move {
    println!("\n=== Debug Initialize Market Test ===");
    
    // Create token first
    let creator = Keypair::new();
    ctx.airdrop(&creator.pubkey(), 1_000_000_000).await?;
    
    let creator_feelssol = ctx.create_ata(&creator.pubkey(), &ctx.feelssol_mint).await?;
    
    let token_mint = Keypair::new();
    let params = feels::instructions::MintTokenParams {
        ticker: "DBG".to_string(),
        name: "Debug".to_string(),
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
    
    // Debug: Check all accounts before instruction
    println!("\nChecking accounts before instruction:");
    
    // Check token_0
    let token_0_data = ctx.get_account_raw(&token_0).await?;
    println!("token_0 ({}):", token_0);
    println!("  exists: {}", token_0_data.data.len() > 0);
    if token_0_data.data.len() > 0 {
        let mint = spl_token::state::Mint::unpack(&token_0_data.data)?;
        println!("  supply: {}", mint.supply);
        println!("  mint_authority: {:?}", mint.mint_authority);
        println!("  freeze_authority: {:?}", mint.freeze_authority);
    }
    
    // Check token_1
    let token_1_data = ctx.get_account_raw(&token_1).await?;
    println!("token_1 ({}):", token_1);
    println!("  exists: {}", token_1_data.data.len() > 0);
    if token_1_data.data.len() > 0 {
        let mint = spl_token::state::Mint::unpack(&token_1_data.data)?;
        println!("  supply: {}", mint.supply);
        println!("  mint_authority: {:?}", mint.mint_authority);
        println!("  freeze_authority: {:?}", mint.freeze_authority);
    }
    
    // Check escrow
    let escrow_data = ctx.get_account_raw(&escrow).await?;
    println!("escrow ({}):", escrow);
    println!("  exists: {}", escrow_data.data.len() > 0);
    println!("  data length: {}", escrow_data.data.len());
    
    // Check protocol token
    let (protocol_token_0, _) = Pubkey::find_program_address(
        &[b"protocol_token", token_0.as_ref()],
        &PROGRAM_ID,
    );
    let (protocol_token_1, _) = Pubkey::find_program_address(
        &[b"protocol_token", token_1.as_ref()],
        &PROGRAM_ID,
    );
    
    if token_0 != ctx.feelssol_mint {
        let ptoken_data = ctx.get_account_raw(&protocol_token_0).await?;
        println!("protocol_token_0 ({}):", protocol_token_0);
        println!("  exists: {}", ptoken_data.data.len() > 0);
        println!("  data length: {}", ptoken_data.data.len());
    }
    
    if token_1 != ctx.feelssol_mint {
        let ptoken_data = ctx.get_account_raw(&protocol_token_1).await?;
        println!("protocol_token_1 ({}):", protocol_token_1);
        println!("  exists: {}", ptoken_data.data.len() > 0);
        println!("  data length: {}", ptoken_data.data.len());
    }
    
    // Now try initialize with detailed error handling
    println!("\nAttempting initialize_market...");
    
    let ix = feels_sdk::initialize_market(
        creator.pubkey(),
        token_0,
        token_1,
        ctx.feelssol_mint,
        30,     // base_fee_bps
        10,     // tick_spacing
        79228162514264337593543950336u128, // sqrt price = 1:1
        0,      // no initial buy
        None,   // no creator_feelssol account
        None,   // no creator_token_out account
    )?;
    
    match ctx.process_instruction(ix, &[&creator]).await {
        Ok(_) => println!("✓ Market initialized successfully!"),
        Err(e) => {
            println!("✗ Failed with error: {:?}", e);
        }
    }
    
    Ok::<(), Box<dyn std::error::Error>>(())
});