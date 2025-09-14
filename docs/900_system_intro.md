# Feels Walkthrough (Protocol vs Pool)

## Baseline 101: Monetizing Volatility

Baseline's core value proposition is converting short-term volatility into a long-term price floor, what practitioners call "up-only" technology.

Their mechanism is simple but robust. At its core, Baseline operates around a tick-based automated market maker (AMM) using a liquid reserve asset (ETH on Ethereum, we'll be using SOL on Solana).

> Background: Tick-based DEXes can be understood as pseudo-order books, where liquidity providers place bids and asks at specific price points. Each tick represents a discrete price level where liquidity can be concentrated, similar to placing a limit order at a specific price with a desired magnitude (e.g., "10 TokenA available at 0.5 TokenA/TokenB").

Baseline's key innovation is how it directs swap fees. Baseline directs trading fees to a pool-owned account. This account then provides liquidity back on the DEX in a specific range that acts as a price floor.

The protocol incrementally adjusts this floor liquidity tick upward over time. Floor liquidity is priced to ensure the protocol stays solvent:

$$\text{floor price} = \frac{\text{quantity reserve assets}}{\text{quantity circulating assets}}$$

This equation guarantees that if every token holder attempted to exit simultaneously, the protocol could fully redeem all tokens at the floor price.

As the protocol accumulates assets through the initial sale of tokens, or through fees, the price at which it can place floor liquidity incrementally moves upward.

## Capital Efficiency

If we're providing tick liquidity at a price no one wants, the obvious question is can we use the capital more effectively?

Yes. We create a lending facility, where token holders from a given pool can borrow floor liquidity against their tokens. This mechanism serves multiple purposes. Holders of illiquid tokens don't need to sell their positions when they need working capital, pushing the price down, instead they can borrow against their holdings. This feature is particularly attractive for new projects as it dampens downward price pressure, while providing some immediate utility beyond speculation.

The lending system also enables leverage. Users can speculate on their tokens through "looping," borrowing the reserve asset against their existing holdings and swapping to acquire more tokens.

## Fundamental Limitations

Despite its elegance, Baseline's approach has several fundamental limitations.

First, the borrowing facility faces strict solvency constraints tied to floor liquidity. The protocol cannot make guarantees about loan duration, why Baseline is now bolting on a staking system to induce users to make durational commitments.

System complexity is also a challenge. You're effectively building four separate systems (five if you count leverage):
- A DEX for spot trading
- A lending protocol for borrowing
- A market maker for liquidity provision  
- A vault system for staking

Ensuring these components don't interfere with one other while maintaining system coherence becomes increasingly difficult as the protocol matures.

I would also argue that the leverage mechanism doesn't fully align with what Degens want. While the system provides leverage it's in the form of collateral-constrained looping strategies that magnify yield (essentially a carry trade).

Real degenerates prefer perpetual contracts because they can magnify their entire position without the friction of repeated borrowing and swapping cycles. On top of that perps are unconstrained by collateral composition, instead compensating risk with a continuous funding rate.

## The Feels MVP

The Feels MVP begins with clear design constraints drawn from Baseline's lessons:
- Must use a tick-based AMM
- Must charge fees on swaps
- Fees must flow to pool-owned accounts
- Must have a strategy for determining the placement of pool-owned liquidity placement

Additionally, Feels must support permissionless token creation and market launch. The current standard, is to discover prices using a bonding curve. This curve behaves like a virtual, self-contained Automated Market Maker (AMM) with a constant-product (`x*y=k`) formula.

The key difference from a standard AMM is that during this initial phase, the protocol itself is the sole counterparty and no outside liquidity providers are allowed. When a user buys, tokens are dispensed according to the curve's price, and when they sell, the tokens are returned to the protocol. This creates a predictable price ramp for initial distribution. Once a certain market cap is reached, the token "graduates": a real, open AMM pool is seeded with a portion of the capital raised, and third-party liquidity provision is enabled.

Feels adopts this same two-phase philosophy. The "Price Discovery" phase uses the protocol's native concentrated liquidity (CLMM) engine to create a bonding curve where the protocol is the sole LP. Once the token graduates, the pool enters its "Steady-State" phase. The capital raised is automatically re-allocated to fund the long-term Floor and JIT liquidity strategies, and the pool is opened for public liquidity providers to join. While there are many ways to improve this mechanism, the meme coin community is familiar with this pattern, making it a good place to start.

We also introduce a clever trick with Feels. In the reserve asset layer we convert a user's SOL into JitoSOL and create a synthetic FeelsSOL token which tracks the SOL price. This allows us to capture staking yield from JitoSOL and automatically directs it into floor liquidity, creating an additional value accrual mechanism beyond trading fees. It also makes for better price charts, because the baseline price will rise in tandem with Jito rewards.

## Feels V2

Rather than building spot trading, lending, and leverage as separate protocols that must be carefully integrated, we're stepping back and designing Feels in a way where we can evolve into a unified system.

### AMM Background

To understand Feels' design, we'll first need a little background on other AMM designs:

**Uniswap V2's** invariant curve uses the constant product formula $x \cdot y = k$. Here $x$ and $y$ represent token reserves and $k$ remains constant during swaps. The instantaneous price equals the ratio of reserves. For token X priced in Y, this gives us $p = y/x$. As traders buy one token, its reserve shrinks and its price rises nonlinearly along a hyperbola. Liquidity depth is represented by the magnitude of reserves—larger reserves create flatter curves near the current price, reducing slippage.

**Balancer** extends this concept beyond two equally-weighted tokens. Instead of the product of two 50/50 token balances stays constant, Balancer allows any number of tokens with custom weights. The invariant generalizes to "the combined weighted balance of all tokens must stay constant." Prices emerge from how far one token's balance shifts relative to others, adjusted by its assigned weight.

**Uniswap V3** introduces a tick system and active market making to achieve better capital efficiency. Instead of one continuous curve, the pool consists of discrete segments, where market makers can create granular positions. A provider can specify "my liquidity only operates between \$1,500-\$2,000 ETH/USDC." This concentrates capital where it's most needed, providing deeper liquidity and lower slippage within active ranges while allowing capital to disappear from inactive ranges.

### 3D Markets

Feels V2 starts with the premise: "everything is lending."
- Loans are lending (obviously)
- Leverage is lending (you're borrowing exposure, paying through risk acceptance)
- And swaps can be viewed as asymptotic lending (a collateralized loan with infinite duration)

This unified view allows us to model spot exchange, duration-based loans, and leveraged positions through the same mathematical framework. These become three dimensions: price, duration, and leverage, each with two sides:
- **Price**: Token A ↔ Token B
- **Duration**: Borrow ↔ Lend  
- **Leverage**: Short ↔ Long

Each dimension forms a complete market with liquidity spanning from one side to the other.

Instead of discrete swaps between tokens, Feels implements a Balancer-style invariant model that allows movement anywhere in this three-dimensional space:

$$K_{\text{trade}} = S^{\hat{w}_s} \cdot T^{\hat{w}_t} \cdot L^{\hat{w}_l}$$

Where $S$, $T$, and $L$ represent the spot, time, and leverage dimensions respectively. The $\hat{w}$ values are normalized weights.

A swap between TokenA and TokenB settles quantities of physical assets, while moving into the lending or leverage dimensions creates synthetic positions backed by the protocol.

### Accounting for Risk

To fit this 3D model into a generalized Balancer framework, Feels adds a crucial innovation: explicit risk terms. While Uniswap and Balancer price risk implicitly through "liquidity risk," Feels makes risk explicit in each dimension:

$$D = \prod_{i} \left( \frac{C_i}{\sqrt{1 + \rho_i^2}} \right)^{w_i}$$

Where $C_i$ represents capacity and $\rho_i$ represents risk in each component. This allows the protocol to:
- Price **spot risk** based on *price volatility*
- Price **duration risk** based on *interest rate volatility*
- Price **leverage risk** based on *directional skew*

These risk terms enable persistent differentials in exposure and create natural funding payments for taking one side versus another in any dimension.

### Financial Physics

We draw on another familiar invariant system, thermodynamics, to help us visualize and formalize our mathematical model. The fundamental identity:

$$\sum_i w_i \ln g_i = 0$$

ensures that no value is created or destroyed. Here $w_i$ represents the weight of each dimension and $g_i$ represents the growth factor (rebase multiplier) for that dimension.

The system handles continuous flows (interest accrual and funding payments) through exact exponential growth factors. A rebasing mechanism settles all flow calculations atomically, guaranteeing exact value preservation.

Feels prices trades based on thermodynamic work. The energy required to move the system from one state to another:

$$W = V_{\text{end}} - V_{\text{start}}$$

Where $V$ is the thermodynamic potential. Trades that push the system away from equilibrium pay fees proportional to work performed. Trades that move the system toward equilibrium can even earn rebates, creating natural incentives for beneficial rebalancing.

The framework resembles the Gibbs free energy equation $\Delta G = \Delta H - T\Delta S$. Here the enthalpy and entropy terms represent potentials related to instantaneous state change and continuous dissipation respectively. In our system, the thermodynamic potential $V$ corresponds to the free energy change $\Delta G$, capturing both the immediate work required for state transitions and the ongoing energy costs of maintaining positions over time.
