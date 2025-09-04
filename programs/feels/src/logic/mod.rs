//! # Market Logic Engine
//! 
//! Core logic for 3D thermodynamic AMM with consolidated modules:
//! - State: P = (S,T,L) with potential V(P) = -Σ ŵᵢ ln(i)
//! - Work: W = V(P₂) - V(P₁) determines fees
//! - Conservation: Σ wᵢ ln(gᵢ) = 0
//! 
//! Modules:
//! 1. **Order System**: Single unified OrderManager for all operations
//! 2. **Thermodynamics**: Work calculations and fee determination
//! 3. **Conservation**: Thermodynamic invariant checking
//! 4. **Safety**: System stability and circuit breakers
//! 5. **Field Management**: State evolution, commitment verification
//! 6. **Core Components**: State access, fallback mode
//! 7. **AMM Engine**: Concentrated liquidity implementation

// ============================================================================
// 1. UNIFIED ORDER SYSTEM - Single OrderManager
// ============================================================================

/// Unified order system with single OrderManager
pub mod order_system;

/// WorkUnit-based order manager (new preferred approach)
pub mod order_manager;

// ============================================================================
// 2. THERMODYNAMIC ENGINE - Work and Fee Calculation
// ============================================================================

/// Thermodynamic work calculations and fee determination
pub mod thermodynamics;

// ============================================================================
// 3. CONSERVATION LAW - Fundamental Invariant
// ============================================================================

/// Conservation law enforcement and verification
pub mod conservation;

// ============================================================================
// 4. SYSTEM SAFETY - Stability and Circuit Breakers
// ============================================================================

/// System safety checks and stability mechanisms
pub mod safety;

// ============================================================================
// 5. FIELD MANAGEMENT - Market State Evolution  
// ============================================================================

/// Market field management and commitments
pub mod field_management;

// ============================================================================
// 6. CORE SYSTEM COMPONENTS
// ============================================================================

/// State access control
pub mod state_access;

/// WorkUnit-based state context for atomic operations
pub mod state_context;

/// Unified state access for consolidated Market account
pub mod unified_state_access;

/// Fallback mode handling
pub mod fallback_mode;

/// Unit of Work pattern for atomic operations
pub mod unit_of_work;

/// Unified unit of work for consolidated Market account
pub mod unified_work_unit;

// ============================================================================
// 7. CONCENTRATED LIQUIDITY ENGINE
// ============================================================================

/// Concentrated liquidity math
pub mod concentrated_liquidity;

/// Tick management
pub mod tick;

/// Position lifecycle
pub mod position_manager;

// ============================================================================
// 8. EXTENSIBILITY SYSTEM
// ============================================================================

/// Event system
pub mod event;

/// Hook extensibility
pub mod hook;

// ============================================================================
// RE-EXPORTS - Clean API Surface
// ============================================================================

// Thermodynamic engine
pub use thermodynamics::{
    // Core thermodynamic types
    ThermodynamicFeeParams, ThermodynamicFeeResult,
    PriceImprovement, OrderFeeResult,
    
    // Work calculation
    calculate_path_work, calculate_swap_work,
    
    // Fee calculation
    calculate_thermodynamic_fee, work_to_fee,
    calculate_price_improvement_bps, calculate_effective_fee_bps,
    
    // Order fees
    calculate_order_fees, calculate_3d_fees,
    
    // Rebate management
    validate_rebate_capacity, process_rebate_payment,
    distribute_fees_to_buffer,
    
    // Fallback and swap fees
    calculate_fallback_fees, calculate_swap_step_fees,
};

// Conservation law module
pub use conservation::{
    // Conservation types
    ConservationProof, ConservationGrowthFactors, ConservationWeightedLogs,
    ConservationOperation, ConservationSnapshot,
    
    // Conservation verification
    verify_conservation, validate_conservation_proof,
};

// System safety module
pub use safety::{
    // Safety types
    SafetyAssessment, SafetyWarning,
    LeverageOperation,
    
    // Safety checks
    check_leverage_safety, assess_system_safety,
    should_trigger_fallback, should_emergency_pause,
    
    // Safety constants
    MAX_NOTIONAL_PERCENT_OF_DEPTH, MIN_TWAP_WINDOW,
    PING_PONG_COOLDOWN, MAX_LEVERAGE_ADJUSTMENTS_PER_EPOCH,
    TWAP_CONFIDENCE_THRESHOLD,
};

// Unified order system (legacy)
pub use order_system::{
    OrderManager as LegacyOrderManager,
    OrderType as LegacyOrderType, OrderResult as LegacyOrderResult,
    HubRoute as LegacyHubRoute,
};

// WorkUnit-based order system (new preferred approach)
pub use order_manager::{
    OrderManager, OrderType, OrderResult, HubRoute,
    create_order_manager,
};

// Work result types (re-exported from feels-core)
pub use feels_core::types::{WorkResult, PathSegment};

// Field management
pub use field_management::{
    FieldUpdateContext, FieldUpdateMode,
    update_market_field_data,
    verify_field_commitment, verify_market_update_enhanced,
};

// State access - Legacy (to be deprecated)
pub use state_access::{
    StateContext as LegacyStateContext, MarketStateAccess, TickStateAccess, 
    PositionStateAccess, BufferStateAccess,
};

// WorkUnit-based state context (new preferred approach)
pub use state_context::{
    StateContext, MarketStateSnapshot,
    create_state_context,
};

// Unified state access
pub use unified_state_access::{
    UnifiedStateContext, MarketAccess,
    TickStateAccess as UnifiedTickStateAccess,
    PositionStateAccess as UnifiedPositionStateAccess,
    BufferStateAccess as UnifiedBufferStateAccess,
};

// Fallback mode
pub use fallback_mode::{
    FallbackModeManager, OperationalMode,
    FallbackModeConfig, FallbackModeStatus,
};

// Concentrated liquidity
pub use concentrated_liquidity::ConcentratedLiquidityMath;
pub use tick::TickManager;
pub use position_manager::PositionManager;

// Unit of Work
pub use unit_of_work::{WorkUnit, UnitOfWork};

// Unified Unit of Work
pub use unified_work_unit::{WorkUnit as UnifiedWorkUnit};

// Events and hooks
pub use event::*;
pub use hook::*;