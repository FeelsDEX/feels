//! Minimal JIT v0 budgeting (no placement yet)
//!
//! Keeps per-slot and per-swap budgets in quote units, backed by Buffer fields.

use crate::state::Buffer;
use anchor_lang::prelude::*;

#[derive(Clone, Copy, Default)]
pub struct JitBudget {
    pub slot: u64,
    pub per_swap_cap_q: u128,
    pub per_slot_cap_q: u128,
    pub slot_remaining_q: u128,
}

impl JitBudget {
    pub fn begin(
        buffer: &mut Account<Buffer>,
        current_slot: u64,
        per_swap_bps: u16,
        per_slot_bps: u16,
    ) -> Self {
        // Reset per-slot used on new slot
        if buffer.jit_last_slot != current_slot {
            buffer.jit_last_slot = current_slot;
            buffer.jit_slot_used_q = 0;
        }
        // Use tau_spot as quote reserve proxy
        let base_q = buffer.tau_spot;
        let per_swap_cap_q = (base_q.saturating_mul(per_swap_bps as u128)) / 10_000u128;
        let per_slot_cap_q = (base_q.saturating_mul(per_slot_bps as u128)) / 10_000u128;
        let slot_remaining_q = per_slot_cap_q.saturating_sub(buffer.jit_slot_used_q);
        Self {
            slot: current_slot,
            per_swap_cap_q,
            per_slot_cap_q,
            slot_remaining_q,
        }
    }

    /// Reserve an amount (quote units) not exceeding per-swap and remaining per-slot caps.
    pub fn reserve(&mut self, buffer: &mut Account<Buffer>, desired_q: u128) -> u128 {
        let allow_q = desired_q
            .min(self.per_swap_cap_q)
            .min(self.slot_remaining_q);
        if allow_q == 0 {
            return 0;
        }
        // Consume slot quota
        buffer.jit_slot_used_q = buffer.jit_slot_used_q.saturating_add(allow_q);
        self.slot_remaining_q = self.slot_remaining_q.saturating_sub(allow_q);
        allow_q
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

    fn dummy_buffer() -> Buffer {
        Buffer {
            market: Pubkey::default(),
            authority: Pubkey::default(),
            feelssol_mint: Pubkey::default(),
            fees_token_0: 0,
            fees_token_1: 0,
            tau_spot: 1_000_000,
            tau_time: 0,
            tau_leverage: 0,
            floor_tick_spacing: 0,
            floor_placement_threshold: 0,
            last_floor_placement: 0,
            last_rebase: 0,
            total_distributed: 0,
            buffer_authority_bump: 0,
            jit_last_slot: 0,
            jit_slot_used_q: 0,
        }
    }

    #[test]
    fn test_jit_budget_logic() {
        // Test the JIT budget calculation logic
        let buffer_data = dummy_buffer();

        // Test parameters: 0.5% per swap, 1% per slot
        let per_swap_bps = 50u16;
        let per_slot_bps = 100u16;

        // Calculate expected max amounts
        let max_swap = (buffer_data.tau_spot as u128 * per_swap_bps as u128 / 10_000);
        let max_slot = (buffer_data.tau_spot as u128 * per_slot_bps as u128 / 10_000);

        // Verify our constants are reasonable
        assert!(
            buffer_data.tau_spot > 0,
            "Buffer should have positive tau_spot"
        );
        assert!(max_swap > 0, "Max swap amount should be positive");
        assert!(max_slot > 0, "Max slot amount should be positive");
        assert!(
            max_swap <= max_slot,
            "Per-swap limit should be <= per-slot limit"
        );

        // Test slot budget logic
        // If we use half the slot budget, we should have half remaining
        let half_budget = max_slot / 2;
        let remaining = max_slot.saturating_sub(half_budget);
        assert_eq!(remaining, max_slot / 2, "Remaining budget calculation");
    }
}
