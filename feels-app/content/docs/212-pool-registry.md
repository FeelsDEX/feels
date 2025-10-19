---
title: "Pool Registry"
description: "Pool registration and management system"
category: "Specifications"
order: 212
draft: false
searchable: true
---

# Pool Registry (MVP)

Ensures one canonical pool per token (paired with FeelsSOL), simplifies discovery, and exposes pool metadata.

## Responsibilities

- Enforce uniqueness: one pool per token mint (token ↔ FeelsSOL).
- Track pool metadata: pool pubkey, token mint, tick_spacing, base_fee_bps, phase, status (paused/degraded flags), creation timestamp.
- Provide iteration for UI/indexers.

## Integration

- `initialize_pool` registers the new pool; reject if an entry exists.
- Registry entry updates on phase changes and pause/resume events.

## Seeds and Fields

PDA seed: `b"pool_registry"`

Entry fields (suggested):
- `pool: Pubkey`
- `token_mint: Pubkey`
- `tick_spacing: u16`
- `base_fee_bps: u16`
- `phase: u8` (0=PriceDiscovery, 1=SteadyState)
- `paused: bool`
- `created_at: i64`
- `last_update_slot: u64`
