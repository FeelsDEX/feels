/// Security module consolidating reentrancy protection and emergency controls.
/// Provides unified security infrastructure for safe market operations including
/// reentrancy guards and emergency circuit breakers.

use anchor_lang::prelude::*;
use crate::error::{FeelsError, FeelsProtocolError};

// ============================================================================
// Reentrancy Protection
// ============================================================================

/// Reentrancy guard status flags
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum ReentrancyStatus {
    /// Pool is unlocked and ready for operations
    Unlocked = 0,
    /// Pool is locked due to ongoing operation
    Locked = 1,
    /// Pool is in hook execution phase
    HookExecuting = 2,
}

impl TryFrom<u8> for ReentrancyStatus {
    type Error = crate::error::FeelsError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(ReentrancyStatus::Unlocked),
            1 => Ok(ReentrancyStatus::Locked),
            2 => Ok(ReentrancyStatus::HookExecuting),
            _ => Err(FeelsError::StateError),
        }
    }
}

impl Default for ReentrancyStatus {
    fn default() -> Self {
        ReentrancyStatus::Unlocked
    }
}

/// Reentrancy guard manager
pub struct ReentrancyGuard;

impl ReentrancyGuard {
    /// Acquire lock for pool operation
    pub fn acquire(status: &mut ReentrancyStatus) -> Result<()> {
        match *status {
            ReentrancyStatus::Unlocked => {
                *status = ReentrancyStatus::Locked;
                Ok(())
            }
            _ => Err(FeelsProtocolError::ReentrancyDetected.into()),
        }
    }

    /// Acquire lock specifically for hook execution
    pub fn acquire_for_hooks(status: &mut ReentrancyStatus) -> Result<()> {
        match *status {
            ReentrancyStatus::Locked => {
                *status = ReentrancyStatus::HookExecuting;
                Ok(())
            }
            _ => Err(FeelsProtocolError::ReentrancyDetected.into()),
        }
    }

    /// Release lock after operation completes
    pub fn release(status: &mut ReentrancyStatus) -> Result<()> {
        match *status {
            ReentrancyStatus::Locked | ReentrancyStatus::HookExecuting => {
                *status = ReentrancyStatus::Unlocked;
                Ok(())
            }
            ReentrancyStatus::Unlocked => {
                // Already unlocked, this might indicate a bug
                msg!("Warning: Attempting to release already unlocked pool");
                Ok(())
            }
        }
    }

    /// Check if pool is currently locked
    pub fn is_locked(status: &ReentrancyStatus) -> bool {
        *status != ReentrancyStatus::Unlocked
    }

    /// Ensure pool is not locked (for read operations)
    pub fn ensure_unlocked(status: &ReentrancyStatus) -> Result<()> {
        if Self::is_locked(status) {
            return Err(FeelsProtocolError::ReentrancyDetected.into());
        }
        Ok(())
    }
}

/// RAII-style guard that automatically releases on drop
pub struct ScopedReentrancyGuard<'a> {
    status: &'a mut ReentrancyStatus,
}

impl<'a> ScopedReentrancyGuard<'a> {
    /// Create a new scoped guard that acquires the lock
    pub fn new(status: &'a mut ReentrancyStatus) -> Result<Self> {
        ReentrancyGuard::acquire(status)?;
        Ok(Self { status })
    }

    /// Transition to hook execution phase
    pub fn enter_hook_phase(&mut self) -> Result<()> {
        if *self.status == ReentrancyStatus::Locked {
            *self.status = ReentrancyStatus::HookExecuting;
            Ok(())
        } else {
            Err(FeelsError::StateError.into())
        }
    }
}

impl<'a> Drop for ScopedReentrancyGuard<'a> {
    fn drop(&mut self) {
        // Always release the lock on drop
        let _ = ReentrancyGuard::release(self.status);
    }
}

// ============================================================================
// Emergency Controls
// ============================================================================

/// Emergency operation flags for protocol safety
#[account(zero_copy)]
#[repr(C)]
pub struct EmergencyFlags {
    /// Market this applies to
    pub market: Pubkey,
    
    /// Authority that can modify flags
    pub emergency_authority: Pubkey,
    
    /// Pause all swaps (0 = false, 1 = true)
    pub pause_swaps: u8,
    
    /// Pause liquidity operations (0 = false, 1 = true)
    pub pause_liquidity: u8,
    
    /// Pause leverage operations (0 = false, 1 = true)
    pub pause_leverage: u8,
    
    /// Pause rebates (0 = false, 1 = true)
    pub pause_rebates: u8,
    
    /// Force maximum fees (0 = false, 1 = true)
    pub force_max_fees: u8,
    
    /// Emergency mode active (0 = false, 1 = true)
    pub emergency_mode: u8,
    
    /// Padding to align to 8-byte boundary
    pub _padding: [u8; 2],
    
    /// Time emergency was activated
    pub emergency_activated_at: i64,
    
    /// Reason for emergency (max 64 chars)
    pub emergency_reason: [u8; 64],
    
    /// Reserved flags for future use
    pub _reserved_flags: [u8; 8],
    
    /// Reserved space
    pub _reserved: [u8; 128],
}

impl EmergencyFlags {
    pub const SIZE: usize = 32 + 32 + 1 + 1 + 1 + 1 + 1 + 1 + 2 + 8 + 64 + 8 + 128;
    
    /// Check if any operations are paused
    pub fn is_operational(&self) -> bool {
        self.emergency_mode == 0 && self.pause_swaps == 0
    }
    
    /// Activate emergency mode
    pub fn activate_emergency(&mut self, reason: &str, timestamp: i64) {
        self.emergency_mode = 1;
        self.emergency_activated_at = timestamp;
        
        // Copy reason (truncate if needed)
        let reason_bytes = reason.as_bytes();
        let len = reason_bytes.len().min(64);
        self.emergency_reason[..len].copy_from_slice(&reason_bytes[..len]);
        
        // Set all pause flags
        self.pause_swaps = 1;
        self.pause_liquidity = 1;
        self.pause_leverage = 1;
        self.pause_rebates = 1;
        self.force_max_fees = 1;
        
        msg!("Emergency mode activated: {}", reason);
    }
    
    /// Deactivate emergency mode
    pub fn deactivate_emergency(&mut self) {
        self.emergency_mode = 0;
        self.pause_swaps = 0;
        self.pause_liquidity = 0;
        self.pause_leverage = 0;
        self.pause_rebates = 0;
        self.force_max_fees = 0;
        self.emergency_reason = [0; 64];
        
        msg!("Emergency mode deactivated");
    }
}

// ============================================================================
// Instructions
// ============================================================================

/// Initialize emergency flags instruction
#[derive(Accounts)]
pub struct InitializeEmergencyFlags<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// Protocol state
    #[account(
        seeds = [b"protocol"],
        bump,
        has_one = authority,
    )]
    pub protocol: Account<'info, crate::state::ProtocolState>,
    
    /// Market field
    pub market_field: Account<'info, crate::state::MarketField>,
    
    /// Emergency flags to initialize
    #[account(
        init,
        payer = authority,
        space = 8 + EmergencyFlags::SIZE,
        seeds = [b"emergency_flags", market_field.pool.as_ref()],
        bump,
    )]
    pub emergency_flags: AccountLoader<'info, EmergencyFlags>,
    
    pub system_program: Program<'info, System>,
}

pub fn initialize_emergency_flags(
    ctx: Context<InitializeEmergencyFlags>,
    emergency_authority: Pubkey,
) -> Result<()> {
    let mut flags = ctx.accounts.emergency_flags.load_init()?;
    
    flags.market = ctx.accounts.market_field.pool;
    flags.emergency_authority = emergency_authority;
    flags.pause_swaps = 0;
    flags.pause_liquidity = 0;
    flags.pause_leverage = 0;
    flags.pause_rebates = 0;
    flags.force_max_fees = 0;
    flags.emergency_mode = 0;
    flags._padding = [0; 2];
    flags.emergency_activated_at = 0;
    flags.emergency_reason = [0; 64];
    flags._reserved_flags = [0; 8];
    flags._reserved = [0; 128];
    
    msg!("Initialized emergency flags for market {}", flags.market);
    
    Ok(())
}

/// Toggle emergency mode instruction
#[derive(Accounts)]
pub struct ToggleEmergencyMode<'info> {
    /// Emergency authority
    #[account(
        constraint = authority.key() == emergency_flags.load()?.emergency_authority
            @ crate::error::FeelsProtocolError::Unauthorized
    )]
    pub authority: Signer<'info>,
    
    /// Emergency flags
    #[account(mut)]
    pub emergency_flags: AccountLoader<'info, EmergencyFlags>,
    
    pub clock: Sysvar<'info, Clock>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct EmergencyModeParams {
    pub activate: bool,
    pub reason: String,
}

pub fn toggle_emergency_mode(
    ctx: Context<ToggleEmergencyMode>,
    params: EmergencyModeParams,
) -> Result<()> {
    let mut flags = ctx.accounts.emergency_flags.load_mut()?;
    let timestamp = ctx.accounts.clock.unix_timestamp;
    
    if params.activate {
        flags.activate_emergency(&params.reason, timestamp);
    } else {
        flags.deactivate_emergency();
    }
    
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reentrancy_guard_lifecycle() {
        let mut status = ReentrancyStatus::Unlocked;

        // Should acquire lock successfully
        assert!(ReentrancyGuard::acquire(&mut status).is_ok());
        assert_eq!(status, ReentrancyStatus::Locked);

        // Should fail to acquire again
        assert!(ReentrancyGuard::acquire(&mut status).is_err());

        // Should transition to hook phase
        assert!(ReentrancyGuard::acquire_for_hooks(&mut status).is_ok());
        assert_eq!(status, ReentrancyStatus::HookExecuting);

        // Should release successfully
        assert!(ReentrancyGuard::release(&mut status).is_ok());
        assert_eq!(status, ReentrancyStatus::Unlocked);
    }

    #[test]
    fn test_scoped_guard() {
        let mut status = ReentrancyStatus::Unlocked;

        {
            let guard = ScopedReentrancyGuard::new(&mut status);
            assert!(guard.is_ok());
            assert_eq!(status, ReentrancyStatus::Locked);
            // Guard automatically releases on drop
        }

        assert_eq!(status, ReentrancyStatus::Unlocked);
    }
}