# Continuous Rebasing

This document describes the continuous mechanism for settling time‑value and funding flows using exact exponentials and conservation within each subdomain, including the role of the buffer $\tau$. The rebasing system maintains strict value conservation while allowing organic growth and contraction of market positions through thermodynamically consistent rebalancing.

## Symbols

- $r$: Annualized rate; $\Delta t$: elapsed time (seconds); $\text{year}$: seconds per year; $dt$: time in years.
- $g$: Multiplicative growth factor $e^{r\,dt}$.
- $w_i$: Snapshot participation weights; $g_i$: multiplicative growth factors.
- $g_A, g_D, g_\tau$: Deposit, debt, and buffer growth factors.
- $w_A, w_D, w_\tau^{(\text{time})}$: Weights for deposits, debt, and buffer (sum to 1 in time subdomain).
- $\tau$: Protocol buffer; $\zeta_{spot}, \zeta_{time}, \zeta_{leverage}$: Fee distribution weights (bps, sum to 10,000).
- $\text{skew}$: Directional imbalance in $[-1,1]$ (used for funding dynamics).
 - $\rho_T(d)$: Time‑domain risk $= \sigma_{\text{rate}}\,\sqrt{d}$ (dimensionless).
 - $\rho_L$: Leverage‑domain risk $= \sigma_{\text{leverage}}\,|\text{skew}|$ (dimensionless).
 - $d$: Duration bucket (years); $T_{\text{lend}}(d), T_{\text{borrow}}(d)$: lending/borrowing notionals.
 - $\sigma_{\text{rate}}$: Annualized interest‑rate volatility; $\sigma_{\text{leverage}}$: annualized position volatility.

## Thermodynamic Growth Framework

### Exact Exponential Compounding

Continuous effects use multiplicative growth over time with exact exponentials:

$$g = \exp\left(\frac{r \cdot \Delta t}{\text{year}}\right)$$

where $r$ is an annualized rate, $\Delta t$ is elapsed time in seconds and $\text{year}$ is a unit‑conversion constant (seconds per year); equivalently, $g = e^{r\,dt}$ with $dt$ measured in years. This approach ensures mathematical precision by avoiding accumulation of linear approximation errors, maintains thermodynamic consistency as exponential relaxation matches physical systems, and provides stability by keeping growth factors bounded and well-behaved.

Growth factors are applied per domain with weights measured at epoch start, preventing mid-period gaming.

## Conservation Laws and Buffer Mechanics

### Fundamental Conservation Identity

Each rebase respects a weighted log‑sum identity within the participating subdomain:

$$\sum_i w_i \ln g_i = 0$$

Here $w_i$ are snapshot participation weights and $g_i$ are multiplicative growth factors for participants in the subdomain. This constraint ensures value preservation by keeping total system value constant, maintains energy conservation so no value is created or destroyed during rebasing, and enables auditable accounting where all value flows remain traceable and verifiable.

Weights are snapshotted in the relevant numeraire at epoch start to prevent manipulation through mid-period position changes.

### Buffer Role as Thermodynamic Reservoir

The buffer $\tau$ serves as a thermodynamic reservoir that absorbs residuals to satisfy conservation exactly, provides liquidity by sourcing rebates and absorbing fees, and stabilizes dynamics by dampening oscillations through strategic participation.

Buffer participation is determined by policy and can vary by subdomain based on market conditions and risk management requirements.

## Lending

Deposits and debt grow at their respective rates. The buffer's change is set so that the identity holds using subdomain weights over deposits, debt, and buffer participation.

The expression for the buffer factor is:

$$\ln g_\tau = -\frac{w_A \ln g_A + w_D \ln g_D}{w_\tau^{(\text{time})}}$$

where $w_A$, $w_D$, and $w_\tau^{(\text{time})}$ sum to one.

### Growth Factor Calculations

For the lending subdomain:
- **Deposits**: $g_A = \exp(r_{\text{supply}} \cdot \Delta t / \text{year})$
- **Debt**: $g_D = \exp(r_{\text{borrow}} \cdot \Delta t / \text{year})$  
- **Buffer**: Computed to satisfy conservation

## Leverage Funding and Price‑Driven Growth

### Funding Mechanism
Funding is derived from directional skew and applied symmetrically to:
- **Long positions**: Receive or pay funding based on skew
- **Short positions**: Receive or pay funding based on skew
- **Buffer participation**: Optional, as determined by policy

### Constraints
- **Daily funding cap**: Hard limit on maximum daily funding rate
- **Price‑driven growth**: Applied from TWAP changes under conservation
- **P&L settlement**: Leveraged P&L settles without creating or destroying value

## Epoch Policy

### Timing and Execution
- **Epoch definition**: A configured rebase interval (e.g., hourly or daily)
- **Snapshot timing**: At the epoch boundary
- **Daily funding**: Subject to hard caps
- **Rounding residuals**: Handled to maintain subdomain identity over each epoch

### Conservation Enforcement
Each epoch maintains the constraint:
$$\sum_i w_i \ln g_i = 0$$

## Implementation Example

This example computes deposit and debt growth with exact exponentials and derives the buffer factor required to satisfy the conservation identity in the lending subdomain.

```rust
// Lending subdomain rebase calculation
fn calculate_lending_rebase(
    r_supply: f64,      // Supply rate (annualized)
    r_borrow: f64,      // Borrow rate (annualized)
    dt: f64,            // Time delta in years
    w_A: f64,           // Deposit weight
    w_D: f64,           // Debt weight  
    w_tau_time: f64,    // Buffer time weight
) -> (f64, f64, f64) {
    // Growth factors for deposits and debt
    let g_A = (r_supply * dt).exp();
    let g_D = (r_borrow * dt).exp();
    
    // Buffer factor to satisfy conservation
    let ln_g_tau = -(w_A * g_A.ln() + w_D * g_D.ln()) / w_tau_time;
    let g_tau = ln_g_tau.exp();
    
    (g_A, g_D, g_tau)
}
```

## Buffer Fee Distribution and Normalization

### Fee Collection and Distribution Process

All fees collected and rebates paid flow through the buffer $\tau$ with systematic distribution across domains:

$$\tau_{\text{new}} = \tau_{\text{old}} + \text{fees}_{\text{collected}} - \text{rebates}_{\text{paid}}$$

#### Fee Distribution by Domain

Every trade triggers fee distribution according to configured participation coefficients:

```rust
// Fee distribution (every trade)
spot_share = (fee × ζ_spot) / 10000      // Default: 33.33%
time_share = (fee × ζ_time) / 10000      // Default: 33.33%  
leverage_share = (fee × ζ_leverage) / 10000 // Default: 33.34%

// Update domain participation
buffer.participation_spot += spot_share
buffer.participation_time += time_share  
buffer.participation_leverage += leverage_share
```

where $\zeta_{spot}$, $\zeta_{time}$, and $\zeta_{leverage}$ are basis‑point weights that sum to 10,000 (or 1.0 if expressed as fractions), set by policy.

### Buffer Normalization Process

Buffer normalization occurs during continuous rebasing to maintain proper accounting across domains:

1. **Snapshot weights** at epoch boundary for each participating subdomain
2. **Apply growth factors** with exact exponentials per domain
3. **Compute buffer adjustment** to satisfy conservation identity exactly
4. **Redistribute residuals** through participation coefficients

This process ensures the buffer acts as a proper thermodynamic reservoir while maintaining transparent fee accounting.

## Mathematical Properties

### Conservation Identity
The fundamental conservation principle ensures value preservation by keeping total system value constant, maintains energy conservation so no value is created or destroyed during rebasing, and enables auditable accounting where all value flows remain traceable and verifiable.

### Growth Compounding
Using exact exponentials rather than linear approximations ensures accuracy through precise compounding over time, maintains consistency by preserving mathematical properties, and provides stability without drift or accumulation errors.
