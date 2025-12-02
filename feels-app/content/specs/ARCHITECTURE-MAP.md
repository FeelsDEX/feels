---
title: "Architecture Map"
description: "System architecture diagrams and component relationships"
category: "Reference"
order: 0
---

# Architecture Map

Visual guides to system architecture, component relationships, and data flow.

## System Overview Diagram

High-level view of the Feels Protocol architecture showing the hub-and-spoke model and key components.

```mermaid
graph TB
    subgraph Users
        Trader[Traders]
        LP[Liquidity Providers]
        Creator[Token Creators]
    end
    
    subgraph "Protocol Layer"
        FeelsSOL[FeelsSOL Hub Token]
        JitoVault[JitoSOL Vault]
        Treasury[Protocol Treasury]
        ProtocolOracle[protocol::Oracle]
    end
    
    subgraph "Pool Layer (per market)"
        Pool[Pool/Market]
        PoolOracle[market::Oracle GTWAP]
        PoolFloor[market::Floor]
        PoolController[PoolController]
        PoolReserve[PoolReserve Strategic]
        PoolBuffer[PoolBuffer τ Tactical]
        SafetyController[SafetyController]
    end
    
    subgraph "Liquidity Strategies"
        BondingCurve[Bonding Curve Phase 1]
        FloorLiq[Floor Liquidity Phase 2]
        JITLiq[JIT Liquidity Phase 2]
    end
    
    Trader -->|swap| Pool
    LP -->|open/close position| Pool
    Creator -->|mint token| Pool
    
    Pool --> PoolOracle
    Pool --> PoolController
    PoolController --> PoolFloor
    PoolController --> PoolReserve
    PoolController --> PoolBuffer
    
    PoolFloor -.-> FloorLiq
    PoolBuffer -.-> JITLiq
    PoolController -.-> BondingCurve
    
    FeelsSOL <-.-> JitoVault
    ProtocolOracle -.-> JitoVault
    SafetyController -.-> ProtocolOracle
    SafetyController -.-> PoolOracle
    
    PoolReserve -.-> Treasury
    PoolController -.-> Treasury
    
    classDef userClass fill:#e1f5e1,stroke:#2d5016
    classDef protocolClass fill:#e1e5f5,stroke:#162d50
    classDef poolClass fill:#f5e1e1,stroke:#501616
    classDef strategyClass fill:#f5f5e1,stroke:#505016
    
    class Trader,LP,Creator userClass
    class FeelsSOL,JitoVault,Treasury,ProtocolOracle protocolClass
    class Pool,PoolOracle,PoolFloor,PoolController,PoolReserve,PoolBuffer,SafetyController poolClass
    class BondingCurve,FloorLiq,JITLiq strategyClass
```

## Document Dependency Graph

Shows reading order and dependencies between specification documents.

```mermaid
graph TD
    GLOSSARY[GLOSSARY.md<br/>Terms & Abbrev]
    I001[001: Introduction<br/>Protocol Overview]
    I003[003: Hub & Spoke<br/>Routing Model]
    I002[002: Quickstart<br/>User Guide]
    
    C203[203: CLMM<br/>AMM Core]
    C204[204: Oracle<br/>GTWAP]
    C200[200: Solvency<br/>Backing Model]
    
    A201[201: Fees<br/>Dynamic Fees]
    A202[202: JIT<br/>Active MM]
    A205[205: Floor<br/>Passive MM]
    A206[206: Allocation<br/>Fee Split & Capital]
    A207[207: Bonding<br/>Price Discovery]
    A208[208: Pipeline<br/>After-Swap]
    
    S300[300: Launch<br/>Sequence]
    S301[301: Lifecycle<br/>Market States]
    
    G209[209: Params<br/>Governance]
    G210[210: Safety<br/>Controller]
    G211[211: Events<br/>Units]
    G212[212: Registry<br/>Pool Registry]
    
    GLOSSARY --> I001
    I001 --> I003
    I001 --> C203
    I003 --> I002
    
    C203 --> C204
    C203 --> C200
    C204 --> C200
    
    C204 --> A201
    C204 --> A202
    C200 --> A201
    C200 --> A202
    C200 --> A205
    
    A205 --> A206
    A202 --> A206
    C203 --> A207
    A206 --> A207
    
    A201 --> A208
    A202 --> A208
    C204 --> A208
    A205 --> A208
    
    A207 --> S300
    A206 --> S300
    S300 --> S301
    A207 --> S301
    
    A201 --> G209
    A202 --> G209
    A205 --> G209
    C204 --> G210
    C200 --> G210
    G210 --> G209
    
    GLOSSARY -.->|Reference| G211
    
    classDef foundation fill:#e1f5e1
    classDef core fill:#e1e5f5
    classDef advanced fill:#f5e1e1
    classDef operations fill:#f5f5e1
    classDef governance fill:#f5e1f5
    
    class GLOSSARY,I001,I003,I002 foundation
    class C203,C204,C200 core
    class A201,A202,A205,A206,A207,A208 advanced
    class S300,S301 operations
    class G209,G210,G211,G212 governance
```

**Legend**:
- 🟢 Foundation (read first)
- 🔵 Core (essential mechanisms)
- 🔴 Advanced (build on core)
- 🟡 Operations (sequences)
- 🟣 Governance (config/safety)

## Component Integration Map

Shows how unified components integrate across the protocol.

```mermaid
graph TB
    subgraph "Unified Components"
        Oracle[market::Oracle<br/>GTWAP]
        Floor[market::Floor<br/>Calculator]
        Flow[FlowSignals<br/>Market Flow]
        Safety[SafetyController<br/>Risk Mgmt]
        Controller[PoolController<br/>Orchestrator]
    end
    
    subgraph "Feature Systems"
        Fees[Dynamic Fees<br/>201]
        JIT[JIT Liquidity<br/>202]
        FloorPOL[Floor POL<br/>205]
    end
    
    subgraph "Core Operations"
        Swap[Swap Execution<br/>203]
        Pipeline[After-Swap<br/>208]
    end
    
    Swap -->|tick, timestamp| Oracle
    Swap -->|direction, amount| Flow
    
    Oracle -->|GTWAP tick| Fees
    Oracle -->|price anchor| JIT
    Floor -->|safe ask tick| JIT
    Floor -->|floor tick| Fees
    
    Flow -->|flow EWMA| Fees
    Flow -->|toxicity| JIT
    
    Fees -->|fee_bps| Controller
    JIT -->|consumed_q| Controller
    Controller -->|split| FloorPOL
    
    Safety -->|health check| Fees
    Safety -->|can execute| JIT
    Safety -->|observe| Pipeline
    
    Pipeline -->|update| Oracle
    Pipeline -->|update| Flow
    Pipeline -->|ratchet| Floor
    Pipeline -->|record| Safety
    
    classDef unifiedClass fill:#e1e5f5,stroke:#162d50,stroke-width:3px
    classDef featureClass fill:#f5e1e1,stroke:#501616
    classDef coreClass fill:#e1f5e1,stroke:#2d5016
    
    class Oracle,Floor,Flow,Safety,Controller unifiedClass
    class Fees,JIT,FloorPOL featureClass
    class Swap,Pipeline coreClass
```

## Swap Flow Diagram

Detailed sequence showing a swap execution through the system.

```mermaid
sequenceDiagram
    participant User
    participant Swap as swap instruction
    participant Pool as Pool State
    participant Oracle as market::Oracle
    participant Fees as Fee Calculator
    participant Controller as PoolController
    participant JIT as JIT (optional)
    participant Pipeline as After-Swap
    
    User->>Swap: Execute swap(amount_in, min_out)
    Swap->>Pool: Get current state
    Note over Pool: sqrt_price, liquidity, tick
    
    opt JIT Enabled
        Swap->>JIT: Check entry guards
        JIT->>Oracle: Get GTWAP anchor
        JIT->>Pool: Place contrarian liquidity
    end
    
    Swap->>Pool: Execute CLMM swap
    Note over Pool: Cross ticks, update price
    Pool-->>Swap: amount_out, end_tick
    
    opt JIT Enabled
        Swap->>JIT: Remove unfilled liquidity
        JIT->>JIT: Calculate toxicity
    end
    
    Swap->>Fees: Calculate fee (start_tick, end_tick)
    Fees-->>Swap: fee_bps
    
    Note over User,Swap: Check min_out, user fee cap
    
    Swap->>User: Transfer amount_out - fees
    
    Swap->>Pipeline: Post-swap updates
    Pipeline->>Oracle: Update GTWAP
    Pipeline->>Fees: Update flow EWMA
    Pipeline->>Controller: Split collected fees
    Controller->>Controller: LP + Reserve + Buffer + Treasury
    Pipeline->>Pool: Record metrics
    
    Pipeline-->>User: Emit SwapExecuted event
```

## Token Launch Flow

Complete flow from token creation through graduation.

```mermaid
sequenceDiagram
    participant Creator
    participant Escrow as PreLaunchEscrow
    participant Pool as Pool/Market
    participant Bonding as Bonding Curve
    participant Floor as Floor POL
    participant JIT as JIT POL
    
    Note over Creator: Has 3000 FeelsSOL
    
    Creator->>Escrow: mint_token(1000 FeelsSOL fee)
    Note over Escrow: Holds 1B tokens + fee
    
    Creator->>Pool: initialize_market(price)
    Note over Pool: Revoke mint/freeze authority
    Pool->>Escrow: Link to market
    
    Creator->>Bonding: deploy_initial_liquidity
    Escrow->>Bonding: Transfer 1B tokens
    Escrow->>Pool: Transfer mint fee to treasury
    Note over Bonding: Deploy 10-40 staircase ranges
    
    opt Initial Buy
        Creator->>Bonding: swap(1000 FeelsSOL)
        Bonding-->>Creator: ~1M tokens
    end
    
    Note over Pool,Bonding: Phase 1: Price Discovery<br/>Public trading, no LP positions
    
    loop Trading
        Traders->>Bonding: swaps
        Bonding->>Pool: Accumulate fees
    end
    
    Note over Pool: Market cap target met!
    
    Anyone->>Pool: graduate_pool (crank)
    
    Pool->>Floor: Deploy 95% capital
    Note over Floor: Single-sided at floor tick
    
    Pool->>JIT: Seed 5% capital
    Note over JIT: Bootstrap PoolBuffer τ
    
    Bonding->>Pool: Withdraw staircase positions
    
    Note over Pool: Phase 2: Steady State<br/>Floor + JIT + Open to LPs
```

## Fee Distribution Flow

How swap fees are split and allocated.

```mermaid
graph LR
    Swap[Swap Fee<br/>Collected] --> Split{Fee Split<br/>Controller}
    
    Split -->|45%| LP[LP Accumulator<br/>Positions Earn]
    Split -->|25%| Reserve[PoolReserve<br/>Floor Capital]
    Split -->|20%| Buffer[PoolBuffer τ<br/>JIT Budget]
    Split -->|8%| Treasury[Protocol Treasury<br/>Development]
    Split -->|2%| Creator[Creator Base<br/>Accrued]
    
    Reserve -->|Deployment| FloorPOL[Floor Liquidity<br/>Single-sided Ask]
    Buffer -->|Per-swap budget| JITPOL[JIT Liquidity<br/>Contrarian Quotes]
    
    Treasury -.->|Yield allocation| Reserve
    
    classDef feeClass fill:#f5e1e1,stroke:#501616
    classDef recipientClass fill:#e1f5e1,stroke:#2d5016
    classDef strategyClass fill:#e1e5f5,stroke:#162d50
    
    class Swap,Split feeClass
    class LP,Reserve,Buffer,Treasury,Creator recipientClass
    class FloorPOL,JITPOL strategyClass
```

## Safety Controller Decision Tree

How SafetyController responds to different conditions.

```mermaid
graph TD
    Start[Safety Check] --> PoolOracle{market::Oracle<br/>Health?}
    
    PoolOracle -->|Healthy| ProtocolOracle{protocol::Oracle<br/>Health?}
    PoolOracle -->|Stale| DegradeGTWAP[Degrade: GTWAP Stale]
    
    DegradeGTWAP --> ActionGTWAP[Disable rebates<br/>Raise impact floor<br/>Allow swaps]
    
    ProtocolOracle -->|Healthy| Volatility{Volatility<br/>Check?}
    ProtocolOracle -->|Stale| DegradeReserve[Degrade: Reserve Oracle]
    
    DegradeReserve --> ActionReserve[Pause exit_feelssol<br/>Allow swaps]
    
    Volatility -->|Normal| Floor{Floor<br/>Invariant?}
    Volatility -->|High| DegradeVol[Degrade: Volatility]
    
    DegradeVol --> ActionVol[Raise min fees<br/>Cap rebates<br/>Widen spreads]
    
    Floor -->|OK| RateLimit{Rate<br/>Limits?}
    Floor -->|Breach| Critical[Critical: Floor Breach]
    
    Critical --> ActionCritical[Pause Pool<br/>Emergency stop]
    
    RateLimit -->|OK| Proceed[Proceed Normal]
    RateLimit -->|Exceeded| Throttle[Throttle Operations]
    
    Throttle --> ActionThrottle[Reduce JIT budget<br/>Queue operations]
    
    classDef healthyClass fill:#e1f5e1,stroke:#2d5016
    classDef degradeClass fill:#f5f5e1,stroke:#505016
    classDef criticalClass fill:#f5e1e1,stroke:#501616
    
    class Proceed,RateLimit,Floor,Volatility,ProtocolOracle,PoolOracle healthyClass
    class DegradeGTWAP,DegradeReserve,DegradeVol,Throttle degradeClass
    class Critical,ActionCritical criticalClass
```

## Component State Dependencies

Shows which state each component reads and writes.

```mermaid
graph LR
    subgraph "State"
        PoolState[Pool State<br/>sqrt_price, tick, liquidity]
        OracleState[Oracle State<br/>observations ring]
        FloorState[Floor State<br/>current_floor, reserves]
        FlowState[Flow State<br/>flow_ewma, toxicity]
        ControllerState[Controller State<br/>phase, split config]
    end
    
    subgraph "Components Read/Write"
        Swap[swap<br/>instruction]
        Oracle[market::Oracle<br/>component]
        Floor[market::Floor<br/>component]
        Fees[Fee<br/>calculator]
        JIT[JIT<br/>system]
        Controller[Pool<br/>Controller]
    end
    
    Swap -->|R/W| PoolState
    Swap -->|R| OracleState
    Swap -->|W| FlowState
    
    Oracle -->|R| PoolState
    Oracle -->|R/W| OracleState
    
    Floor -->|R| FloorState
    Floor -->|R| PoolState
    
    Fees -->|R| OracleState
    Fees -->|R| FloorState
    Fees -->|R| FlowState
    
    JIT -->|R| OracleState
    JIT -->|R| FloorState
    JIT -->|R/W| FlowState
    JIT -->|R| ControllerState
    
    Controller -->|R/W| ControllerState
    Controller -->|R| FloorState
    Controller -->|W| PoolState
    
    classDef stateClass fill:#e1e5f5,stroke:#162d50
    classDef componentClass fill:#f5e1e1,stroke:#501616
    
    class PoolState,OracleState,FloorState,FlowState,ControllerState stateClass
    class Swap,Oracle,Floor,Fees,JIT,Controller componentClass
```

## Read Order Optimization

Suggested reading order to minimize context loading for common tasks.

### Implementing Swaps

```mermaid
graph LR
    Start([Start]) --> CLMM[203: CLMM §4.4<br/>~50 lines]
    CLMM --> Fees[201: Fees §4<br/>~100 lines]
    Fees --> Pipeline[208: Pipeline<br/>~59 lines]
    Pipeline --> Done([Done<br/>~200 lines total])
    
    classDef readClass fill:#e1f5e1,stroke:#2d5016
    class Start,CLMM,Fees,Pipeline,Done readClass
```

### Launching Tokens

```mermaid
graph LR
    Start([Start]) --> Launch[300: Launch<br/>~390 lines]
    Launch --> Bonding[207: Bonding §2-3<br/>~200 lines]
    Bonding --> Lifecycle[301: Lifecycle §3<br/>~100 lines]
    Lifecycle --> Done([Done<br/>~700 lines total])
    
    classDef readClass fill:#f5f5e1,stroke:#505016
    class Start,Launch,Bonding,Lifecycle,Done readClass
```

### Understanding Solvency

```mermaid
graph LR
    Start([Start]) --> Solvency[200: Solvency §1-2<br/>~150 lines]
    Solvency --> Floor[205: Floor §3<br/>~100 lines]
    Floor --> Safety[210: Safety<br/>~64 lines]
    Safety --> Done([Done<br/>~300 lines total])
    
    classDef readClass fill:#e1e5f5,stroke:#162d50
    class Start,Solvency,Floor,Safety,Done readClass
```

## Module Dependency Map (Code)

Maps code modules to their documentation.

```mermaid
graph TB
    subgraph "programs/feels/src/"
        Instructions[instructions/<br/>swap.rs, open_position.rs...]
        State[state/<br/>pool.rs, position.rs...]
        Logic[logic/<br/>liquidity_math.rs...]
        Constants[constants.rs]
        Errors[error.rs]
        Events[events.rs]
    end
    
    subgraph "Documentation"
        D203[203: CLMM]
        D201[201: Fees]
        D204[204: Oracle]
        D205[205: Floor]
        D202[202: JIT]
        D211[211: Events]
        D209[209: Params]
    end
    
    Instructions -->|swap| D203
    Instructions -->|swap| D201
    Instructions -->|open/close_position| D203
    
    State -->|Pool| D203
    State -->|OracleState| D204
    State -->|PoolFloor| D205
    
    Logic -->|liquidity_math| D203
    Logic -->|fee_calculation| D201
    Logic -->|jit_*| D202
    Logic -->|oracle_*| D204
    
    Constants -->|parameters| D209
    Events -->|event defs| D211
    Errors -->|error codes| D203
    
    classDef codeClass fill:#f5e1e1,stroke:#501616
    classDef docClass fill:#e1e5f5,stroke:#162d50
    
    class Instructions,State,Logic,Constants,Errors,Events codeClass
    class D203,D201,D204,D205,D202,D211,D209 docClass
```

## Data Flow: Swap Execution

Detailed data flow showing how information moves through a swap.

```mermaid
flowchart TD
    Start([User initiates swap]) --> Input[amount_in, min_out,<br/>sqrt_price_limit, max_fee_bps]
    
    Input --> LoadState[Load Pool State<br/>current tick, sqrt_price, liquidity]
    
    LoadState --> JITCheck{JIT<br/>Enabled?}
    
    JITCheck -->|Yes| JITGuards[Check JIT Guards<br/>oracle health, budgets, floor]
    JITCheck -->|No| ExecuteSwap[Execute CLMM Swap Loop]
    
    JITGuards --> JITPlace[Place Contrarian Liquidity<br/>around GTWAP anchor]
    JITPlace --> ExecuteSwap
    
    ExecuteSwap --> TickLoop[Cross Ticks<br/>consume liquidity, update price]
    TickLoop --> SwapResult[amount_out, end_tick,<br/>ticks_crossed]
    
    SwapResult --> JITRemove{JIT<br/>Active?}
    JITRemove -->|Yes| RemoveJIT[Remove Unfilled JIT<br/>calculate toxicity]
    JITRemove -->|No| CalcFee[Calculate Fee<br/>start_tick → end_tick]
    RemoveJIT --> CalcFee
    
    CalcFee --> FeeCheck{fee_bps ≤<br/>max_fee_bps?}
    FeeCheck -->|No| Revert([Revert: Fee Exceeds Cap])
    FeeCheck -->|Yes| ApplyFee[amount_out - fees]
    
    ApplyFee --> MinCheck{amount_out ≥<br/>min_out?}
    MinCheck -->|No| Revert2([Revert: Slippage])
    MinCheck -->|Yes| Transfer[Transfer Tokens]
    
    Transfer --> UpdateOracle[Update market::Oracle<br/>new tick, timestamp]
    UpdateOracle --> UpdateFlow[Update FlowSignals<br/>flow_ewma, toxicity]
    UpdateFlow --> SplitFees[Split Fees<br/>LP, Reserve, Buffer, Treasury]
    SplitFees --> RatchetCheck{Can<br/>Ratchet?}
    
    RatchetCheck -->|Yes| RatchetFloor[Ratchet Floor Up]
    RatchetCheck -->|No| SafetyObserve[SafetyController Observe]
    RatchetFloor --> SafetyObserve
    
    SafetyObserve --> EmitEvent[Emit SwapExecuted Event]
    EmitEvent --> Done([Success])
    
    classDef inputClass fill:#e1f5e1,stroke:#2d5016
    classDef processClass fill:#e1e5f5,stroke:#162d50
    classDef decisionClass fill:#f5f5e1,stroke:#505016
    classDef errorClass fill:#f5e1e1,stroke:#501616
    
    class Start,Input,Done inputClass
    class LoadState,ExecuteSwap,TickLoop,CalcFee,Transfer,UpdateOracle,UpdateFlow,SplitFees,RatchetFloor,SafetyObserve,EmitEvent processClass
    class JITCheck,JITRemove,FeeCheck,MinCheck,RatchetCheck decisionClass
    class Revert,Revert2 errorClass
```

## Component Interaction Summary

Quick reference table for component interactions.

| Component | Reads From | Writes To | Used By | Documents |
|-----------|-----------|-----------|---------|-----------|
| **market::Oracle** | Pool.current_tick | OracleState.observations | Fees, JIT, Floor | 204 |
| **market::Floor** | PoolFloor state, Pool state | PoolFloor.current_floor | Fees, JIT, Controller | 205, 200 §7 |
| **FlowSignals** | Swap results, JIT results | flow_ewma, toxicity | Fees, JIT | 201 §9 |
| **SafetyController** | Oracle health, metrics | Health status, pauses | All operations | 210, 200 §6.3 |
| **PoolController** | Fee amounts, phase | Fee splits, allocations | After-swap, Launch | 206 |
| **Dynamic Fees** | Oracle, Floor, Flow | fee_bps | Swap instruction | 201 |
| **JIT System** | Oracle, Floor, Buffer | PoolBuffer, toxicity | Swap instruction | 202 |

## See Also

- **[DOCS-INDEX.md](DOCS-INDEX.md)**: Task-based navigation guide
- **[GLOSSARY.md](GLOSSARY.md)**: Terms and abbreviations
- **[CONCEPT-CARDS.md](CONCEPT-CARDS.md)**: Quick component summaries
- **[README.md](../../../README.md)**: Build and development setup

