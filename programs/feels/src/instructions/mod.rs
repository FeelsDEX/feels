//! # Instruction Handlers
//! 
//! Entry points for 3D thermodynamic AMM operations:
//! 
//! 1. **Market**: Initialize and update markets with unified configuration
//! 2. **Order**: Universal trading handler for all order types (swaps, liquidity, positions)
//! 3. **Maintenance**: Keeper operations, cleanups, rebases, and system maintenance
//! 4. **Token**: Asset creation and management
//! 
//! All operations calculate work W = V(P₂) - V(P₁) and enforce conservation.

// ============================================================================
// INSTRUCTION MODULES
// ============================================================================

/// Operation trait framework for modular handlers
pub mod operation;

/// Market operations for consolidated Market account
pub mod market;

/// Order handler using consolidated Market account
pub mod order;

/// Maintenance operations: keeper registry, cleanup, rebases
pub mod maintenance;

/// Token management
pub mod token;

// ============================================================================
// PUBLIC API EXPORTS
// ============================================================================

// Market operations
pub use market::{
    // Initialization
    initialize_market,
    InitializeMarket,
    InitializeMarketParams,
    
    // Updates
    update_market,
    UpdateMarket,
    UpdateMarketParams,
    
    // Pause/Unpause
    pause_market,
    unpause_market,
    PauseMarket,
    UnpauseMarket,
};

// Order operations
pub use order::{
    handler as order_handler,
    UnifiedOrder as Order,
    OrderParams,
    OrderResult,
    OrderType,
    PositionType,
};

// Maintenance operations
pub use maintenance::{
    handler as maintenance_handler,
    MaintenanceOperation,
    RebaseType,
    PoolOperationalStatus,
    KeeperRegistry,
    PoolStatus,
    MaintenanceAccounts,
};

// Token operations
pub use token::{
    handler as token_create_handler,
    TokenCreateParams,
    TokenCreateResult,
    CreateToken,
};