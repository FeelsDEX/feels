//! Debug account ordering
use crate::common::*;
use feels::state::{ProtocolToken, PreLaunchEscrow};

test_in_memory!(test_account_debug, |ctx: TestContext| async move {
    println!("\n=== Account Debug Test ===");
    
    // Create token
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
    
    // Order tokens
    let (token_0, token_1) = if ctx.feelssol_mint < token_mint.pubkey() {
        (ctx.feelssol_mint, token_mint.pubkey())
    } else {
        (token_mint.pubkey(), ctx.feelssol_mint)
    };
    
    println!("\nToken ordering:");
    println!("  token_0: {} ({})", token_0, if token_0 == ctx.feelssol_mint { "FeelsSOL" } else { "Project" });
    println!("  token_1: {} ({})", token_1, if token_1 == ctx.feelssol_mint { "FeelsSOL" } else { "Project" });
    
    // Get the instruction from SDK
    let ix = feels_sdk::initialize_market(
        creator.pubkey(),
        token_0,
        token_1,
        ctx.feelssol_mint,
        30,
        10,
        79228162514264337593543950336u128,
        0,
        None,
        None,
    )?;
    
    println!("\nSDK generated {} accounts:", ix.accounts.len());
    
    // Expected accounts based on InitializeMarket struct:
    let expected_accounts = vec![
        "creator (signer)",
        "token_0 (Mint)",
        "token_1 (Mint)",
        "market (init)",
        "buffer (init)",
        "oracle (init)",
        "vault_0 (init)",
        "vault_1 (init)",
        "market_authority (CHECK)",
        "feelssol_mint (AccountInfo)",
        "protocol_token_0 (CHECK)",
        "protocol_token_1 (CHECK)",
        "escrow (CHECK)",
        "creator_feelssol (CHECK)",
        "creator_token_out (CHECK)",
        "system_program (Program)",
        "token_program (Program)",
        "rent (Sysvar)",
    ];
    
    println!("\nExpected {} accounts:", expected_accounts.len());
    for (i, name) in expected_accounts.iter().enumerate() {
        println!("  {}: {}", i, name);
    }
    
    println!("\nActual accounts from SDK:");
    for (i, account) in ix.accounts.iter().enumerate() {
        let expected_name = expected_accounts.get(i).unwrap_or(&"EXTRA");
        println!("  {}: {} - {}", i, account.pubkey, expected_name);
        
        // Check for specific accounts
        if account.pubkey == solana_sdk::system_program::id() {
            println!("     ^ This is System Program");
        }
        if account.pubkey == spl_token::id() {
            println!("     ^ This is Token Program");
        }
        if account.pubkey == solana_sdk::sysvar::rent::id() {
            println!("     ^ This is Rent Sysvar");
        }
    }
    
    // Check if we have the right number of accounts
    if ix.accounts.len() != expected_accounts.len() {
        println!("\n❌ Account count mismatch!");
        println!("   Expected: {}", expected_accounts.len());
        println!("   Got: {}", ix.accounts.len());
    } else {
        println!("\n✓ Account count matches");
    }
    
    Ok::<(), Box<dyn std::error::Error>>(())
});