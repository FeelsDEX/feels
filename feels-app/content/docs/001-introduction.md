---
title: "Introduction"
description: "Welcome to Feels Protocol documentation - learn about our physics-based AMM on Solana"
category: "Getting Started"
order: 1
---

# Introduction

Feels Protocol is a concentrated liquidity AMM that converts speculative trading into long-term value through programmatic market making and floor price mechanisms.

## Architecture

The protocol implements concentrated liquidity with tick-based positioning to place capital precisely where needed. Each token has exactly one market paired with FeelsSOL, creating unified liquidity and eliminating routing complexity. Protocol-owned accounts deploy autonomous market making strategies including floor liquidity that creates hard price floors and JIT liquidity that captures value from directional trades.

Key features include:

- Concentrated liquidity AMM with tick-based price ranges
- Hub-and-spoke topology with FeelsSOL as universal routing token
- Protocol-owned market making with floor and JIT strategies
- Geometric time-weighted average pricing for manipulation resistance
- Dynamic fee structure based on price impact and market conditions

Trading fees accumulate in protocol-owned accounts that provide just-in-time liquidity and maintain hard price floors. The system uses bounded routing with maximum 2 hops, segmented trade execution, and zero-copy account management for efficient state updates.

## Quick Links

- [Quickstart Guide](/docs/quickstart) - Get started with Feels Protocol
- [Hub and Spoke Model](/docs/hub-and-spoke-architecture) - Understand our routing system
- [SDK Reference](/docs/sdk-reference) - Integrate with Feels Protocol
