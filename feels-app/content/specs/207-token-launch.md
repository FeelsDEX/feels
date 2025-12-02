# **Launching New Markets with Feels**

**Launch Overview**

A new market launches with spot and lending active. The market begins with three vaults. Each vault holds a fixed amount of the coin. Users deposit the reserve asset to acquire the coin or to open a lending position. Each vault prices its supply through a linear set of tick prices. Early buyers get lower prices. Later buyers move through higher ticks.

**Spot Vault**

The spot vault gives users a spot position token. Holders of this token may redeem it at any time to receive the underlying coin. The vault prices its supply through its own fixed tick prices. Under the hood, the vault provides liquidity to the JIT liquidity system. The JIT system uses this capital to supply reactive liquidity during swaps. Users who want immediate exposure to the coin choose this path, and they may redeem the position token right away if they want the underlying asset.

**Duration Vaults**

The duration vaults gives user vault position tokens. There are 3 durations vaults, short, medium, and long, with 7 day, 21 day, and 49 day lock terms respectively.  Users deposit reserve assets into the vault, the vault opens loans for the corresponding duration. On deposit, users receive position tokens that are redeemable at the end of the term.

The starting price level for each vault differs depending on the term length. The starting price for longer terms is lower than the starting price for shorter terms reflecting the time value of money.

**Vault Mechanics**

Each vault has a warmup period. The warmup period gives time for initial price discovery to occur. During this period the liquidity structure for each vault is fixed. The linear tick prices remain unchanged until the warmup period ends. The vault does not LP any funds it has received from user deposits until this period is over.

When the warmup period ends, each vault repositions all of its liquidity according to its steady state strategy. The spot vault deposits its liquidity into the JIT system. The JIT system provides liquidity at the time weighted average price.

The short term vault and long term vault reposition their liquidity in a similar way. They use historical price information from previous terms to place liquidity at a time weighted target price level.

**Vault Mechanics**

Each vault moves through its ticks in order. Buyers always consume the cheapest available tick in the chosen vault. The vaults do not share liquidity with each other. Each vault has its own price range and allocation. When a vault sells out its ticks the vault is complete. The vault configuration and liquidity positioning used here is the same for all markets, so each launch follows the same pattern.

**Initial Buy**

The creator can perform an initial buy during deployment. This uses a hook that allows the creator to execute a purchase instruction at the moment the market launches. It is a fair launch mechanism because the creator only receives the chance to enter first, and the vault pricing remains unchanged.

**Combined Market Structure**

The three vaults provide three paths into the new market. One path gives instant access to the coin. One path gives lower prices for a short commitment. One path gives the best price for a long commitment. This design creates a clear tradeoff between immediacy and cost. All vaults are created at launch, so price discovery for the three durations begins at the same time. This forms a simple yield curve. It also seeds the market with short and long duration liquidity from the first block.

**Vault Comparison**

| **Vault Type** | **Asset User Receives** | **User Lockup Duration** | **Starting Price Level** | **Market Effect** |
| --- | --- | --- | --- | --- |
| Spot | Spot vault position token | 0 days | Highest | Supplies Jit liquidity which can be withdrawn on demand |
| Short term | Short term vault position token | 7 days | High | Supplies short duration liquidity |
| Medium term | Medium term vault position token | 21 days | Medium | Supplies medium duration liquidity |
| Long term | Long term vault position token | 49 days | Lowest | Supplies long duration liquidity |