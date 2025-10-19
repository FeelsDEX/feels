---
title: "Events and Units"
description: "Protocol events and unit conventions"
category: "Specifications"
order: 211
draft: false
searchable: true
---

# Events, Units, and Rounding Policy

Defines canonical events, unit suffixes, and rounding rules for MVP.

## Units and Suffixes

- `_bps`: basis points (1/10,000)
- `_ticks`: price ticks (log scale; 1.0001^tick)
- `_q16`, `_q32`, `_x64`: fixed-point formats

## Rounding Policy

- Credits to protocol solvency (PoolReserve/Protocol Treasury): round in favor of solvency on ties.
- Credits to PoolBuffer and Creator: floor where necessary to avoid leakage; document exceptions.
- Fees applied to output: integer math; document that fee is applied to output amount after swap.

## Core Events (Fields indicative)

- FeeSplitApplied { pool, fee_bps, lps_amount, reserve_amount, buffer_amount, treasury_amount, creator_amount }
  - Include: `jit_consumed_quote` (amount of PoolBuffer used by JIT v0 in this swap; 0 if none)
- RebateApplied { pool, rebate_bps, amount }
- OracleUpdatedPool { pool, tick, twap_tick, timestamp }
- OracleUpdatedProtocol { rate, source, timestamp }
- FloorRatcheted { pool, old_tick, new_tick }
- PoolPhaseChanged { pool, old_phase, new_phase }
- PoolGraduated { pool, cap_met, timestamp }
- CreatorFeeAccrued { pool, amount }
- FeelsSOLMinted { user, amount }
- FeelsSOLRedeemed { user, amount }
- SafetyDegraded { scope, component, level }
- SafetyPaused { scope }
- SafetyResumed { scope }
- CircuitBreakerActivated { component, threshold_bps, window_secs }
- RedemptionsPaused { reason }
- RedemptionsResumed {}

## Planned Events (Vaults & Lending; future)

- VaultDeposit { user, amount, duration }
- VaultWithdrawScheduled { user, amount, unlock_time }
- VaultWithdrawExecuted { user, amount }
- LendingAllocationChanged { pool, old_q, new_q }
- VaultCapacityObserved { pool, capacity_q }
