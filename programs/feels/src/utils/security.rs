/// Centralized security utilities and macros for the Feels Protocol.
/// Provides consistent reentrancy protection and authority validation across all instructions.
use anchor_lang::prelude::*;
use crate::state::{ReentrancyStatus, ReentrancyGuard}; 
// MarketManager, ProtocolState, ScopedReentrancyGuard // Unused imports
use crate::error::FeelsProtocolError;

// ============================================================================
// Security Macros
// ============================================================================

/// Macro for consistent reentrancy guard application
/// Usage: `apply_reentrancy_guard!(ctx.accounts.pool)`
#[macro_export]
macro_rules! apply_reentrancy_guard {
    ($pool:expr) => {{
        use $crate::state::{ReentrancyStatus, ReentrancyGuard};
        
        let mut pool = $pool.load_mut()?;
        let mut status = match pool.reentrancy_status {
            1 => ReentrancyStatus::Locked,
            _ => ReentrancyStatus::Unlocked,
        };
        ReentrancyGuard::acquire(&mut status)?;
        pool.reentrancy_status = status as u8;
        Ok::<_, anchor_lang::error::Error>(())
    }};
}

/// Macro for authority validation
/// Usage: `validate_authority!(ctx.accounts.authority, expected_authority)`
#[macro_export]
macro_rules! validate_authority {
    ($signer:expr, $expected:expr) => {{
        require!(
            $signer.key() == $expected,
            $crate::error::FeelsProtocolError::InvalidAuthority
        );
    }};
}

/// Macro for pool authority validation
/// Usage: `validate_pool_authority!(ctx.accounts.authority, ctx.accounts.pool)`
#[macro_export]
macro_rules! validate_pool_authority {
    ($signer:expr, $pool_loader:expr) => {{
        let pool = $pool_loader.load()?;
        require!(
            $signer.key() == pool.authority,
            $crate::error::FeelsProtocolError::InvalidAuthority
        );
    }};
}

/// Macro for protocol authority validation
/// Usage: `validate_protocol_authority!(ctx.accounts.authority, ctx.accounts.protocol_state)`
#[macro_export]
macro_rules! validate_protocol_authority {
    ($signer:expr, $protocol:expr) => {{
        require!(
            $signer.key() == $protocol.authority,
            $crate::error::FeelsProtocolError::InvalidAuthority
        );
    }};
}

/// Combined security check macro for common instruction pattern
/// Usage: `apply_security_checks!(ctx, require_reentrancy_guard: true, require_pool_authority: true)`
#[macro_export]
macro_rules! apply_security_checks {
    ($ctx:expr, require_reentrancy_guard: $reentrancy:expr, require_pool_authority: $pool_auth:expr) => {{
        // Apply reentrancy guard if required
        let _guard = if $reentrancy {
            Some($crate::apply_reentrancy_guard!($ctx.accounts.pool)?)
        } else {
            None
        };
        
        // Validate pool authority if required
        if $pool_auth {
            $crate::validate_pool_authority!($ctx.accounts.authority, $ctx.accounts.pool);
        }
        
        _guard
    }};
}

// ============================================================================
// Security Functions
// ============================================================================

/// Validate that an account is initialized
pub fn validate_initialized<T: AccountDeserialize>(_account: &T) -> Result<()> {
    // The fact that we can deserialize means it's initialized
    // This is a placeholder for any additional validation
    Ok(())
}

/// Validate that a value is within acceptable bounds
pub fn validate_bounds<T: PartialOrd>(value: T, min: T, max: T, _field_name: &str) -> Result<()> {
    require!(
        value >= min && value <= max,
        FeelsProtocolError::InvalidParameter
    );
    Ok(())
}

/// Validate freshness of data
pub fn validate_freshness(timestamp: i64, max_age: i64, current_time: i64) -> Result<()> {
    require!(
        current_time - timestamp <= max_age,
        FeelsProtocolError::StaleData
    );
    Ok(())
}

/// Validate rate of change for updates
pub fn validate_rate_of_change(old_value: u128, new_value: u128, max_change_bps: u32) -> Result<()> {
    if old_value == 0 {
        // Allow any change from zero
        return Ok(());
    }
    
    let change = if new_value > old_value {
        new_value - old_value
    } else {
        old_value - new_value
    };
    
    let change_bps = ((change * 10000) / old_value) as u32;
    
    require!(
        change_bps <= max_change_bps,
        FeelsProtocolError::ExcessiveChange
    );
    
    Ok(())
}

// ============================================================================
// Scoped Guards
// ============================================================================

/// Enhanced reentrancy guard with automatic cleanup
pub struct ScopedSecurityGuard<'info> {
    _reentrancy_guard: Option<()>,
    pub pool: &'info AccountLoader<'info, crate::state::MarketManager>,
}

impl<'info> ScopedSecurityGuard<'info> {
    /// Create a new security guard with specified checks
    pub fn new(
        pool: &'info AccountLoader<'info, crate::state::MarketManager>,
        require_reentrancy: bool,
    ) -> Result<Self> {
        let _reentrancy_guard = if require_reentrancy {
            let mut pool_mut = pool.load_mut()?;
            let mut status = match pool_mut.reentrancy_status {
                1 => ReentrancyStatus::Locked,
                _ => ReentrancyStatus::Unlocked,
            };
            ReentrancyGuard::acquire(&mut status)?;
            pool_mut.reentrancy_status = status as u8;
            Some(())
        } else {
            None
        };
        
        Ok(Self {
            _reentrancy_guard,
            pool,
        })
    }
    
    /// Get immutable pool reference
    pub fn pool(&self) -> Result<std::cell::Ref<crate::state::MarketManager>> {
        self.pool.load()
    }
    
    /// Get mutable pool reference (only if no reentrancy guard)
    pub fn pool_mut(&self) -> Result<std::cell::RefMut<crate::state::MarketManager>> {
        require!(
            self._reentrancy_guard.is_none(),
            FeelsProtocolError::ReentrancyDetected
        );
        self.pool.load_mut()
    }
}

// ============================================================================
// Common Validation Patterns
// ============================================================================

/// Validate swap parameters
pub fn validate_swap_params(
    amount_in: u64,
    min_amount_out: u64,
    sqrt_rate_limit: Option<u128>,
) -> Result<()> {
    require!(
        amount_in > 0,
        FeelsProtocolError::InvalidParameter
    );
    
    require!(
        min_amount_out > 0,
        FeelsProtocolError::InvalidParameter
    );
    
    if let Some(limit) = sqrt_rate_limit {
        require!(
            limit > 0,
            FeelsProtocolError::InvalidParameter
        );
    }
    
    Ok(())
}

/// Validate liquidity parameters
pub fn validate_liquidity_params(
    tick_lower: i32,
    tick_upper: i32,
    liquidity_amount: u128,
) -> Result<()> {
    require!(
        tick_lower < tick_upper,
        FeelsProtocolError::InvalidParameter
    );
    
    require!(
        liquidity_amount > 0,
        FeelsProtocolError::InvalidParameter
    );
    
    // Check tick bounds
    use crate::constant::{MIN_TICK, MAX_TICK};
    validate_bounds(tick_lower, MIN_TICK, MAX_TICK, "tick_lower")?;
    validate_bounds(tick_upper, MIN_TICK, MAX_TICK, "tick_upper")?;
    
    Ok(())
}