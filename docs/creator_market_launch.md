# Creator Market Launch Guide

This guide explains the creator-only market launch mechanism and the initial buy option.

## Overview

When a token is minted through the protocol's `mint_token` instruction:
- The minter becomes the token's "creator"
- Only the creator can launch a market for that token
- The creator can optionally be the first buyer when launching the market

## Key Features

### 1. Creator-Only Market Launch
- **Exclusive Right**: Only the account that minted a token can create its first market
- **FeelsSOL Requirement**: All markets must pair with FeelsSOL
- **No Token Ownership**: The creator doesn't own any tokens initially (all go to buffer)
- **Market Authority**: The creator becomes the market authority

### 2. Initial Buy Option
- **First Purchase**: Creator can buy tokens with FeelsSOL during market launch
- **Atomic Operation**: Buy happens in the same transaction as market creation
- **Price Setting**: Buy executes at the initial price set for the market

## How It Works

### Step 1: Mint a Token
```rust
// Creator mints a new token
let params = MintTokenParams {
    ticker: "MYTOKEN".to_string(),
    name: "My Token".to_string(),
    uri: "https://metadata.url".to_string(),
};

let ix = mint_token(
    creator.pubkey(),
    token_mint.pubkey(),
    feelssol_mint,
    params,
)?;
```

### Step 2: Launch Market (No Initial Buy)
```rust
// Launch market without initial buy
let ix = initialize_market(
    creator.pubkey(),        // Must be token creator
    feelssol_mint,          // Token 0 or 1 must be FeelsSOL
    token_mint.pubkey(),    // Your minted token
    feelssol_mint,
    30,                     // Fee (0.3%)
    10,                     // Tick spacing
    initial_price,          // Starting price
    0,                      // No initial buy
    None,                   // No FeelsSOL account needed
    None,                   // No token out account needed
)?;
```

### Step 3: Launch Market with Initial Buy
```rust
// Launch market with initial buy
let initial_buy_amount = 1_000_000_000; // 1 FeelsSOL

let ix = initialize_market(
    creator.pubkey(),
    feelssol_mint,
    token_mint.pubkey(),
    feelssol_mint,
    30,
    10,
    initial_price,
    initial_buy_amount,                    // Amount of FeelsSOL to spend
    Some(creator_feelssol_account),        // Creator's FeelsSOL account
    Some(creator_token_out_account),       // Where to receive tokens
)?;
```

## Initial Buy Mechanics

### How It Works
1. Creator specifies FeelsSOL amount to spend
2. FeelsSOL is transferred to market vault during initialization
3. Tokens are calculated based on initial price
4. Buy is executed after initial liquidity deployment

### Important Notes
- Initial buy happens at the exact initial price
- No slippage or price impact on the initial buy
- Creator must have sufficient FeelsSOL balance
- Buy amount is included in market initialization event

## Security Features

### Creator Validation
```rust
// Only the token creator can launch markets
require!(
    protocol_token.creator == creator.key(),
    FeelsError::UnauthorizedSigner
);
```

### Token Pairing Rules
- At least one token must be FeelsSOL
- Both tokens cannot be non-FeelsSOL protocol tokens
- External tokens cannot create markets

### Account Validation
- Creator must be a system account (not PDA)
- Token accounts validated for correct mints
- Sufficient balance checks for initial buy

## Example: Complete Flow

```rust
// 1. Mint token
let token_mint = Keypair::new();
let mint_params = MintTokenParams {
    ticker: "LAUNCH".to_string(),
    name: "Launch Token".to_string(),
    uri: "https://metadata.json".to_string(),
};

process_instruction(
    mint_token(creator.pubkey(), token_mint.pubkey(), feelssol_mint, mint_params)?,
    &[&creator, &token_mint]
)?;

// 2. Prepare for market launch
let creator_feelssol = get_ata(&creator.pubkey(), &feelssol_mint);
let creator_token = get_ata(&creator.pubkey(), &token_mint.pubkey());

// 3. Launch with initial buy
let market = process_instruction(
    initialize_market(
        creator.pubkey(),
        feelssol_mint,
        token_mint.pubkey(),
        feelssol_mint,
        30,                     // 0.3% fee
        10,                     // tick spacing
        price_1_to_1(),         // 1:1 initial price
        100_000_000,            // Buy 0.1 FeelsSOL worth
        Some(creator_feelssol),
        Some(creator_token),
    )?,
    &[&creator]
)?;
```

## Benefits

### For Creators
- **Guaranteed Launch**: No one can front-run your market creation
- **First Mover**: Option to be the first buyer at your chosen price
- **Control**: Set initial parameters and price

### For the Protocol
- **Quality Control**: Only serious projects create tokens
- **Accountability**: Clear link between token and creator
- **Fair Launch**: No insider token allocations

## Common Patterns

### Pattern 1: Pure Fair Launch
- Creator mints token
- Creates market at desired price
- No initial buy (lets community be first buyers)

### Pattern 2: Creator Bootstrap
- Creator mints token
- Creates market with small initial buy
- Provides initial liquidity and price discovery

### Pattern 3: Coordinated Launch
- Creator mints token
- Announces launch time
- Creates market with larger initial buy
- Community can immediately trade

## Error Handling

### Common Errors

**UnauthorizedSigner**
- Non-creator trying to launch market
- Solution: Only token creator can launch

**InsufficientBalance**
- Not enough FeelsSOL for initial buy
- Solution: Ensure adequate balance

**InvalidMint**
- Wrong token accounts provided
- Solution: Verify account mints match

**RequiresFeelsSOLPair**
- Neither token is FeelsSOL
- Solution: One token must be FeelsSOL

## Best Practices

1. **Test First**: Use devnet to test your launch strategy
2. **Check Balances**: Ensure sufficient FeelsSOL for initial buy
3. **Set Realistic Prices**: Consider market dynamics
4. **Plan Liquidity**: Have a strategy for post-launch liquidity
5. **Communicate**: Let community know your launch plans

## FAQ

**Q: Can I transfer creator rights?**
A: No, creator rights are permanently tied to the minting account.

**Q: Can I create multiple markets for my token?**
A: Yes, after the first market exists, standard market creation rules apply.

**Q: What happens to the initial buy tokens?**
A: They go directly to your specified token account, like a normal swap.

**Q: Can I cancel an initial buy?**
A: No, the initial buy is atomic with market creation.

**Q: Do I pay fees on the initial buy?**
A: Yes, normal market fees apply to the initial buy.