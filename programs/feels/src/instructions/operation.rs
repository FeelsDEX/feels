//! # Operation Trait Framework
//! 
//! Provides a trait-based dispatcher pattern for instruction handlers.
//! This makes the code more modular, testable, and easier to extend.

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;

// ============================================================================
// Core Operation Trait
// ============================================================================

/// Base trait for all executable operations
pub trait Operation {
    /// The context type this operation requires
    type Context<'info>;
    
    /// The result type this operation returns
    type Result;
    
    /// Execute the operation
    fn execute<'info>(
        &self,
        ctx: Context<'_, '_, 'info, 'info, Self::Context<'info>>,
    ) -> Result<Self::Result>;
    
    /// Validate the operation before execution (optional)
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// Market Operation Trait
// ============================================================================

/// Trait for market-specific operations
pub trait MarketOperation: Operation {
    /// Get the operation name for logging
    fn name(&self) -> &'static str;
    
    /// Check if this operation requires admin authority
    fn requires_admin(&self) -> bool {
        false
    }
    
    /// Check if this operation requires keeper authority
    fn requires_keeper(&self) -> bool {
        false
    }
}

// ============================================================================
// Order Operation Trait
// ============================================================================

/// Trait for order/trading operations
pub trait OrderOperation: Operation {
    /// Get the order type name
    fn order_type(&self) -> &'static str;
    
    /// Calculate slippage tolerance
    fn slippage_tolerance(&self) -> Option<u16> {
        None
    }
    
    /// Check if this order requires conservation proof
    fn requires_conservation_proof(&self) -> bool {
        false
    }
}

// ============================================================================
// Maintenance Operation Trait
// ============================================================================

/// Trait for maintenance/keeper operations
pub trait MaintenanceOperation: Operation {
    /// Get the maintenance operation type
    fn operation_type(&self) -> &'static str;
    
    /// Check required authority level
    fn required_authority(&self) -> AuthorityLevel;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthorityLevel {
    None,
    Keeper,
    Admin,
    DataProvider,
}

// ============================================================================
// Operation Dispatcher
// ============================================================================

/// Generic dispatcher for operations
pub struct Dispatcher<T> {
    operation: T,
}

impl<T> Dispatcher<T> {
    pub fn new(operation: T) -> Self {
        Self { operation }
    }
}

impl<T: Operation> Dispatcher<T> {
    /// Dispatch the operation with validation
    pub fn dispatch<'info>(
        self,
        ctx: Context<'_, '_, 'info, 'info, T::Context<'info>>,
    ) -> Result<T::Result> {
        // Validate first
        self.operation.validate()?;
        
        // Then execute
        self.operation.execute(ctx)
    }
}

// ============================================================================
// Error Handling
// ============================================================================

/// Convert operation errors to protocol errors
pub trait OperationError {
    fn to_protocol_error(&self) -> FeelsProtocolError;
}

// ============================================================================
// Serialization Support
// ============================================================================

/// Trait for operations that can be deserialized from instruction data
pub trait DeserializableOperation: Sized {
    fn try_from_slice(data: &[u8]) -> Result<Self>;
}