/// Hook system definitions for extensible pool behavior in Phase 2.
/// Defines event types, execution stages, and hook registry structures that allow
/// external programs to observe and react to pool operations. Uses bitmask patterns
/// for efficient event filtering and supports hook permission levels for security.

use anchor_lang::prelude::*;
use borsh::{BorshSerialize, BorshDeserialize};
use crate::constant::MAX_HOOKS_PER_POOL;

// ============================================================================
// Event and Stage Definitions (Bitmasks for Gas Efficiency)
// ============================================================================

// Lifecycle events as bit flags (up to 32 events)
pub const EVENT_POOL_INITIALIZED: u32 = 1 << 0;
pub const EVENT_RATE_UPDATED: u32 = 1 << 1;
pub const EVENT_LIQUIDITY_CHANGED: u32 = 1 << 2;
pub const EVENT_FEES_COLLECTED: u32 = 1 << 3;
pub const EVENT_TICK_CROSSED: u32 = 1 << 4;
pub const EVENT_TICK_ACTIVATED: u32 = 1 << 5;
pub const EVENT_TICK_DEACTIVATED: u32 = 1 << 6;
pub const EVENT_POSITION_OPENED: u32 = 1 << 7;
pub const EVENT_POSITION_MODIFIED: u32 = 1 << 8;
pub const EVENT_POSITION_CLOSED: u32 = 1 << 9;
pub const EVENT_SWAP_EXECUTED: u32 = 1 << 10;
pub const EVENT_ORDER_CREATED: u32 = 1 << 11;
pub const EVENT_ORDER_FILLED: u32 = 1 << 12;
pub const EVENT_ORDER_MODIFIED: u32 = 1 << 13;
pub const EVENT_REDENOMINATION: u32 = 1 << 14;

// Type alias for backwards compatibility
pub const EVENT_PRICE_UPDATED: u32 = EVENT_RATE_UPDATED;

// Hook execution stages as bit flags (up to 8 stages)
pub const STAGE_VALIDATE: u8 = 1 << 0;      // Can abort operation
pub const STAGE_PRE_EXECUTE: u8 = 1 << 1;   // Can modify parameters
pub const STAGE_POST_EXECUTE: u8 = 1 << 2;  // Can observe results
pub const STAGE_ASYNC: u8 = 1 << 3;         // Creates message for off-chain

// ============================================================================
// Hook Configuration
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct HookConfig {
    /// Hook program address
    pub program_id: Pubkey,
    
    /// Bitmask of events this hook subscribes to
    pub event_mask: u32,
    
    /// Bitmask of stages this hook executes in
    pub stage_mask: u8,
    
    /// Permission level (0=Disabled, 1=ReadOnly, 2=Modify, 3=Halt)
    pub permission: u8,
    
    /// Whether hook is currently active
    pub enabled: bool,
    
    /// Maximum compute units allowed for this hook
    pub max_compute_units: u32,
    
    /// Statistics for monitoring
    pub call_count: u64,
    pub last_execution_slot: u64,
}

impl HookConfig {
    pub const SIZE: usize = 32 + 4 + 1 + 1 + 1 + 4 + 8 + 8;
    
    #[inline(always)]
    pub fn subscribes_to(&self, event: u32, stage: u8) -> bool {
        self.enabled && 
        (self.event_mask & event) != 0 && 
        (self.stage_mask & stage) != 0
    }
    
    pub fn permission_level(&self) -> HookPermission {
        match self.permission {
            0 => HookPermission::Disabled,
            1 => HookPermission::ReadOnly,
            2 => HookPermission::Modify,
            3 => HookPermission::Halt,
            _ => HookPermission::Disabled,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookType {
    /// Price feed and oracle hooks
    PriceFeed = 0,
    /// Liquidity management hooks  
    Liquidity = 1,
    /// Arbitrage and MEV hooks
    Arbitrage = 2,
    /// Custom validation hooks
    Validation = 3,
    /// Before swap execution
    BeforeSwap = 4,
    /// After swap execution
    AfterSwap = 5,
    /// Before liquidity add
    BeforeAdd = 6,
    /// After liquidity add
    AfterAdd = 7,
    /// Before liquidity remove
    BeforeRemove = 8,
    /// After liquidity remove
    AfterRemove = 9,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BorshSerialize, BorshDeserialize)]
pub enum HookPermission {
    Disabled = 0,
    ReadOnly = 1,    // Can only observe
    Modify = 2,      // Can modify non-critical state
    Halt = 3,        // Can abort operations
}

impl TryFrom<u8> for HookPermission {
    type Error = crate::state::FeelsProtocolError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(HookPermission::Disabled),
            1 => Ok(HookPermission::ReadOnly),
            2 => Ok(HookPermission::Modify),
            3 => Ok(HookPermission::Halt),
            _ => Err(crate::state::FeelsProtocolError::InvalidPermission),
        }
    }
}

// ============================================================================
// Hook Registry (Per Pool)
// ============================================================================

#[account]
pub struct HookRegistry {
    /// Pool this registry belongs to
    pub pool: Pubkey,
    
    /// Authority who can modify hooks
    pub authority: Pubkey,
    
    /// Fixed array of hook configurations
    pub hooks: [HookConfig; MAX_HOOKS_PER_POOL],
    
    /// Number of active hooks
    pub hook_count: u8,
    
    /// Global enable/disable flag
    pub hooks_enabled: bool,
    
    /// Whether to write messages to queue
    pub message_queue_enabled: bool,
    
    /// Emergency pause authority
    pub emergency_authority: Option<Pubkey>,
    
    /// Last update timestamp
    pub last_update_timestamp: i64,
    
    /// Reserved for future upgrades
    pub _reserved: [u8; 64],
}

impl HookRegistry {
    pub const SIZE: usize = 8 + 32 + 32 + (HookConfig::SIZE * MAX_HOOKS_PER_POOL) 
        + 1 + 1 + 1 + 33 + 8 + 64;
    
    /// Get hooks for a specific event and stage (gas-optimized)
    #[inline(always)]
    pub fn get_hooks_for(&self, event: u32, stage: u8) -> Vec<&HookConfig> {
        if !self.hooks_enabled {
            return Vec::new();
        }
        
        self.hooks[..self.hook_count as usize]
            .iter()
            .filter(|h| h.subscribes_to(event, stage))
            .collect()
    }
    
    pub fn register_hook(
        &mut self,
        program_id: Pubkey,
        event_mask: u32,
        stage_mask: u8,
        permission: HookPermission,
    ) -> Result<usize> {
        use crate::state::FeelsProtocolError;
        
        require!(
            self.hook_count < MAX_HOOKS_PER_POOL as u8,
            FeelsProtocolError::HookRegistryFull
        );
        
        let index = self.hook_count as usize;
        self.hooks[index] = HookConfig {
            program_id,
            event_mask,
            stage_mask,
            permission: permission as u8,
            enabled: true,
            max_compute_units: 100_000,
            call_count: 0,
            last_execution_slot: 0,
        };
        self.hook_count += 1;
        
        Ok(index)
    }
}

// ============================================================================
// Hook Execution Context
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct HookContext {
    /// Pool being operated on
    pub pool: Pubkey,
    
    /// User performing the operation
    pub user: Pubkey,
    
    /// Current event being processed
    pub event: u32,
    
    /// Execution stage
    pub stage: u8,
    
    /// Event-specific data
    pub event_data: EventData,
    
    /// Timestamp
    pub timestamp: i64,
    
    /// Slot for ordering
    pub slot: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum EventData {
    PriceUpdate {
        old_sqrt_price: u128,
        new_sqrt_price: u128,
    },
    LiquidityChange {
        position_id: Pubkey,
        liquidity_delta: i128,
        amount_0: u64,
        amount_1: u64,
        tick_lower: i32,
        tick_upper: i32,
    },
    TickCross {
        from_tick: i32,
        to_tick: i32,
        liquidity_net: i128,
    },
    Swap {
        amount_in: u64,
        amount_out: u64,
        token_in: Pubkey,
        token_out: Pubkey,
        fee_amount: u64,
    },
}

// ============================================================================
// Message Queue for Async Processing
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct HookMessage {
    pub event_type: u16,
    pub pool: Pubkey,
    pub timestamp: i64,
    pub slot: u64,
    pub data: MessageData,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum MessageData {
    PriceUpdate { old_price: u64, new_price: u64 },
    LiquidityChange { position_id: [u8; 32], delta: i64 },
    TickCross { from: i32, to: i32 },
    SwapExecuted { volume: u64, fee: u64 },
    Generic { data: [u8; 32] },
}

#[account]
pub struct HookMessageQueue {
    /// Pool this queue belongs to
    pub pool: Pubkey,
    
    /// Ring buffer of messages
    pub messages: [HookMessage; 32],
    
    /// Ring buffer indices
    pub head: u8,
    pub tail: u8,
    
    /// Total messages written (for off-chain tracking)
    pub sequence: u64,
    
    /// Last acknowledged sequence by off-chain processor
    pub ack_sequence: u64,
    
    /// Reserved space
    pub _reserved: [u8; 64],
}

impl HookMessageQueue {
    pub const SIZE: usize = 8 + 32 + (88 * 32) + 1 + 1 + 8 + 8 + 64;
    
    pub fn push(&mut self, message: HookMessage) -> Result<()> {
        use crate::state::FeelsProtocolError;
        
        let next_head = (self.head.wrapping_add(1)) % 32;
        require!(next_head != self.tail, FeelsProtocolError::MessageQueueFull);
        
        self.messages[self.head as usize] = message;
        self.head = next_head;
        self.sequence = self.sequence.saturating_add(1);
        Ok(())
    }
    
    pub fn is_full(&self) -> bool {
        ((self.head.wrapping_add(1)) % 32) == self.tail
    }
}