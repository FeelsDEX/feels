# Unified Feels Markets

Every Feels asset has a unified token, primary market, protocol-owned balance sheet, and autonomous market-making system, which can be deployed across three dimensions: spot exchange, lending, and leverage. This architecture has unique affordances: capital can flow dynamically between dimensions based on demand and market conditions, each asset autonomously shapes its own market structure. As a result, Feels lets token communities provide structural guarantees, like hard price floors, that would be impossible without this level of composability.

### Phase 1: Price Discovery and Spot

Our MVP implements a tick-based concentrated liquidity AMM with a unique architecture. Each token has exactly one market paired with FeelsSOL, creating a hub-and-spoke topology where FeelsSOL serves as the common routing asset. Every market has a dedicated protocol-owned account that holds and manages assets.

The protocol employs two core strategies. First, a bonding curve strategy handles initial token distribution and price discovery through a simple linear pricing mechanism. At the end of the bonding phase any remaining liquidity is pulled back to the protocol-owned account, leaving some ratio of the reserve asset and its pair.

The protocol-owned account deploys a second market making strategy, with liquidity placed at two critical levels. Floor liquidity is placed at a tick where the protocol can guarantee redemption of all circulating tokens, creating a hard price floor that rises monotonically as fees accumulate. Support liquidity is positioned near the bounded trading range edge, providing price stability and defending against manipulation.

The floor tick calculation is straightforward:

```
floor_price = protocol_reserves / circulating_supply
floor_tick = price_to_tick(floor_price)
```

This ensures mathematical solvency. If all token holders were to sell simultaneously, everyone could exit at the floor price.

### Phase 2: Lending

In Feels V2, the same protocol-owned account that manages spot liquidity will extend into lending. Token holders can borrow FeelsSOL against their holdings without selling, reducing downward price pressure during market stress. The protocol will dynamically allocate lending capacity from its floor liquidity based on market volatility, duration preferences, and system health.

The protocol will balance spot depth against lending demand through by measuring utilization and managing liquidity provided on each side. Durational liquidity is managed via time-weighted positions, allowing the protocol to match borrower needs with available capital. Users interact peer-to-pool with the protocol's inventory, eliminating counterparty risk and ensuring consistent pricing.

This eliminates the need for separate lending protocols while maintaining capital efficiency. The same reserves that establish a price floor can generate yield via lending when market conditions permit.

### Phase 3: Leverage

The final phase introduces leverage capabilities, transforming our markets into complete financial venues. The protocol-owned account provides peer-to-pool leveraged exposure, with funding rates naturally emerging from directional skew in the market. The protocol acts as the perpetual counterparty, managing risk through dynamic capacity limits based on volatility, funding rate adjustments to balance long/short interest, and position limits that respect overall market depth.

Under this unified risk framework, all three dimensions share the same collateral base. Risk is managed holistically within a given pool, allowing capital to flow where its needed most. This creates natural equilibrium dynamics where excess demand in one dimension can be satisfied by reallocating capital from underutilized areas.

### More Than a Market:

The culmination of this phased rollout is a fully unified market where spot, lending, and leverage are not separate primitives but integrated functions of a single, protocol-owned balance sheet. By managing capital and risk holistically, Feels offers structural guarantees and capital efficiency that would otherwise be impossible, creating a more robust and complete financial foundation for token communities.