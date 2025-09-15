//! Protocol Safety Controller (MVP)
//!
//! Tracks de-peg observations and pauses redemptions when criteria are met.

use anchor_lang::prelude::*;

#[account]
pub struct SafetyController {
    /// Whether redemptions are paused due to de-peg
    pub redemptions_paused: bool,
    /// Consecutive divergence observations over threshold
    pub consecutive_breaches: u8,
    /// Consecutive safe observations since last breach
    pub consecutive_clears: u8,
    /// Last state change slot
    pub last_change_slot: u64,
    /// Per-slot mint tracking (FeelsSOL units)
    pub mint_last_slot: u64,
    pub mint_slot_amount: u64,
    /// Per-slot redeem tracking (FeelsSOL units)
    pub redeem_last_slot: u64,
    pub redeem_slot_amount: u64,
}

impl SafetyController {
    pub const SEED: &'static [u8] = b"safety_controller";
    pub const LEN: usize = 8 + // disc
        1 + // redemptions_paused
        1 + // consecutive_breaches
        1 + // consecutive_clears
        8 +  // last_change_slot
        8 +  // mint_last_slot
        8 +  // mint_slot_amount
        8 +  // redeem_last_slot
        8; // redeem_slot_amount
}
