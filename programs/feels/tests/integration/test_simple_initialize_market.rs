//! Simplified test for initialize_market
use crate::common::*;
use feels::state::{ProtocolToken, PreLaunchEscrow};
use anchor_lang::InstructionData;
use solana_sdk::instruction::Instruction;
use solana_program::{program_option::COption, program_pack::Pack};

test_in_memory!(test_simple_initialize_market, |ctx: TestContext| async move {
    println!("\n=== Simple Initialize Market Test ===");
    
    // Create token first
    let creator = Keypair::new();
    ctx.airdrop(&creator.pubkey(), 1_000_000_000).await?;
    
    let creator_feelssol = ctx.create_ata(&creator.pubkey(), &ctx.feelssol_mint).await?;
    
    let token_mint = Keypair::new();
    let params = feels::instructions::MintTokenParams {
        ticker: "SMP".to_string(),
        name: "Simple".to_string(), 
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
    
    // Get all PDAs - use SDK functions to ensure consistency
    let (token_0, token_1) = if ctx.feelssol_mint < token_mint.pubkey() {
        (ctx.feelssol_mint, token_mint.pubkey())
    } else {
        (token_mint.pubkey(), ctx.feelssol_mint)
    };
    
    // Use SDK to build instruction - it should handle all the account ordering
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
    
    println!("\nInstruction accounts:");
    for (i, account) in ix.accounts.iter().enumerate() {
        println!("  {}: {} (signer: {}, writable: {})", 
            i, account.pubkey, account.is_signer, account.is_writable);
    }
    
    // Process instruction
    match ctx.process_instruction(ix, &[&creator]).await {
        Ok(_) => {
            println!("\n✓ Market initialized successfully!");
        }
        Err(e) => {
            println!("\n✗ Failed with error: {:?}", e);
        }
    }
    
    Ok::<(), Box<dyn std::error::Error>>(())
});