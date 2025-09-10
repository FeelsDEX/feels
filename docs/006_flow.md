# Flow-Centric Fee Model (Price-Only MVP)

This document specifies a simple, fully on‑chain, price‑only, flow‑centric fee model for the Feels protocol. The presentation mirrors the style of the system overview: we motivate the physics analogy, write down a compact mathematical model, and describe an implementation that is robust, bounded, and efficient on chain.

## Introduction

Our goal is to incentivize price stability and healthy liquidity by charging more when trades extend imbalances and charging less when trades relieve them. We do this with a physics‑inspired model: price displacement from equilibrium behaves like potential energy and persistent order flow behaves like momentum. Uphill moves (away from equilibrium) pay a surcharge; downhill moves (toward equilibrium) receive a discount, bounded by caps. This creates a self‑correcting market that remains simple enough to implement and audit fully on chain.

This MVP focuses on the spot/price dimension only. Time and leverage dimensions remain inactive in the fee model here. The design uses only small per‑swap computations (integer arithmetic), a TWAP oracle read, and a single signed flow EWMA per market.

## Analogy and Intuition

- Equilibrium: The time‑weighted average price (TWAP) is a local equilibrium point.
- Potential: Displacement from TWAP stores “potential energy.” Larger deviations imply greater stored energy.
- Momentum: Recent signed order flow is a proxy for price momentum. Sustained flow in one direction tends to extend displacement.
- Work and Resistance: The protocol charges fees proportional to a mixture of potential (displacement) and momentum (flow), capped and bounded. Trades that reduce potential are rewarded with a discount factor; trades that increase potential pay a surcharge.

## Mathematical Model

Let $t$ be the current tick and $t_{\text{twap}}$ a TWAP over a fixed on‑chain window. Define the displacement magnitude

\[
\delta \;=\; |\, t - t_{\text{twap}} \,|, \qquad \delta_{\text{eff}} \;=\; \max\{ 0,\, \delta - \varepsilon \},
\]

where $\varepsilon \geq 0$ is a deadband in ticks. Map displacement to a bounded surcharge via a saturating concave response

\[
\textstyle g(\delta_{\text{eff}}) \;=\; \frac{\delta_{\text{eff}}}{\delta_{\text{eff}} + S}, \qquad S>0, \quad 0\le g<1.
\]

Let $q$ be a signed flow EWMA (defined below). Map its magnitude to a bounded response

\[
\textstyle h(|q|) \;=\; \frac{|q|}{|q| + Q}, \qquad Q>0, \quad 0\le h<1.
\]

Define a direction gate $d \in \{+1, -\gamma\}$ with $\gamma \in [0,1]$:
- $d = +1$ if the swap pushes price away from TWAP (extends displacement).
- $d = -\gamma$ if the swap pushes price toward TWAP (relieves displacement).

Given policy parameters
\[\; f_{\text{base}}\in\mathbb{N},\; k_{\text{disp}},k_{\text{flow}}\in\mathbb{N},\; f_{\text{min}}\le f_{\text{max}},\; \varepsilon,S,Q\ge 0,\; \gamma\in[0,1], \]

we define the instantaneous fee (in basis points) as

\[
\boxed{\quad f \;=\; \operatorname{clamp}\Big( f_{\text{min}},\; f_{\text{base}} + k_{\text{disp}}\,g(\delta_{\text{eff}}) + k_{\text{flow}}\,h(|q|)\,d,\; f_{\text{max}} \Big).\quad}
\]

Remarks:
- $g$ increases with displacement and approaches 1 as $\delta_{\text{eff}} \to \infty$.
- $h$ increases with the magnitude of recent flow and approaches 1 as $|q| \to \infty$.
- The factor $d$ applies a discount ($-\gamma$) when trades reduce displacement and a surcharge ($+1$) when trades extend it.
- $\text{clamp}$ bounds fees between $f_{\text{min}}$ and $f_{\text{max}}$.

### Signed Flow EWMA

We update a signed flow EWMA per swap (or per segment) as

\[
q^{+} \;=\; q + \alpha\,(q_{\text{obs}} - q), \qquad 0<\alpha\le 1,
\]

where $q_{\text{obs}}$ is a signed observation proportional to trade direction and size. A simple choice is

\[
q_{\text{obs}} \;=\; s\,\min(\,x,\,X_{\text{cap}}\,), \qquad s\in\{+1,-1\},
\]

with $s = +1$ when the swap increases price and $s = -1$ when it decreases price; $x$ is the net input (per segment or entire swap), and $X_{\text{cap}}$ caps the contribution. Choose $\alpha$ as a fixed Q‑format constant or compute $\alpha = 1 - e^{-\Delta t/\tau}$ with an integer approximation using the on‑chain clock.

### Direction Gate

Let $\text{sgn}(t - t_{\text{twap}})$ be the sign of displacement and let $\text{sgn}(\Delta t)$ be the sign of the price move induced by the swap (up for one‑for‑zero, down for zero‑for‑one). The gate is

\[
 d = \begin{cases}
   +1, & \text{if } \operatorname{sgn}(t - t_{\text{twap}})\cdot \operatorname{sgn}(\Delta t) = +1 \quad (\text{away}),\\[3pt]
   -\gamma, & \text{otherwise} \quad (\text{toward}).
 \end{cases}
\]

This makes extending displacement more expensive and relieving displacement cheaper (bounded by $\gamma$).

## On‑Chain Implementation

- Read $t$ from market state and $t_{\text{twap}}$ from the on‑chain oracle over a fixed window; if insufficient data, set $t_{\text{twap}} = t$ to disable surcharges gracefully.
- Compute $\delta_{\text{eff}}$ and $g(\cdot)$ using integer ratios. Compute $h(\cdot)$ from $|q|$ similarly.
- Determine $d$ from displacement direction and swap direction.
- Compute $f$ and clamp to $[f_{\text{min}}, f_{\text{max}}]$.
- Use $f$ (bps) in the swap stepper for fee gross‑up and fee growth updates (input side only).
- After the swap (or per segment), update $q$ via the EWMA; clamp $q$ to $|q| \leq Q_{\text{cap}}$.

All math is integer and bounded; no logarithms or exponentials are required in the hot path.

## Robustness and Bounds

- Deadband $\varepsilon$ prevents micro‑noise; $S$ and $Q$ set the half‑saturation scales for displacement and flow.
- Caps: $f_{\text{min}} \leq f \leq f_{\text{max}}$; $|q| \leq Q_{\text{cap}}$; $g,h \in [0,1)$.
- Oracle safety: require a minimum elapsed time between observations; when not met, fall back to base fees only.
- Determinism: all state transitions use Q0/Q64 integer arithmetic; no data‑dependent loops beyond the existing stepper.

## Parameters (MVP)

- $f_{\text{base}}$: base fee in bps (e.g., 5)
- $k_{\text{disp}}$: max displacement surcharge in bps (e.g., 20)
- $\varepsilon$: deadband in ticks (e.g., 25)
- $S$: displacement half‑saturation (ticks, e.g., 150)
- $k_{\text{flow}}$: max flow surcharge in bps (e.g., 10)
- $Q$: flow half‑saturation (units of EWMA, scaled to inputs)
- $\gamma$: reversion discount factor (e.g., $1/2$)
- $f_{\text{min}}, f_{\text{max}}$: global bounds (e.g., 2 and 100)
- $\alpha$: EWMA gain; $X_{\text{cap}}$: absolute cap for $x$; $Q_{\text{cap}}$: absolute cap for $|q|$
- $t_{\text{twap}}$ window (seconds): e.g., 300

These defaults produce stable, intuitive behavior while keeping fees predictable.

## Interpretation and Energy View

The surcharge can be viewed as a discrete approximation to "work" against a resistant field. Let $U(\delta_{\text{eff}}) \propto g(\delta_{\text{eff}})$ be a potential and let a flow‑penalty term scale with $h(|q|)$. Then the marginal resistance per step is

\[
\mathrm{d}W \;\propto\; \underbrace{\nabla U(\delta_{\text{eff}}) \cdot \mathrm{d}x}_{\text{displacement}} \; + \; \underbrace{\lambda\, h(|q|)\, d}_{\text{flow gate}},
\]

which in our discrete implementation reduces to the bounded, additively composable fee defined above.

## Testing and Validation

- Unit: verify monotonicity in $\delta_{\text{eff}}$, correct gating $d$, saturation at bounds, and stability under deadband.
- Engine: compare fees for equal‑magnitude steps "away" vs "toward"; per‑segment recompute maintains invariants.
- Integration: (a) no‑oracle ⇒ base only; (b) sustained one‑sided flow raises $h(|q|)$; reversing swaps get discounted; (c) fee growth increases only on the input side; (d) large swaps clamp at configured bounds.

## Roadmap

This MVP models only the price dimension. Natural extensions include:
- Inventory/skew term (small surcharge based on vault/LP inventory imbalance, capped)
- Time/leverage domains (extend surcharge fields to $T$ and $L$ with consistent conservation)
- Hysteresis for fee fields to avoid oscillation at thresholds

The above can be introduced incrementally without changing the core on‑chain abstractions.

