# Design Justification: Why Thermodynamic AMM

This document explains the design constraints that led to the use of a thermodynamic-inspired model for Feels Protocol and describes why this solution is not only sound but potentially optimal for the given requirements.

## Core Design Constraints

### 1. Unified Market Structure

The protocol must integrate three distinct market types: spot exchange for token-to-token swaps with concentrated liquidity, lending markets for time-based borrowing and lending, and leverage markets for directional exposure with funding rates. Traditional DEXs treat these as separate systems, which leads to fragmented liquidity, inconsistent pricing, complex arbitrage relationships, and poor capital efficiency. The challenge is creating a single coherent system where these three market types work together harmoniously.

### 2. Self-Contained Price Discovery

The protocol cannot rely on external price oracles because tokens launched on the platform have no external markets initially. Oracle dependencies create attack vectors and compromise decentralization, which requires autonomous price discovery. Without external references, the system must discover prices organically from trading activity, maintain coherent relationships between spot, lending, and leverage prices, and resist manipulation while remaining responsive to genuine market forces.

### 3. Pool-Owned Market Making

One of the primary advantages of the protocol is that it implements a value accrual mechanism where fees accumulate to pool-owned liquidity positions. This liquidity provides a rising price floor over time, effectively converting short-term volatility into long-term value. This requires precise tracking of fee distribution, automated liquidity management, and mathematical guarantees against value leakage.

### 4. Borrowing Against Protocol Liquidity

Token holders must be able to borrow against their positions using pool-owned liquidity as collateral. This allows them to maintain long-term token exposure while accessing working capital, with predictable borrowing costs based on utilization. The challenge is integrating lending with automated market making while maintaining solvency guarantees, fair interest rate discovery, and protection against bad debt.

### 5. Leverage as a Value Creation Engine

The protocol must include leverage markets not just as a trading feature, but as a fundamental component of the value creation mechanism. Volatility generates fees, and these fees over the long-term increase the price floor. The leverage system serves three critical constituencies: traders with risk appetite can obtain leveraged exposure, token holders seeking downside protection can take the opposite side of these trades, and the protocol captures value from the induced trading activity to build durable value. This creates a virtuous cycle where short-term speculation directly funds long-term value accrual.

## Why the Thermodynamic Model is Well Suited

### 1. Natural Unification

The thermodynamic approach treats the three markets as dimensions of a single energy landscape, expressed through the potential function:

$$V = -\hat{w}_s \ln(S) - \hat{w}_t \ln(T) - \hat{w}_l \ln(L)$$

The gradient of the potential function naturally produces consistent prices across all dimensions. Cross-dimensional arbitrage opportunities emerge naturally from the physics, and all three markets share liquidity, maximizing capital efficiency. Rather than having three separate pools with their own dynamics, the system operates as a unified whole where movements in one dimension naturally affect the others through the potential landscape.

### 2. Emergent Price Discovery

In the thermodynamic model, prices emerge as gradients of the potential function ($\nabla V$ gives marginal exchange rates). Markets naturally evolve toward minimum potential, which represents equilibrium. This creates a closed thermodynamic system where energy (value) is conserved through the fundamental identity $\sum_i w_i \ln(g_i) = 0$. Prices adjust to balance supply and demand, and market depth emerges from the curvature of the potential surface. The beauty of this approach is that it requires no external inputs—prices emerge entirely from internal market dynamics.

### 3. Work-Based Fee Structure

The physics model provides an elegant solution to fee calculation through the concept of thermodynamic work:

$$W = V_{\text{end}} - V_{\text{start}}$$

$$\text{Fee} = f(W, \text{path}, \text{market state})$$

This creates natural fee emergence where fees scale with how much a trade moves the market from equilibrium. Trades that reduce potential by moving the system toward equilibrium can even earn rebates. There are no arbitrary fee tiers. Instead, fees adjust continuously based on market conditions. This work-based approach ensures that those who provide value to the system (by rebalancing it) are rewarded, while those who extract value (by pushing it further from equilibrium) pay proportionally.

### 4. Conservation Laws Enable POMM

The fundamental conservation identity ensures value preservation:

$$\sum_i w_i \ln(g_i) = 0$$

This mathematical constraint prevents value leakage, total system value remains constant during rebalancing operations. It enables precise fee tracking where every fee collected is accounted for and properly distributed. Most importantly, it supports the rising floor mechanism where pool-owned liquidity grows monotonically from accumulated fees. The conservation law acts as an unbreakable accounting principle that makes the POMM mechanism possible.

### 5. Thermodynamic Reservoir for Lending

The pool buffer ($\tau$) acts as a thermodynamic reservoir that absorbs excess energy (fees) from volatile trading and provides stable energy (liquidity) for borrowing. It maintains system balance through conservation laws, creating a natural lending market where interest rates emerge from utilization like pressure in a gas. Borrowing capacity grows with protocol success, and risk is naturally bounded by conservation constraints. This reservoir model connects trading activity to lending capacity, i.e. more trading creates more fees, which increases borrowing capacity.

## Mathematical Soundness

### Conservation Guarantees

The model enforces strict conservation at every step of operation. During instantaneous trades, fees extracted match work performed exactly. In continuous rebalancing, exponential growth factors are constrained to satisfy $\sum_i w_i \ln(g_i) = 0$. When value moves between dimensions through cross-domain flows, it is exactly preserved with no creation or destruction of value.

### Stability Properties

The thermodynamic approach provides inherent stability through several mechanisms. The potential function $V$ serves as a Lyapunov function, guaranteeing system stability. Markets naturally evolve toward balanced states through the gradient flow dynamics. Conservation laws prevent runaway conditions by ensuring bounded dynamics. This creates a self-regulating system that resists manipulation and extreme states.

### Efficiency Optimizations

While the theory uses continuous mathematics, the implementation leverages discrete structures for efficiency. Concentrated liquidity uses discrete ticks for efficient on-chain execution. Pre-computed values at tick boundaries eliminate redundant calculations. Piecewise approximations enable fast on-chain verification of complex functions. These optimizations ensure that the elegant theory translates into practical, gas-efficient implementation.

## Comparison with Alternatives

### Traditional AMM + Separate Lending + Separate Perps

The traditional approach of combining separate protocols suffers from fundamental inefficiencies. Three separate liquidity pools mean capital is fragmented and utilized poorly. There are no natural arbitrage relationships between the systems, requiring complex external mechanisms. Integration requirements become Byzantine, with different teams, codebases, and security models. Risk models are inconsistent across the different protocols, creating opportunities for exploitation.

### Other Combined Protocols

Some protocols attempt unification by bolting together existing primitives, but this approach has significant limitations. Without a coherent mathematical framework, these systems are fragile and prone to edge cases. They rely heavily on external oracles, creating dependencies and attack vectors. Most critically, they cannot support new token launches that lack external price feeds, limiting their utility for emerging projects.

### Why Thermodynamics Wins

The thermodynamic model is uniquely suited to these constraints because physical models are built around handling multiple coupled dimensions. Conservation is built into the fundamental structure rather than added as an afterthought. The equilibrium-seeking behavior mirrors how real markets behave, making the model both theoretically sound and practically effective. Continuous rebalancing follows thermodynamic relaxation principles, providing smooth, predictable evolution of market states.

## Implementation Strategy

### For Developers

The complexity of the physics model is hidden behind simple, familiar interfaces:

```typescript
// Simple interface hiding physics complexity
const swapResult = await feels.swap({
  tokenIn: "ABC",
  tokenOut: "FeelsSOL",
  amount: 1000
});
// Returns: { amountOut, fee, priceImpact }
```

Developers see a standard swap interface with predictable behavior. They benefit from deep, unified liquidity without needing to understand the underlying physics. The SDK handles all complexity, providing a seamless integration experience.

### For Traders

Traders interact with a familiar swap interface that provides predictable fees based on trade size and market impact. They benefit from deep liquidity across all three market types and can even earn rebates for trades that help rebalance the system. The unified liquidity means better prices and lower slippage compared to fragmented systems.

### For Protocols

Other protocols can achieve single integration for spot, lending, and leverage functionality. Operations are composable, allowing complex strategies to be built on top. The transparent fee structure makes it easy to predict costs and optimize routing. This creates a powerful building block for the broader DeFi ecosystem.

## Conclusion

The thermodynamic AMM design provides an intuitive framework to understand a complex set of constraints. Allowing us to think about the unification of three markets in a mathematically coherent way. It enables self-contained price discovery without external dependencies, solving a critical problem for new token launches. The novel POMM mechanism with rising price floors creates sustainable value accrual. Support for borrowing against protocol-owned liquidity provides utility beyond simple trading. Mathematical guarantees of value conservation ensure system integrity.

The physics framework provides both theoretical soundness and practical benefits. The result is more capital efficient than separate markets because liquidity is shared across all functions. It's more robust than oracle-dependent systems because prices emerge from internal dynamics. The value accrual mechanism is aligned with building a healthy market structure, making it useful for new token launches that need bootstrap liquidity and price discovery.