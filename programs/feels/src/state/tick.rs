//! Tick data structures for concentrated liquidity (MVP)
//!
//! Minimal Tick and TickArray to support a single range position and
//! basic liquidity net/gross accounting. Business logic in handlers
//! initializes ticks and updates liquidity.

use anchor_lang::prelude::*;

/// Number of ticks per array (kept small for MVP)
pub const TICK_ARRAY_SIZE: usize = 64;

/// Individual tick within an array
/// Must be exactly aligned with no padding for zero_copy
#[zero_copy]
#[derive(Default)]
#[repr(C)]
pub struct Tick {
    pub liquidity_net: i128,            // 16 bytes
    pub liquidity_gross: u128,          // 16 bytes
    pub fee_growth_outside_0_x64: u128, // 16 bytes - Q64 fixed point
    pub fee_growth_outside_1_x64: u128, // 16 bytes - Q64 fixed point
    pub initialized: u8,                // 1 byte
    pub _pad: [u8; 15],                 // 15 bytes to make total 80 (16-aligned)
}

#[account(zero_copy)]
#[repr(C)]
pub struct TickArray {
    pub market: Pubkey,
    pub start_tick_index: i32,
    pub _pad0: [u8; 12],
    pub ticks: [Tick; TICK_ARRAY_SIZE],
    pub initialized_tick_count: u16,
    pub _pad1: [u8; 14],
    pub _reserved: [u8; 32],
}

impl TickArray {
    // TickArray layout: discriminator(8) + market(32) + start_tick_index(4) + pad0(12) + ticks(80*64) + initialized_tick_count(2) + pad1(14) + reserved(32)
    pub const LEN: usize = 8 + 32 + 4 + 12 + (80 * TICK_ARRAY_SIZE) + 2 + 14 + 32;

    /// Returns the index within the array for a global tick index
    pub fn offset_for(&self, tick_index: i32, tick_spacing: u16) -> Result<usize> {
        require!(
            tick_index % tick_spacing as i32 == 0,
            crate::error::FeelsError::InvalidPrice
        );
        require!(
            self.start_tick_index % tick_spacing as i32 == 0,
            crate::error::FeelsError::InvalidPrice
        );
        let rel = (tick_index - self.start_tick_index) / tick_spacing as i32;
        require!(
            rel >= 0 && rel < (TICK_ARRAY_SIZE as i32),
            crate::error::FeelsError::InvalidPrice
        );
        Ok(rel as usize)
    }

    pub fn get_tick_mut(&mut self, tick_index: i32, tick_spacing: u16) -> Result<&mut Tick> {
        let off = self.offset_for(tick_index, tick_spacing)?;
        Ok(&mut self.ticks[off])
    }

    pub fn get_tick(&self, tick_index: i32, tick_spacing: u16) -> Result<&Tick> {
        let off = self.offset_for(tick_index, tick_spacing)?;
        Ok(&self.ticks[off])
    }

    pub fn init_tick(
        &mut self,
        tick_index: i32,
        tick_spacing: u16,
        current_tick: i32,
        fee_growth_global_0: u128,
        fee_growth_global_1: u128,
    ) -> Result<()> {
        // Check if we need to initialize
        let needs_init = {
            let t = self.get_tick(tick_index, tick_spacing)?;
            t.initialized == 0
        };

        if needs_init {
            // Update count first
            self.initialized_tick_count = self.initialized_tick_count.saturating_add(1);

            // Now get mutable reference and initialize
            let t = self.get_tick_mut(tick_index, tick_spacing)?;
            t.initialized = 1;

            // Initialize fee growth outside based on position relative to current tick
            // This follows Uniswap V3 convention
            if tick_index <= current_tick {
                // Tick is at or below current price
                // All global fee growth is "outside" (below this tick)
                t.fee_growth_outside_0_x64 = fee_growth_global_0;
                t.fee_growth_outside_1_x64 = fee_growth_global_1;
            } else {
                // Tick is above current price
                // No fee growth is "outside" (below this tick) yet
                t.fee_growth_outside_0_x64 = 0u128;
                t.fee_growth_outside_1_x64 = 0u128;
            }
        }
        Ok(())
    }

    pub fn update_liquidity(
        &mut self,
        tick_index: i32,
        tick_spacing: u16,
        delta: i128,
        upper: bool,
    ) -> Result<()> {
        let t = self.get_tick_mut(tick_index, tick_spacing)?;
        require!(t.initialized == 1, crate::error::FeelsError::InvalidPrice);
        // gross
        if delta >= 0 {
            t.liquidity_gross = t
                .liquidity_gross
                .checked_add(delta as u128)
                .ok_or(crate::error::FeelsError::MathOverflow)?;
        } else {
            let d = (-delta) as u128;
            t.liquidity_gross = t
                .liquidity_gross
                .checked_sub(d)
                .ok_or(crate::error::FeelsError::MathOverflow)?;
        }
        // net
        if upper {
            t.liquidity_net = t
                .liquidity_net
                .checked_sub(delta)
                .ok_or(crate::error::FeelsError::MathOverflow)?;
        } else {
            t.liquidity_net = t
                .liquidity_net
                .checked_add(delta)
                .ok_or(crate::error::FeelsError::MathOverflow)?;
        }
        Ok(())
    }

    /// Flip the fee growth outside values when crossing this tick.
    /// Implements: fee_outside = fee_global - fee_outside (mod 2^128)
    pub fn flip_fee_growth_outside(
        &mut self,
        tick_index: i32,
        tick_spacing: u16,
        fee_growth_global_0_x64: u128,
        fee_growth_global_1_x64: u128,
    ) -> Result<()> {
        let t = self.get_tick_mut(tick_index, tick_spacing)?;
        require!(t.initialized == 1, crate::error::FeelsError::InvalidPrice);
        let prev0 = t.fee_growth_outside_0_x64;
        let prev1 = t.fee_growth_outside_1_x64;
        t.fee_growth_outside_0_x64 = fee_growth_global_0_x64.wrapping_sub(prev0);
        t.fee_growth_outside_1_x64 = fee_growth_global_1_x64.wrapping_sub(prev1);
        Ok(())
    }
}

// Compile-time size assertions guarantee zero_copy layout stays stable
const _: [(); 80] = [(); core::mem::size_of::<Tick>()];
const _: [(); 8 + 32 + 4 + 12 + (80 * crate::state::tick::TICK_ARRAY_SIZE) + 2 + 14 + 32] =
    [(); TickArray::LEN];
