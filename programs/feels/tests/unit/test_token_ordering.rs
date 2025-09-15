//! Test token ordering validation

use crate::common::*;
use feels_sdk as sdk;

#[tokio::test]
async fn test_feelssol_must_be_token_0() {
    let ctx = TestContext::new(TestEnvironment::InMemory).await.unwrap();

    // Create a non-FeelsSOL token
    let token_mint = ctx.create_mint(&ctx.payer().await, 6).await.unwrap();

    // Try to create market with FeelsSOL as token_1 (should fail)
    let result = ctx
        .initialize_market(
            &ctx.accounts.market_creator,
            &token_mint.pubkey(), // token_0 (non-FeelsSOL)
            &ctx.feelssol_mint,   // token_1 (FeelsSOL)
            30,                   // fee tier
            10,                   // tick spacing
            1u128 << 64,          // initial price
            0,                    // no initial buy
        )
        .await;

    // Should fail with InvalidTokenOrder
    assert!(result.is_err());
    let err = result.unwrap_err();
    println!("Error received: {}", err);
    assert!(
        err.to_string().contains("Invalid token order")
            || err.to_string().contains("FeelsSOL must be token_0"),
        "Expected token order error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_correct_token_ordering() {
    let ctx = TestContext::new(TestEnvironment::InMemory).await.unwrap();

    // In the hub-and-spoke model:
    // 1. FeelsSOL must always be token_0
    // 2. Token ordering must satisfy token_0 < token_1
    // This test validates the SDK handles these requirements correctly

    // Create a non-FeelsSOL token with pubkey > FeelsSOL
    let token_mint = loop {
        let mint = ctx.create_mint(&ctx.payer().await, 6).await.unwrap();
        if mint.pubkey() > ctx.feelssol_mint {
            break mint;
        }
    };

    // Use SDK to validate token ordering
    let result =
        sdk::sort_tokens_with_feelssol(ctx.feelssol_mint, token_mint.pubkey(), ctx.feelssol_mint);

    // Should succeed and return FeelsSOL as token_0
    match result {
        Ok((token_0, token_1)) => {
            assert_eq!(token_0, ctx.feelssol_mint, "FeelsSOL should be token_0");
            assert_eq!(
                token_1,
                token_mint.pubkey(),
                "Other token should be token_1"
            );
            assert!(token_0 < token_1, "Token ordering should be maintained");
        }
        Err(e) => panic!("Token sorting failed: {}", e),
    }
}

#[tokio::test]
async fn test_no_feelssol_fails() {
    let ctx = TestContext::new(TestEnvironment::InMemory).await.unwrap();

    // Create two non-FeelsSOL tokens
    let token_a = ctx.create_mint(&ctx.payer().await, 6).await.unwrap();
    let token_b = ctx.create_mint(&ctx.payer().await, 9).await.unwrap();

    // Order them correctly
    let (token_0, token_1) = if token_a.pubkey() < token_b.pubkey() {
        (token_a.pubkey(), token_b.pubkey())
    } else {
        (token_b.pubkey(), token_a.pubkey())
    };

    // Try to create market without FeelsSOL (should fail)
    let result = ctx
        .initialize_market(
            &ctx.accounts.market_creator,
            &token_0,
            &token_1,
            30,          // fee tier
            10,          // tick spacing
            1u128 << 64, // initial price
            0,           // no initial buy
        )
        .await;

    // Should fail with RequiresFeelsSOLPair
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("One token must be FeelsSOL"));
}

#[tokio::test]
async fn test_sdk_validation() {
    use feels_sdk as sdk;

    let feelssol_mint = pubkey!("FeeLsW8fYn1CqkPuVChUdVVRMDYvdSkBEemkpf2ahXQ");
    let token_mint = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    // Test SDK validation for incorrect order
    let result = sdk::initialize_market(
        Keypair::new().pubkey(),
        token_mint,    // token_0 (non-FeelsSOL)
        feelssol_mint, // token_1 (FeelsSOL)
        feelssol_mint,
        30,
        10,
        1u128 << 64,
        0,
        None,
        None,
    );

    assert!(result.is_err());
    match result {
        Err(sdk::SdkError::InvalidParameters(msg)) => {
            assert!(msg.contains("FeelsSOL must be token_0"));
        }
        _ => panic!("Expected InvalidParameters error"),
    }

    // Test SDK validation for no FeelsSOL
    let other_token = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    let result = sdk::initialize_market(
        Keypair::new().pubkey(),
        token_mint,
        other_token,
        feelssol_mint,
        30,
        10,
        1u128 << 64,
        0,
        None,
        None,
    );

    assert!(result.is_err());
    match result {
        Err(sdk::SdkError::InvalidParameters(msg)) => {
            assert!(msg.contains("One token must be FeelsSOL"));
        }
        _ => panic!("Expected InvalidParameters error"),
    }
}

#[tokio::test]
async fn test_sdk_sort_tokens_with_feelssol() {
    use feels_sdk::sort_tokens_with_feelssol;

    let feelssol_mint = pubkey!("FeeLsW8fYn1CqkPuVChUdVVRMDYvdSkBEemkpf2ahXQ");
    let token_mint = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    // Test sorting with FeelsSOL as first argument
    let result = sort_tokens_with_feelssol(feelssol_mint, token_mint, feelssol_mint).unwrap();
    assert_eq!(result.0, feelssol_mint);
    assert_eq!(result.1, token_mint);

    // Test sorting with FeelsSOL as second argument
    let result = sort_tokens_with_feelssol(token_mint, feelssol_mint, feelssol_mint).unwrap();
    assert_eq!(result.0, feelssol_mint);
    assert_eq!(result.1, token_mint);

    // Test with no FeelsSOL (should fail)
    let other_token = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    let result = sort_tokens_with_feelssol(token_mint, other_token, feelssol_mint);
    assert!(result.is_err());
}
