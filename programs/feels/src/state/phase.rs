//! Market phase tracking
//!
//! Defines the lifecycle phases of a market from launch to steady state

use anchor_lang::prelude::*;

/// Market lifecycle phase
#[derive(Default, Clone, Copy, Debug, PartialEq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum MarketPhase {
    /// Initial state - market created but not launched
    #[default]
    Created = 0,

    /// Bonding curve phase - initial liquidity distribution
    BondingCurve = 1,

    /// Transitioning phase - moving from bonding to AMM
    Transitioning = 2,

    /// Steady state AMM - normal operation
    SteadyState = 3,

    /// Graduated - bonding curve cleaned up
    Graduated = 4,

    /// Paused - temporary halt (safety)
    Paused = 5,

    /// Deprecated - market no longer active
    Deprecated = 6,
}

impl MarketPhase {
    /// Check if phase allows trading
    pub fn allows_trading(&self) -> bool {
        matches!(
            self,
            MarketPhase::BondingCurve | MarketPhase::Transitioning | MarketPhase::SteadyState
        )
    }

    /// Check if phase allows liquidity provision
    pub fn allows_liquidity(&self) -> bool {
        matches!(self, MarketPhase::Transitioning | MarketPhase::SteadyState)
    }

    /// Check if in bonding curve mode
    pub fn is_bonding(&self) -> bool {
        matches!(self, MarketPhase::BondingCurve)
    }

    /// Check if market is graduated
    pub fn is_graduated(&self) -> bool {
        matches!(self, MarketPhase::Graduated)
    }

    /// Validate phase transition
    pub fn can_transition_to(&self, new_phase: MarketPhase) -> bool {
        match (self, new_phase) {
            // Creation flow
            (MarketPhase::Created, MarketPhase::BondingCurve) => true,
            (MarketPhase::Created, MarketPhase::SteadyState) => true, // Direct launch

            // Bonding curve flow
            (MarketPhase::BondingCurve, MarketPhase::Transitioning) => true,
            (MarketPhase::Transitioning, MarketPhase::SteadyState) => true,
            (MarketPhase::SteadyState, MarketPhase::Graduated) => true,

            // Pause/unpause from any active phase
            (phase, MarketPhase::Paused) if phase.allows_trading() => true,
            (MarketPhase::Paused, phase) if phase.allows_trading() => true,

            // Deprecation is final
            (_, MarketPhase::Deprecated) => true,

            _ => false,
        }
    }
}

/// Phase transition event data
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct PhaseTransition {
    pub from_phase: MarketPhase,
    pub to_phase: MarketPhase,
    pub timestamp: i64,
    pub slot: u64,
    pub trigger: PhaseTrigger,
}

/// What triggered the phase transition
#[derive(Clone, Copy, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum PhaseTrigger {
    /// Manual governance action
    Governance,

    /// Automatic based on volume threshold
    VolumeThreshold,

    /// Automatic based on liquidity threshold
    LiquidityThreshold,

    /// Automatic based on time elapsed
    TimeElapsed,

    /// Safety circuit breaker
    SafetyTrigger,

    /// Creator action
    Creator,
}

/// Phase-specific parameters
#[derive(Clone, Debug, Default, AnchorSerialize, AnchorDeserialize)]
pub struct PhaseParams {
    /// Bonding curve parameters
    pub bonding_curve_supply: u64,
    pub bonding_curve_virtual_sol: u64,
    pub bonding_curve_virtual_token: u64,
    pub bonding_curve_fee_bps: u16,

    /// Graduation thresholds
    pub graduation_volume_threshold: u64,
    pub graduation_liquidity_threshold: u64,
    pub graduation_time_threshold: i64,

    /// Transition parameters
    pub transition_duration_slots: u64,
    pub transition_start_slot: u64,
}

impl PhaseParams {
    /// Check if graduation criteria met
    pub fn graduation_criteria_met(
        &self,
        total_volume: u64,
        total_liquidity: u128,
        elapsed_time: i64,
    ) -> bool {
        // Volume threshold
        if self.graduation_volume_threshold > 0 && total_volume >= self.graduation_volume_threshold
        {
            return true;
        }

        // Liquidity threshold
        if self.graduation_liquidity_threshold > 0
            && total_liquidity >= self.graduation_liquidity_threshold as u128
        {
            return true;
        }

        // Time threshold
        if self.graduation_time_threshold > 0 && elapsed_time >= self.graduation_time_threshold {
            return true;
        }

        false
    }
}
