/// Reentrancy protection system preventing reentrant attacks during pool operations.
/// Implements a state-based locking mechanism that prevents multiple concurrent
/// operations on the same pool. Essential for ensuring external hooks don't trigger
/// reentrancy attacks through cross-program invocations.

use anchor_lang::prelude::*;
use crate::error::FeelsError;
use crate::state::FeelsProtocolError;

// ============================================================================
// Reentrancy Status Types
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

// ============================================================================
// Reentrancy Guard Manager
// ============================================================================

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