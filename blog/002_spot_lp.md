# Feels Programmatic Spot Liquidity

## Turning Short-term Volatility into Long-term Value

One of the primary goals of Feels is to convert speculative energy into a rising floor price by directing swap fees to increase the capital backing the floor. As this capital accumulates, the floor price ratchets upward, creating a guaranteed redemption price that can only increases over time.

## Why Concentrated Liquidity

Establishing a price floor requires placing liquidity at a specific price. This is impossible with constant product AMMs (similar to Uniswap v2), as they spread liquidity across an infinite price range (x * y = k). A concentrated liquidity AMM (CLMM), however, allows liquidity to be placed in discrete price ranges. Feels uses a CLMM to create a deep bid wall at the exact floor price.

## Rethinking Automated Market Making

Traders expect a price quote to be available at any requested swap size. Most Solana launchpads achieve this by reserving tokens to provide permanent liquidity across the full price range. While this ensures the protocol will provide an offer, most of this capital sits idle at a price the market will never trade.

Feels uses a CLMM to implement its floor mechanism. Why not use this provide better automatic spot liquidity provision? Instead of spreading capital across the full range, Feels concentrates liquidity within the active trading range. 

## On-chain Pricing

While concentrated liquidity AMMs have improved capital efficiency, they still face the same fundamental problem as constant product AMMs: LPs are passive takers who fill any trade at the quoted price. Arbitrageurs with access to upstream price information from centralized exchanges trade against DEX quotes, systematically extracting value from LPs and eroding markets.

This is a problem for on-chian BTC or SOL markets, however tokens launched through Feels are created and distributed entirely through the protocol. Price discovery happens on Feels, and liquidity is programmatically built up on-chain, rather than a centralized exchange. Without external prices to arbitrage against, LPs see reduced toxic flow. Less adverse LP conditions opens the door to *active* automated market making, Feels can protocolize an LP strategy that's responsive to market conditions.

## Just-in-Time Liquidity

However, any strategy that leaves resting liquidity on the books is still at risk. Faster on-chain actors can exploit stale quotes before an active manager can react. The best defense against is providing liquidity only for the exact moment it's needed, ensuring the quote is never stale.

Instead of maintaining passive positions, Feels provides liquidity reactively. When a swap arrives, A Just-in-Time execution system places a narrow, contrarian liquidity band ahead of the trade. The position is priced around a manipulation-resistant geometric time-weighted average price (GTWAP).

The swap instruction includes callback, which places liquidity, executes a trade against it, and removes liquidity in a single transaction. This atomic design prevents sandwich attacks and eliminates risk of stale quotes.

## Complementary Market Making

The Feels JIT system provides price leadership without competing for the tightest quotes. Instead, liquidity is placed at a distance from the current price, allowing external LPs to profitably quote tighter. A dynamic fee model complements this system by charging LPs less for trades that add depth. JIT captures value from trades that move the market price, while external LPs service the flow at the current spread.

Providing liquidity deeper in the market is only sustainable with a layered defense. The system protects itself by tracking directional toxicity and reducing the amount of liquidity it offers when the price moves against its fills. A rolling budget cap prevents draining attacks, while a circuit breaker provides a final backstop against extreme volatility.

## From JIT to Floor

By providing on-demand liquidity for impactful trades, the Feels JIT system generates revenue from the market's speculative energy. The fees from these swaps are collected in a protocol-owned account, which funds JIT operations and accumulates a surplus. This excess capital is then used to systematically raise the floor price, creating a one-way ratchet that converts trading activity into long-term value.
