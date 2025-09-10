# Feels Protocol Position System

## Overview

The Feels Protocol implements a sophisticated position system that unifies spot trading, time-based lending/borrowing, and leveraged positions through a thermodynamic model. This document explains the architecture of the position system and how different asset states interact within the protocol.

## The Three States Framework

### 1. Spot Trading (Native Assets)

Spot trading involves holding native tokens (like SOL, USDC) and trading directly between them. In the protocol's model, this operates in the **S dimension** (spot/swap) where you're exchanging actual tokens at discrete price ticks.

**Key characteristics:**
- Direct token-to-token exchanges
- Uses concentrated liquidity positions
- No synthetic assets created
- Operates on discrete price ticks

### 2. Position Trading (Synthetic Assets)

Position trading creates synthetic "contracts" that the protocol divides into two distinct dimensions:

#### T dimension (Time)
- Lending/borrowing positions with specific duration buckets (1 year, 2 years, 3 years, 5 years)
- Creates synthetic lending or borrowing position tokens

#### L dimension (Leverage)
- Long/short directional positions
- Creates synthetic leveraged position tokens

When you open these positions, you receive **position tokens** that automatically participate in the protocol's value flows. These synthetic assets:
- Express rights to underlying value
- Accumulate fees/funding through continuous rebasing
- Can be traded back to FeelsSOL (the hub token)

### 3. Liquidity Provision

Liquidity provision is then a way of participating in the three dimensional market:

- **Spot Liquidity**: You provide native assets to concentrated liquidity positions
- **Time Liquidity**: You lend assets (becoming a lender in the T dimension)
- **Leverage Liquidity**: You provide capacity for others to take leveraged positions

## Are LP positions Synthetic Assets?

**Sometimes**, depending on the dimension:

### When LP Doesn't Create Synthetic Assets:
- **Spot dimension**: Traditional concentrated liquidity positions are more like "staked" native assets earning fees, not synthetic tokens

### When LP Creates Synthetic Assets:
- **Time dimension**: When you lend, you receive synthetic lending position tokens
- **Leverage dimension**: Providing leverage capacity gives you synthetic position tokens

## Token Representation

The protocol uses different token standards for different position types:

### LP Positions: Non-Fungible Tokens (NFTs)
- Each liquidity position is a unique NFT
- Has specific tick ranges, liquidity amounts, and accumulated fees
- Cannot be combined with other positions due to unique parameters
- Enables individual position tracking and transfer

### Time & Leverage Positions: Fungible Tokens
- Standardized position tokens (e.g., all "30-day lending" tokens are identical)
- Can be freely transferred and aggregated
- Standard SPL tokens with dynamic exchange rates
- Backed by FeelsSOL

## The Unified Model

The protocol's innovation is that all three dimensions (S, T, L) are unified through a thermodynamic potential function:

$$V = -w_s \ln(S) - w_t \ln(T) - w_l \ln(L)$$

where:
- $V$ is the thermodynamic potential
- $w_s$, $w_t$, $w_l$ are the weights for each dimension
- $S$, $T$, $L$ are the state values for spot, time, and leverage dimensions

This means:
- All liquidity is interconnected
- Cross-dimensional arbitrage naturally emerges
- Position tokens in T and L dimensions are synthetic assets that can be valued against the spot dimension

## Enhanced Position System

### Position Architecture

The protocol implements a sophisticated position system that tracks individual position NFTs and distributes fees based on thermodynamic work attribution:

```rust
pub struct PositionNFT {
    pub position_account: Pubkey,  // The position NFT account (not owner)
    pub pool_id: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub fee_growth_inside_last: FeeGrowth,
    pub accumulated_fees: TokenAmounts,
    pub work_attribution_total: u128,
}
```

Note that the protocol tracks position accounts (NFTs), not the owner field. This preserves pseudonymity - the protocol doesn't know or care who owns the NFT.

### Fee Accumulation Mechanism

Position NFTs accumulate fees through two complementary mechanisms:

1. **Instantaneous Fee Collection**: Fees from trades are immediately attributed to participating position NFTs based on their contribution to trade execution
2. **Continuous Rebasing**: During epoch rebasing, accumulated fees compound into position value through growth factor adjustments

### Work-Based Fee Distribution

The protocol distributes fees to position NFTs proportionally to thermodynamic work performed:

$$\text{Position Fee}_i = \text{Total Position Fees} \times \frac{W_{\text{pos},i}}{\sum_j W_{\text{pos},j}}$$

where $W_{\text{pos},i}$ is the work attributed to position NFT $i$ across all trades in the epoch. The NFT automatically receives its fee share through rebasing growth factors.

### Cross-Dimensional Position Strategies

Position NFTs can deploy sophisticated strategies across dimensions:

#### Spot Position Strategy
- Provide concentrated liquidity around active price ranges
- Earn fees from spot trading volume
- Dynamic fee rates based on volatility and flow

#### Time Position Strategy  
- Lend assets across duration buckets
- Earn interest plus trading fees from duration transitions
- Protected from rate volatility through dynamic adjustments

#### Leverage Position Strategy
- Provide directional capacity for leveraged positions
- Earn funding fees plus trading fees
- Risk-adjusted returns based on market skew

### Position Protection Mechanisms

The protocol implements multiple layers of position protection:

1. **Dynamic Fee Scaling**: Fees increase with market stress to compensate positions for higher risk
2. **Impermanent Loss Mitigation**: Protocol buffer can subsidize position losses during extreme moves
3. **Work Attribution Weighting**: Positions providing liquidity where it's most needed earn proportionally more
4. **Tick-Based Gradients**: Distant tick liquidity earns higher fee multipliers

### Position NFT Properties

Position NFTs have the following properties:

```rust
pub struct PositionTokenMetadata {
    pub position_id: u64,
    pub pool: Pubkey,
    pub tick_range: (i32, i32),
    pub initial_liquidity: u128,
    pub creation_time: i64,
    pub fee_tier: FeeTier,
    pub accumulated_work: u128,
    pub lifetime_fees: TokenAmounts,
}
```

This NFT structure enables:
- Transfer of positions between wallets without protocol knowledge
- Composability with other DeFi protocols
- Transparent tracking of position performance
- Historical attribution of work and fees
- Pseudonymous liquidity provision - position ownership can change freely

## A Good Mental Model

Think of the protocol as having:

### 1. Native Layer
- Spot assets (actual tokens like SOL, USDC)
- Direct token swaps

### 2. Synthetic Layer
Position tokens representing:
- Time positions (lending/borrowing)
- Leverage positions (long/short)

### 3. Liquidity Provision
A mechanism that exists in both layers:
- In spot: You provide native assets
- In time/leverage: You create synthetic positions

## Hub-and-Spoke Architecture

All routing happens through FeelsSOL (hub token), creating a maximum 2-hop path for any transaction:
- **Entry**: JitoSOL → FeelsSOL
- **Position**: FeelsSOL ↔ Position Token
- **Exit**: FeelsSOL → JitoSOL

This unified approach means liquidity provision isn't a separate state but a role you can play in any dimension, sometimes creating synthetic assets (T, L dimensions) and sometimes just deploying native assets more efficiently (S dimension).

## Example: Borrowing Against a Synthetic Position

Let's walk through an example where you hold 100 tokens of a "30-day 2x long MEME" position and want to borrow against it:

### How Borrowing Against Synthetic Positions Works

The Feels protocol enables borrowing against synthetic positions by leveraging its exact pricing function to determine collateral value. This is a fundamental innovation that sets Feels apart from traditional DeFi lending protocols.

When you hold position tokens like "30-day 2x long MEME," these tokens have an exact value that can be determined by the thermodynamic pricing function at any moment. The protocol calculates this value using the formula $V_{\text{collateral}} = f_{\text{price}}(\text{PositionToken}) \rightarrow \text{FeelsSOL}$, which gives the precise FeelsSOL-equivalent value of your position without relying on external oracles.

The pricing function is sophisticated and accounts for multiple factors simultaneously. It considers the current market state across all three dimensions (S, T, L), applies time decay for duration-based positions, incorporates leverage risk premiums, and captures cross-dimensional arbitrage effects. The value of your position updates continuously based on the thermodynamic potential function $V = -w_s \ln(S) - w_t \ln(T) - w_l \ln(L)$, ensuring that collateral values always reflect the true market state.

Risk management is built into the system through differentiated collateral factors. Spot positions, being the most stable, receive higher loan-to-value ratios around 80%. Time-based positions like lending tokens receive medium LTV ratios of 60-70%, while leveraged positions receive the most conservative treatment with LTV ratios of 40-50%. This tiered approach ensures system stability while maximizing capital efficiency for users.

### Borrowing Example: 100 Tokens of "30-day 2x Long MEME"

Here's how borrowing against your synthetic position works:

**Step 1: Position Valuation via Exact Pricing Function**
- 100 tokens of "30-day 2x long MEME"
- The thermodynamic pricing function calculates: $V_{\text{FeelsSOL}} = f_{\text{price}}(\text{100 tokens})$
- Example: Let's say this equals 5,000 FeelsSOL based on current market state

**Step 2: Risk-Adjusted Collateral Parameters**
- Base collateral factor for 2x leveraged positions: 50% LTV
- Time adjustment for 30-day duration: $f_{\text{time}}(30) = 0.95$
- Effective LTV: $\text{LTV}_{\text{effective}} = 0.50 \times 0.95 = 47.5\%$

**Step 3: Calculate Borrowing Capacity**
- Maximum borrowing amount: $B_{\text{max}} = 5,000 \times 0.475 = 2,375$ FeelsSOL
- You can borrow up to 2,375 FeelsSOL while keeping your position

**Step 4: Dynamic Position Monitoring**
- The protocol continuously recalculates your position value using the pricing function
- Positions are never liquidated; instead, they rebase continuously to maintain conservation
- As market conditions change across S, T, and L dimensions, your position value updates through the rebasing mechanism
- The conservation law $\sum_i w_i \ln g_i = 0$ ensures value is preserved but redistributed