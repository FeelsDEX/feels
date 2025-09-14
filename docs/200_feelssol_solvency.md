# Feels Protocol Solvency

## Executive Summary

This document analyzes the solvency mechanics of the Feels Protocol, a thermodynamic AMM on Solana that creates isolated trading pools backed by a unified JitoSOL reserve system. The protocol's two-layer architecture—isolated pools managing FeelsSOL distribution and a protocol-level JitoSOL backing system—provides strong solvency guarantees through natural fault isolation and conservation laws.

**Key Finding**: With perfect conservation in isolated pools and JitoSOL's staking yield accumulation, protocol-level insolvency is extremely unlikely, with solvency ratios improving over longer time horizons despite short-term market fluctuations.

## 1. System Architecture

### 1.1 Three-Token System

The Feels Protocol operates with three distinct asset types:

1. **JitoSOL**: The backing asset held in protocol reserves
   - Liquid staking token that earns staking rewards over time
   - Subject to market dynamics and liquidity premiums/discounts
   - Protocol's primary reserve asset for all redemptions
   - Maintained in protocol-controlled vaults

2. **FeelsSOL**: The hub token for all trading activity
   - Minted 1:1 against deposited JitoSOL
   - Tracks SOL price (not JitoSOL price) for UI simplicity
   - Can exist in two states: user-held or pool-escrowed

3. **Pool Tokens**: Individual meme/project tokens (e.g., $MEME)
   - Created and traded within isolated pool environments
   - No direct redemption rights to underlying assets
   - Value derived entirely from pool-internal market dynamics

### 1.2 Flow Architecture

```
User JitoSOL → Protocol Vault → Mint FeelsSOL (1:1 ratio)
User FeelsSOL → Pool Escrow → Trade for Pool Tokens
Pool Tokens → Pool Escrow → Redeem FeelsSOL
User FeelsSOL → Protocol Vault → Redeem JitoSOL
```

### 1.3 Isolation Design

**Pool Isolation**: Each trading pool (e.g., FeelsSOL/MEME) operates as a completely isolated system:
- Pool-escrowed FeelsSOL cannot migrate between pools
- Pool tokens cannot be redeemed for assets outside their pool
- Pool failures cannot directly impact other pools or protocol reserves

**FeelsSOL Reserve**: All FeelsSOL redemptions are backed by a unified JitoSOL reserve system:
- Single JitoSOL vault backs all FeelsSOL regardless of origin
- Protocol maintains aggregate solvency across all pools
- JitoSOL appreciation benefits all FeelsSOL holders equally

## 2. Two-Layer Solvency Model

### 2.1 Layer 1: Pool-Level Solvency

**Definition**: A pool is solvent when it contains sufficient FeelsSOL liquidity to facilitate all reasonable exit scenarios for its tokens.

**Pool Solvency Constraint**:

$$
\text{Available\_FeelsSOL\_in\_Pool} \geq \text{Required\_FeelsSOL\_for\_Market\_Exit}
$$

**Key Properties**:
- Each pool maintains its own FeelsSOL escrow balance
- Pool solvency is independent of other pools
- Pool insolvency affects only that specific market
- Protocol-owned floor liquidity provides base-case exit capacity

### 2.2 Layer 2: Protocol-Level Solvency

**Definition**: The protocol is solvent when its JitoSOL reserves can redeem all outstanding FeelsSOL tokens.

**Protocol Solvency Constraint**:

$$
\text{JitoSOL\_Reserves} \geq (\text{User\_Held\_FeelsSOL} + \sum \text{Pool\_Escrowed\_FeelsSOL})
$$

**Key Properties**:
- Unified backing system for all FeelsSOL
- Solvency ratio generally improves over longer timeframes due to staking yield accumulation
- Subject to short-term market volatility in JitoSOL/SOL exchange rates
- Independent of individual pool performance
- Natural safety margin from staking rewards, tempered by market dynamics

## 3. Risk Analysis and Mitigations

### 3.1 Pool-Level Risks

#### Risk 1: Liquidity Concentration Risk
**Scenario**: Liquidity providers withdraw from critical price ranges, creating exit bottlenecks.

**Impact**: Users cannot convert pool tokens back to FeelsSOL at reasonable prices.

**Probability**: Medium - Natural during market stress

**Mitigation**:
- Protocol-owned floor liquidity provides guaranteed exit capacity
- Wide tick ranges (-100,800 to +100,800) ensure broad coverage
- Dynamic fee scaling during volatility to incentivize LP retention
- Emergency liquidity injection mechanisms from protocol reserves

#### Risk 2: Extreme Price Volatility
**Scenario**: Pool token experiences rapid price collapse, overwhelming available FeelsSOL liquidity.

**Impact**: Severe slippage for exits, potential temporary illiquidity.

**Probability**: High for speculative tokens

**Mitigation**:
- Isolated pools prevent contagion to other markets
- Protocol-owned positions earn fees from increased volatility
- Natural bounds: maximum loss limited to pool's FeelsSOL reserves
- Concentrated liquidity automatically adjusts to price movements

#### Risk 3: Coordinated Exit Attacks
**Scenario**: Large coordinated selling pressure attempts to drain pool FeelsSOL reserves.

**Impact**: Temporary exit difficulties, potential pool illiquidity.

**Probability**: Low - Requires significant coordination and capital

**Mitigation**:
- First-come-first-served exit processing
- Pool isolation prevents spillover effects
- Protocol floor liquidity acts as ultimate backstop
- Attack cost scales with pool size, making large attacks expensive

### 3.2 Protocol-Level Risks

#### Risk 1: Implementation Bugs Violating Conservation
**Scenario**: Software bugs allow FeelsSOL creation without corresponding JitoSOL backing.

**Impact**: Protocol insolvency if FeelsSOL supply exceeds backing.

**Probability**: Low - Mitigated by testing and audits

**Mitigation**:
- Explicit conservation invariant checks: $\sum w_i \ln(g_i) = 0$
- Real-time solvency monitoring: $\text{JitoSOL\_reserves} \geq \text{FeelsSOL\_supply}$
- Formal verification of critical minting/burning functions
- Multi-signature requirements for system parameter changes

#### Risk 2: Administrative Access Control Failures
**Scenario**: Compromised admin keys allow unauthorized JitoSOL withdrawals or FeelsSOL minting.

**Impact**: Direct protocol insolvency through reserve depletion.

**Probability**: Low - Depends on key management practices

**Mitigation**:
- Multi-signature wallets for all administrative functions
- Time-locked withdrawals for large amounts
- Automated monitoring for unusual administrative activity
- Separation of operational and emergency key sets

#### Risk 3: JitoSOL Market and Systematic Risk
**Scenario**: JitoSOL experiences market volatility, liquidity crises, slashing events, smart contract bugs, or validator failures.

**Impact**: Backing asset trades below its fundamental value relative to SOL, temporarily reducing effective protocol reserves.

**Probability**: 
- Market volatility: Medium - Natural due to delayed unstaking and liquidity dynamics
- Systematic failures: Very Low - JitoSOL has strong operational history

**Mitigation**:
- Conservative oracle design using minimum of available rates
- Safety buffers in exchange rate calculations (0.5% in Phase 1)
- Hybrid oracle approach incorporating both protocol and market rates
- Real-time monitoring of JitoSOL health metrics and market spreads
- Potential future diversification to multiple liquid staking tokens

#### Risk 4: Precision and Rounding Errors
**Scenario**: Cumulative rounding errors in fee calculations, rebasing, or price updates.

**Impact**: Gradual erosion of conservation properties over time.

**Probability**: Medium - Inherent to high-frequency operations

**Mitigation**:
- High-precision arithmetic libraries (Q64.64 for prices)
- Explicit precision bounds checking
- Periodic reconciliation of calculated vs. actual balances
- Conservative rounding always favoring protocol solvency

## 4. Worst-Case Exit Scenarios

### 4.1 Individual Pool Collapse

**Scenario**: A popular meme token crashes to near-zero value with massive selling pressure.

**Process**:
1. Users rush to convert pool tokens → FeelsSOL
2. Pool FeelsSOL liquidity depletes rapidly
3. Later sellers face severe slippage or temporary illiquidity
4. Pool floor liquidity provides final exit capacity

**Protocol Impact**: None - Pool isolation prevents contagion

**Resolution**: Pool-specific issue resolves independently, protocol remains fully functional

### 4.2 Multiple Pool Stress

**Scenario**: Market-wide crash affects multiple pools simultaneously.

**Process**:
1. Coordinated selling across multiple pools
2. Multiple pools experience liquidity stress
3. Pool-owned positions across pools face impermanent loss
4. Some pools may become temporarily illiquid

**Protocol Impact**: Minimal - Each pool's maximum loss bounded by its FeelsSOL reserves

**Resolution**: Long-term JitoSOL yield accumulation and fee collection from increased volatility help offset losses, though short-term market dynamics may temporarily impact effective reserves

### 4.3 Complete System Stress Test

**Scenario**: Every pool experiences maximum selling pressure simultaneously.

**Mathematical Analysis**:

$$
\text{Maximum\_Possible\_Loss} = \sum \text{Max\_Loss\_Per\_Pool}_i
$$

Where: $\text{Max\_Loss\_Per\_Pool}_i = \text{FeelsSOL\_Escrowed}_i$

$$
\text{Total\_Protocol\_Exposure} = \sum \text{FeelsSOL\_Escrowed}_i \leq \text{Total\_FeelsSOL\_Supply}
$$

Since: $\text{JitoSOL\_Reserves} \geq \text{Total\_FeelsSOL\_Supply} \times (1 + \text{Cumulative\_Yield} - \text{Market\_Discount})$

Therefore: Protocol remains solvent under maximum stress assuming market discount remains below cumulative yield

**Result**: Protocol maintains redemption capacity as long as JitoSOL's long-term yield accumulation exceeds any temporary market discounts and maximum theoretical losses.

## 5. Solvency Invariants

### 5.1 Conservation Invariant

$$
\sum w_i \ln(g_i) = 0 \quad \text{(across all system participants)}
$$

**Meaning**: Total value in the system cannot increase or decrease through internal operations.

### 5.2 Backing Invariant

$$
\text{JitoSOL\_Reserves} \geq \text{FeelsSOL\_Total\_Supply}
$$

**Meaning**: Protocol always holds sufficient backing assets for full redemption.

### 5.3 Supply Invariant

$$
\text{FeelsSOL\_Total\_Supply} = \text{User\_Held\_FeelsSOL} + \sum \text{Pool\_Escrowed\_FeelsSOL}
$$

**Meaning**: All FeelsSOL is accounted for in either user wallets or pool escrows.

### 5.4 Isolation Invariant

$$
\text{Pool}_i\_\text{FeelsSOL\_Outflow} \leq \text{Pool}_i\_\text{FeelsSOL\_Inflow}
$$

**Meaning**: No pool can distribute more FeelsSOL than was deposited into it.

### 5.5 Long-Term Appreciation Tendency

$$
\lim_{T \to \infty} \frac{1}{T} \sum_{t=0}^{T} [\text{JitoSOL\_Rate}(t) - \text{JitoSOL\_Rate}(0)] > 0
$$

**Meaning**: JitoSOL's protocol exchange rate trends upward over long periods due to staking rewards, though market rates may fluctuate below this value due to liquidity dynamics and redemption delays.

## 6. Oracle Architecture (Layered)

The system separates oracle responsibilities by layer to match solvency responsibilities:

- Protocol: `protocol::Oracle` provides a conservative FeelsSOL↔JitoSOL exchange rate for global backing and redemption. Start with Jito native rate plus safety buffer; later, optionally validate with DEX TWAP and take the minimum with divergence guards.
- Pool: `pool::Oracle` provides the per‑pool GTWAP used by pool subsystems (fees, JIT, floor). See 204_pool_oracle.md for full design.

### 6.1 Protocol Oracle Requirements

The protocol oracle must:
- Provide conservative valuation of JitoSOL backing, biasing safety over precision
- Resist manipulation via monotonic protocol rate and/or conservative min(protocol, market) composition
- Maintain availability and expose health/staleness signals

### 6.3 Data Sources Analysis

#### Option A: Jito Protocol Native Rate
**Mechanism**: Use Jito's internal calculation of accumulated staking rewards.

**Formula**: $\text{Rate} = \frac{\text{Total\_Staked\_SOL\_Value}}{\text{JitoSOL\_Token\_Supply}}$

**Advantages**:
- Most authoritative source reflecting actual staking performance
- Immune to market manipulation
- Always available regardless of market conditions
- Monotonically increasing at the protocol level, supporting long-term solvency

**Disadvantages**:
- Ignores liquidity premiums/discounts in secondary markets
- Single point of failure if Jito's calculation is compromised
- May not reflect immediate redemption constraints

#### Option B: DEX TWAP (JitoSOL/SOL Markets)
**Mechanism**: Time-weighted average price from on-chain DEX trading.

**Formula**: $\text{TWAP} = \frac{\sum(\text{Price}_i \times \text{Duration}_i)}{\text{Total\_Duration}}$

**Advantages**:
- Reflects actual market trading and liquidity conditions
- Incorporates premium for immediate vs. delayed redemption
- Transparent and verifiable on-chain
- Market-driven price discovery

**Disadvantages**:
- Vulnerable to manipulation if liquidity is insufficient
- May trade at discount during market stress
- Dependent on DEX liquidity and functionality
- Added complexity in implementation

### 6.4 Protocol Reserve Oracle Design

The reserve oracle design serves as the protocol's critical price discovery mechanism, determining the exchange rate between JitoSOL and FeelsSOL for all minting and redemption operations. Its primary purpose is to protect protocol solvency while providing accurate pricing that reflects the fundamental value of staked SOL, gradually incorporating real-time market signals.

The system starts conservatively by using only Jito's protocol rate (with a safety buffer) to ensure the protocol never overvalues its backing assets. As the system matures, it gradually incorporates market signals from DEX trading to capture liquidity premiums and provide more responsive pricing.

The protocol oracle integrates with the broader system architecture:

**Phase 1: Conservative Foundation (Launch)**
```rust
impl ProtocolOracle {
    pub fn get_exchange_rate_v1(&self) -> Result<u64> {
        let jito_rate = self.backing.jito_rate;
        let conservative_rate = jito_rate * 9950 / 10000;  // 0.5% safety buffer
        Ok(conservative_rate)
    }
    
    pub fn update(&mut self, slot: u64, tick: i32, timestamp: i64) -> Result<()> {
        // Protocol oracle: update backing rate if needed (pool GTWAP is separate)
        // Update backing rate if needed
        if slot > self.backing.last_jito_update + UPDATE_INTERVAL {
            self.backing.update_jito_rate()?;
        }
        
        self.last_update = slot;
        self.update_health_status()?;
        Ok(())
    }
}
```

**Phase 2: Market Integration (Post-Launch)**
```rust
fn get_exchange_rate_v2() -> Result<u64> {
    let jito_rate = get_jito_native_rate()?;
    let market_rate = get_dex_twap(1800)?;  // 30-minute TWAP
    
    let divergence_bps = abs_diff(jito_rate, market_rate) * 10000 / jito_rate;
    
    if divergence_bps < 25 {  // < 0.25% divergence
        Ok(market_rate)
    } else {
        Ok(min(jito_rate, market_rate))  // Conservative choice
    }
}
```


### 6.3 Protocol Safety Controller

The Safety Controller manages risk on behalf of the protocol, monitoring the health of all critical components and coordinating protective responses across subsystems. Its primary purpose is to prevent cascading failures by detecting anomalies early and implementing graduated responses, from gentle rate limiting during minor stress to full system pauses during critical events.

Rather than each subsystem implementing its own safety mechanisms (which could conflict or create gaps), the Safety Controller provides consistent, protocol-wide protection. It tracks the health of oracles, liquidity conditions, and solvency metrics, automatically degrading service quality rather than failing completely when issues arise. This "graceful degradation" approach ensures the protocol remains usable even under adverse conditions while protecting user funds above all else.

The oracle system integrates with a protocol-wide safety controller:

```rust
pub struct SafetyController {
    // Component health tracking
    pub oracle_health: HealthStatus,
    pub liquidity_health: HealthStatus,
    pub solvency_health: HealthStatus,
    
    // Global controls
    pub global_pause: bool,
    pub degraded_mode: bool,
    
    // Shared rate limiter
    pub rate_limiter: RateLimiter,
}

pub struct HealthStatus {
    pub is_healthy: bool,
    pub last_healthy_slot: u64,
    pub error_count: u16,
    pub degradation_level: u8,  // 0 = healthy, 1-3 = degraded, 4+ = critical
}

impl SafetyController {
    pub fn check_oracle_update(
        &self,
        new_rate: u64,
        previous_rate: u64,
        slot: u64
    ) -> Result<u64> {
        // Coordinate with global safety state
        if self.global_pause {
            return Err(ErrorCode::GlobalPause);
        }
        
        // Apply rate limiting
        if !self.rate_limiter.check_oracle_update(slot)? {
            return Ok(previous_rate);  // Rate limited
        }
        
        // Velocity check
        let hourly_change = calculate_hourly_change_bps(new_rate, previous_rate);
        if hourly_change > 10 && self.oracle_health.degradation_level > 0 {
            // Tighter limits during degraded state
            return Ok(previous_rate);
        }
        
        // Rate protection: Use minimum of current and previous for conservative approach
        // Note: While protocol rate is monotonic, market rates can fluctuate
        if new_rate < previous_rate {
            // Log the event but allow it for market-based oracles
            msg!("Oracle rate decreased from {} to {}", previous_rate, new_rate);
            
            // For protocol rates, maintain monotonicity
            // For market rates, allow decrease but apply additional safety checks
            if self.oracle_source == OracleSource::Protocol {
                return Ok(previous_rate);
            }
        }
        
        Ok(new_rate)
    }
}
```

### 6.4 Oracle Evolution

**Immediate Implementation**: Start with Jito's native rate plus safety buffer for maximum security and simplicity.

**Short-term Enhancement**: Add DEX TWAP as validation source, using minimum of available rates for conservative estimates.

**Long-term Sophistication**: Implement full multi-source consensus mechanism with confidence scoring and dynamic weighting.

**Key Principles**:
- Conservative bias favoring protocol solvency over precise pricing
- Gradual complexity increase as system proves stable
- Multiple fallback mechanisms for edge cases
- Transparent and auditable calculations

## 7. Pool Floor Management

### 7.1 Floor (Pool) Architecture

Each pool employs a `pool::Floor` that provides consistent floor calculations across all subsystems:

```rust
pub struct PoolFloor {
    pub current_floor: i32,
    pub floor_buffer: i32,
    pub last_ratchet_slot: u64,
    pub appreciation_rate: u16,
    pub total_feels_supply: u128,
    pub jitosol_reserves: u128,
}

impl PoolFloor {
    pub fn calculate_floor_tick(&self) -> i32 {
        // Floor = JitoSOL reserves / FeelsSOL supply
        let floor_price = (self.jitosol_reserves * PRECISION) / self.total_feels_supply;
        price_to_tick(floor_price)
    }
    
    pub fn can_ratchet(&self, current_slot: u64) -> bool {
        current_slot > self.last_ratchet_slot + RATCHET_COOLDOWN
    }
    
    pub fn get_safe_ask_tick(&self) -> i32 {
        self.current_floor + self.floor_buffer
    }
    
    pub fn update_after_swap(&mut self, jitosol_change: i128, slot: u64) {
        // Update reserves based on JitoSOL appreciation
        if jitosol_change > 0 {
            self.jitosol_reserves = self.jitosol_reserves
                .saturating_add(jitosol_change as u128);
        }
        
        // Check for ratchet opportunity
        if self.can_ratchet(slot) {
            let new_floor = self.calculate_floor_tick();
            if new_floor > self.current_floor {
                self.current_floor = new_floor;
                self.last_ratchet_slot = slot;
            }
        }
    }
}
```

This system:
- Provides single source of truth for floor calculations
- Ensures consistent floor enforcement across dynamic fees, JIT liquidity, and solvency checks
- Enables atomic floor updates that propagate to all systems
- Maintains the monotonic floor property through ratchet mechanism
- Accounts for potential market volatility in JitoSOL rates through conservative calculations

## 8. Hierarchical Parameter Management

The protocol employs a hierarchical parameter system that simplifies governance while ensuring consistency across all subsystems:

```rust
// Core parameters that drive the entire system
pub struct CoreParameters {
    pub risk_tolerance: u16,      // 0-100 scale
    pub responsiveness: u16,      // 0-100 scale  
    pub floor_safety_margin: u16, // Basis points
}

// Solvency-specific parameters derived from core
pub struct SolvencyParameters {
    pub oracle_safety_buffer_bps: u16,
    pub rate_limit_hourly_change_bps: u16,
    pub ratchet_cooldown_slots: u64,
    pub min_divergence_for_market_rate: u16,
}

impl SolvencyParameters {
    pub fn from_core(core: &CoreParameters) -> Self {
        Self {
            // More conservative with lower risk tolerance
            oracle_safety_buffer_bps: 50 + (50 - core.risk_tolerance / 2) as u16,
            rate_limit_hourly_change_bps: 10 + (20 - core.risk_tolerance / 5) as u16,
            ratchet_cooldown_slots: 1800 + (600 - core.responsiveness as u64 * 6),
            min_divergence_for_market_rate: 25 - (core.risk_tolerance / 4) as u16,
        }
    }
}
```

This approach ensures that parameter changes have predictable, system-wide effects while maintaining flexibility for fine-tuning specific subsystems.

## 9. Component Integration

The solvency system integrates with all four protocol components:

### 9.1 Protocol Oracle
- Provides FeelsSOL↔JitoSOL exchange rates for protocol solvency calculations
- Monitors divergence between protocol and market rates (if integrated)
- Ensures conservative valuation of backing assets

### 9.2 Pool Floor
- Maintains each pool's floor price
- Ensures pool operations respect minimum solvency constraints
- Provides ratcheting mechanism for monotonic floor improvement

### 9.3 SafetyController
- Monitors overall protocol health
- Implements circuit breakers for extreme conditions
- Coordinates emergency responses across all subsystems

### 9.4 FlowSignals
- Tracks market flow patterns that might indicate solvency risks
- Provides early warning signals for potential bank runs
- Feeds into dynamic risk assessment

## 10. Conclusions and Recommendations

### 10.1 Solvency Assessment

The Feels Protocol's two-layer architecture with isolated pools provides exceptionally strong solvency guarantees:

**Protocol-Level Solvency**: Strong solvency guarantees under normal conditions, with backing ratios that generally improve over longer timeframes due to staking yield accumulation, despite potential short-term market volatility in JitoSOL/SOL rates.

**Pool-Level Resilience**: Natural fault isolation prevents cross-contamination while maintaining individual market functionality.

### 10.2 Risk Prioritization

**Highest Priority**: Implementation bug prevention through comprehensive testing, formal verification, and security audits.

**Medium Priority**: Robust oracle design with conservative bias and multiple fallback mechanisms.

**Lower Priority**: Pool-level liquidity management, as isolation prevents systemic impact.

### 10.3 Implementation Recommendations

1. **Deploy Unified Components First**: Begin with protocol::Oracle, pool::Floor, and SafetyController as foundational infrastructure.

2. **Start Conservative**: Begin with simple, secure oracle design and gradually add sophistication.

3. **Monitor Continuously**: Implement real-time solvency monitoring with automated alerts through the unified SafetyController.

4. **Plan for Edge Cases**: Design fallback mechanisms for every identified failure mode.

5. **Maintain Transparency**: Ensure all oracle calculations and solvency metrics are publicly verifiable.

6. **Leverage Shared Infrastructure**: Use unified components to reduce code duplication and ensure consistency.

The protocol's innovative architecture successfully addresses the fundamental challenge of maintaining solvency across multiple speculative markets while providing strong guarantees to users. The mathematical foundation, combined with practical safety measures, creates a robust system capable of withstanding extreme market conditions.
