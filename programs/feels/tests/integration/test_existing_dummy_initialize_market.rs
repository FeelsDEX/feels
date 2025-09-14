//! Test with existing dummy accounts
use crate::common::*;
use feels::state::{ProtocolToken, PreLaunchEscrow};
use anchor_lang::InstructionData;
use solana_sdk::instruction::Instruction;

test_in_memory!(test_existing_dummy_initialize_market, |ctx: TestContext| async move {
    println!("\n=== Existing Dummy Initialize Market Test ===");
    
    // Create token
    let creator = Keypair::new();
    ctx.airdrop(&creator.pubkey(), 1_000_000_000).await?;
    
    let creator_feelssol = ctx.create_ata(&creator.pubkey(), &ctx.feelssol_mint).await?;
    
    let token_mint = Keypair::new();
    let params = feels::instructions::MintTokenParams {
        ticker: "DUM".to_string(),
        name: "Dummy".to_string(),
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
    
    // Create dummy token accounts that actually exist
    let dummy_feelssol_account = ctx.create_ata(&creator.pubkey(), &ctx.feelssol_mint).await?;
    let dummy_token_out_account = ctx.create_ata(&creator.pubkey(), &token_mint.pubkey()).await?;
    
    println!("✓ Created dummy accounts:");
    println!("  dummy_feelssol: {}", dummy_feelssol_account);
    println!("  dummy_token_out: {}", dummy_token_out_account);
    
    // Order tokens
    let (token_0, token_1) = if ctx.feelssol_mint < token_mint.pubkey() {
        (ctx.feelssol_mint, token_mint.pubkey())
    } else {
        (token_mint.pubkey(), ctx.feelssol_mint)
    };
    
    // Use SDK to build instruction with real dummy accounts
    let ix = feels_sdk::initialize_market(
        creator.pubkey(),
        token_0,
        token_1,
        ctx.feelssol_mint,
        30,     // base_fee_bps
        10,     // tick_spacing
        79228162514264337593543950336u128, // sqrt price = 1:1
        0,      // no initial buy
        Some(dummy_feelssol_account),   // use real account
        Some(dummy_token_out_account),  // use real account
    )?;
    
    // Process
    match ctx.process_instruction(ix, &[&creator]).await {
        Ok(_) => println!("\n✓ Market initialized successfully!"),
        Err(e) => println!("\n✗ Failed with error: {:?}", e),
    }
    
    Ok::<(), Box<dyn std::error::Error>>(())
});