---
title: "Hub and Spoke Architecture"
description: "Understanding the FeelsSOL-centric routing model"
category: "Specifications"
order: 3
---

# Hub and Spoke Architecture

## Overview

The hub-and-spoke architecture is a fundamental design principle of Feels Protocol. Unlike traditional AMMs where any token can pair with any other token, Feels Protocol requires all tokens to trade through a central hub: FeelsSOL.

## Why Hub-and-Spoke?

### Liquidity Concentration
Instead of fragmenting liquidity across multiple pairs (ETH/USDC, ETH/USDT, USDC/USDT), all liquidity for a token concentrates in a single pool with FeelsSOL.

### Simplified Routing
- Direct trades: Token ↔ FeelsSOL (1 hop)
- Cross trades: TokenA → FeelsSOL → TokenB (2 hops max)
- No complex routing algorithms needed

### Capital Efficiency
Liquidity providers only need to provide liquidity to one pool per token, maximizing capital efficiency and fee generation.

## FeelsSOL Token

FeelsSOL is the hub token that:
- Wraps yield-bearing LSTs (initially JitoSOL)
- Provides base liquidity for all pairs
- Accrues staking rewards to holders

## Trade Examples

### Direct Trade (1 hop)
```
USDC → FeelsSOL
FeelsSOL → BONK
```

### Cross Trade (2 hops)
```
USDC → FeelsSOL → BONK
WIF → FeelsSOL → SAMO
```

## Benefits for Traders

1. **Better prices**: Concentrated liquidity means less slippage
2. **Predictable routing**: Always know your trade path
3. **Lower fees**: Efficient routing reduces total fees

## Benefits for LPs

1. **Higher yields**: Concentrated positions capture more fees
2. **Simpler management**: One pool per token
3. **Reduced impermanent loss**: Paired with stable, yield-bearing FeelsSOL