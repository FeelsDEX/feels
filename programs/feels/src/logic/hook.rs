/// Hook execution logic for extensible pool behaviors and external program integration.
/// Manages hook registry operations, CPI calls to registered hooks, and async message queues.
/// Supports validation, pre/post execution stages with different permission levels for security.
/// Gas-optimized implementation minimizes overhead when no hooks are registered.

use anchor_lang::prelude::*;
use std::collections::BTreeMap;
use crate::state::{
    HookRegistry, HookConfig, HookMessage, HookMessageQueue,
    EventData, MessageData, HookPermission, FeelsProtocolError,
};

// Re-export all hook constants from state module - single source of truth
pub use crate::state::{
    // Event constants
    EVENT_POOL_INITIALIZED, EVENT_RATE_UPDATED, EVENT_LIQUIDITY_CHANGED,
    EVENT_FEES_COLLECTED, EVENT_TICK_CROSSED, EVENT_TICK_ACTIVATED, 
    EVENT_TICK_DEACTIVATED, EVENT_POSITION_OPENED, EVENT_POSITION_MODIFIED,
    EVENT_POSITION_CLOSED, EVENT_SWAP_EXECUTED, EVENT_ORDER_CREATED,
    EVENT_ORDER_FILLED, EVENT_ORDER_MODIFIED, EVENT_REDENOMINATION,
    EVENT_TICK_CHANGED, EVENT_REBASE_APPLIED, EVENT_LEVERAGE_ADJUSTED,
    // Stage constants
    STAGE_VALIDATE, STAGE_PRE_EXECUTE, STAGE_POST_EXECUTE, STAGE_ASYNC,
};

// Import the state HookContext for serialization
use crate::state::{HookContext as StateHookContext};

/// Extended hook context for internal use
#[derive(Clone, Debug)]
pub struct HookContext {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub event: u32,
    pub stage: u8,
    pub event_data: EventData,
    pub timestamp: i64,
    pub slot: u64,
    pub data: BTreeMap<String, String>,
}

impl HookContext {
    /// Convert to state HookContext for serialization
    pub fn to_state_context(&self) -> StateHookContext {
        StateHookContext {
            pool: self.pool,
            user: self.user,
            event: self.event,
            stage: self.stage,
            event_data: self.event_data.clone(),
            timestamp: self.timestamp,
            slot: self.slot,
        }
    }
}

// Add AnchorSerialize/Deserialize implementation for HookContext
impl AnchorSerialize for HookContext {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Serialize as StateHookContext for CPI
        self.to_state_context().serialize(writer)
    }
}

impl AnchorDeserialize for HookContext {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        // Deserialize from StateHookContext
        let state_context = StateHookContext::deserialize_reader(reader)?;
        Ok(Self {
            pool: state_context.pool,
            user: state_context.user,
            event: state_context.event,
            stage: state_context.stage,
            event_data: state_context.event_data,
            timestamp: state_context.timestamp,
            slot: state_context.slot,
            data: BTreeMap::new(),
        })
    }
}

// ============================================================================
// Hook Executor - Gas Optimized
// ============================================================================

pub struct HookExecutor;

impl HookExecutor {
    /// Main entry point - executes hooks if present
    #[inline(always)]
    pub fn execute(
        registry: Option<&Account<HookRegistry>>,
        event: u32,
        stage: u8,
        context: &HookContext,
        message_queue: Option<&mut Account<HookMessageQueue>>,
        remaining_accounts: &[AccountInfo],
    ) -> Result<()> {
        // Early exit if no registry
        let registry = match registry {
            Some(r) if r.hooks_enabled => r,
            _ => return Ok(()),
        };
        
        // Handle async stage separately (no CPI)
        if stage == STAGE_ASYNC {
            return Self::handle_async_stage(registry, message_queue, context);
        }
        
        // Get relevant hooks
        let hooks = registry.get_hooks_for(event, stage);
        if hooks.is_empty() {
            return Ok(());
        }
        
        // Execute based on stage
        match stage {
            STAGE_VALIDATE => Self::execute_validation(hooks, context, remaining_accounts),
            STAGE_PRE_EXECUTE => Self::execute_pre(hooks, context, remaining_accounts),
            STAGE_POST_EXECUTE => Self::execute_post(hooks, context, remaining_accounts),
            _ => Ok(()),
        }
    }
    
    #[inline]
    fn execute_validation(
        hooks: Vec<&HookConfig>,
        context: &HookContext,
        accounts: &[AccountInfo],
    ) -> Result<()> {
        let mut account_offset = 0;
        
        for hook in hooks {
            let permission = hook.permission_level();
            
            // Only Halt permission can actually abort in validation
            if permission == HookPermission::Halt {
                let result = Self::invoke_hook(
                    hook,
                    context,
                    &accounts[account_offset..],
                );
                
                if result.is_err() {
                    return Err(FeelsProtocolError::HookValidationFailed.into());
                }
            }
            
            // Move offset for next hook's accounts
            account_offset += Self::get_hook_account_count(hook);
        }
        Ok(())
    }
    
    #[inline]
    fn execute_pre(
        hooks: Vec<&HookConfig>,
        context: &HookContext,
        accounts: &[AccountInfo],
    ) -> Result<()> {
        let mut account_offset = 0;
        
        for hook in hooks {
            let permission = hook.permission_level();
            
            if permission >= HookPermission::Modify {
                Self::invoke_hook(hook, context, &accounts[account_offset..])?;
            }
            
            account_offset += Self::get_hook_account_count(hook);
        }
        Ok(())
    }
    
    #[inline]
    fn execute_post(
        hooks: Vec<&HookConfig>,
        context: &HookContext,
        accounts: &[AccountInfo],
    ) -> Result<()> {
        let mut account_offset = 0;
        
        for hook in hooks {
            // Post hooks can't halt, so we ignore errors
            let _ = Self::invoke_hook(hook, context, &accounts[account_offset..]);
            account_offset += Self::get_hook_account_count(hook);
        }
        Ok(())
    }
    
    #[inline]
    fn handle_async_stage(
        registry: &Account<HookRegistry>,
        message_queue: Option<&mut Account<HookMessageQueue>>,
        context: &HookContext,
    ) -> Result<()> {
        // Only process if queue is enabled and present
        let queue = match (registry.message_queue_enabled, message_queue) {
            (true, Some(q)) => q,
            _ => return Ok(()),
        };
        
        // Don't push if queue is full
        if queue.is_full() {
            return Ok(());
        }
        
        // Convert context to message
        let message = Self::context_to_message(context)?;
        queue.push(message)?;
        
        Ok(())
    }
    
    fn invoke_hook(
        hook: &HookConfig,
        context: &HookContext,
        accounts: &[AccountInfo],
    ) -> Result<()> {
        // Serialize context (using state context for CPI)
        let data = context.to_state_context().try_to_vec()?;
        
        // Build instruction
        let instruction = solana_program::instruction::Instruction {
            program_id: hook.program_id,
            accounts: accounts
                .iter()
                .map(|acc| AccountMeta {
                    pubkey: *acc.key,
                    is_signer: acc.is_signer,
                    is_writable: acc.is_writable,
                })
                .collect(),
            data,
        };
        
        // Execute CPI
        solana_program::program::invoke(&instruction, accounts)?;
        
        Ok(())
    }
    
    fn context_to_message(context: &HookContext) -> Result<HookMessage> {
        let data = match &context.event_data {
            EventData::PriceUpdate { old_sqrt_price, new_sqrt_price } => {
                MessageData::PriceUpdate {
                    old_price: (*old_sqrt_price >> 64) as u64,
                    new_price: (*new_sqrt_price >> 64) as u64,
                }
            },
            EventData::LiquidityChange { position_id, liquidity_delta, .. } => {
                MessageData::LiquidityChange {
                    position_id: position_id.to_bytes(),
                    delta: *liquidity_delta as i64,
                }
            },
            EventData::TickCross { from_tick, to_tick, .. } => {
                MessageData::TickCross {
                    from: *from_tick,
                    to: *to_tick,
                }
            },
            EventData::Swap { amount_in, fee_amount, .. } => {
                MessageData::SwapExecuted {
                    volume: *amount_in,
                    fee: *fee_amount,
                }
            },
        };
        
        Ok(HookMessage {
            event_type: context.event.trailing_zeros() as u16,
            pool: context.pool,
            timestamp: context.timestamp,
            slot: context.slot,
            data,
        })
    }
    
    fn get_hook_account_count(_hook: &HookConfig) -> usize {
        // In practice, each hook would declare how many accounts it needs
        // TODO: For now, assume a fixed number
        4
    }
}

// ============================================================================
// Context Builders
// ============================================================================

pub struct HookContextBuilder;

impl HookContextBuilder {
    pub fn base(
        pool: Pubkey,
        user: Pubkey,
    ) -> HookContext {
        HookContext {
            pool,
            user,
            event: 0,
            stage: 0,
            event_data: EventData::PriceUpdate { 
                old_sqrt_price: 0, 
                new_sqrt_price: 0 
            },
            timestamp: Clock::get().unwrap_or_default().unix_timestamp,
            slot: Clock::get().unwrap_or_default().slot,
            data: BTreeMap::new(),
        }
    }
    
    pub fn price_update(
        pool: Pubkey,
        user: Pubkey,
        old_sqrt_price: u128,
        new_sqrt_price: u128,
    ) -> HookContext {
        HookContext {
            pool,
            user,
            event: EVENT_RATE_UPDATED,
            stage: 0, // Set by caller
            event_data: EventData::PriceUpdate {
                old_sqrt_price,
                new_sqrt_price,
            },
            timestamp: Clock::get().unwrap_or_default().unix_timestamp,
            slot: Clock::get().unwrap_or_default().slot,
            data: BTreeMap::new(),
        }
    }
    
    pub fn liquidity_change(
        pool: Pubkey,
        user: Pubkey,
        position_id: Pubkey,
        liquidity_delta: i128,
        amount_0: u64,
        amount_1: u64,
        tick_lower: i32,
        tick_upper: i32,
    ) -> HookContext {
        HookContext {
            pool,
            user,
            event: EVENT_LIQUIDITY_CHANGED,
            stage: 0,
            event_data: EventData::LiquidityChange {
                position_id,
                liquidity_delta,
                amount_0,
                amount_1,
                tick_lower,
                tick_upper,
            },
            timestamp: Clock::get().unwrap_or_default().unix_timestamp,
            slot: Clock::get().unwrap_or_default().slot,
            data: BTreeMap::new(),
        }
    }
    
    pub fn tick_cross(
        pool: Pubkey,
        user: Pubkey,
        from_tick: i32,
        to_tick: i32,
        liquidity_net: i128,
    ) -> HookContext {
        HookContext {
            pool,
            user,
            event: EVENT_TICK_CROSSED,
            stage: 0,
            event_data: EventData::TickCross {
                from_tick,
                to_tick,
                liquidity_net,
            },
            timestamp: Clock::get().unwrap_or_default().unix_timestamp,
            slot: Clock::get().unwrap_or_default().slot,
            data: BTreeMap::new(),
        }
    }
    
    pub fn swap(
        pool: Pubkey,
        user: Pubkey,
        amount_in: u64,
        amount_out: u64,
        token_in: Pubkey,
        token_out: Pubkey,
        fee_amount: u64,
    ) -> HookContext {
        HookContext {
            pool,
            user,
            event: EVENT_SWAP_EXECUTED,
            stage: 0,
            event_data: EventData::Swap {
                amount_in,
                amount_out,
                token_in,
                token_out,
                fee_amount,
            },
            timestamp: Clock::get().unwrap_or_default().unix_timestamp,
            slot: Clock::get().unwrap_or_default().slot,
            data: BTreeMap::new(),
        }
    }
}

// ============================================================================
// Integration Helpers
// ============================================================================

/// Macro for easy hook execution in instructions
#[macro_export]
macro_rules! execute_hooks {
    ($registry:expr, $queue:expr, $event:expr, $context:expr, $remaining:expr) => {{
        use crate::logic::hook::{HookExecutor, STAGE_VALIDATE, STAGE_PRE_EXECUTE};
        
        // Validation stage
        HookExecutor::execute(
            $registry,
            $event,
            STAGE_VALIDATE,
            &$context,
            None,
            $remaining,
        )?;
        
        // Pre-execution stage
        HookExecutor::execute(
            $registry,
            $event,
            STAGE_PRE_EXECUTE,
            &$context,
            None,
            $remaining,
        )?;
        
        // Note: POST_EXECUTE and ASYNC stages called after operation
    }};
}

#[macro_export]
macro_rules! execute_post_hooks {
    ($registry:expr, $queue:expr, $event:expr, $context:expr, $remaining:expr) => {{
        use crate::logic::hook::{HookExecutor, STAGE_POST_EXECUTE, STAGE_ASYNC};
        
        // Post-execution stage
        HookExecutor::execute(
            $registry,
            $event,
            STAGE_POST_EXECUTE,
            &$context,
            None,
            $remaining,
        )?;
        
        // Async stage (if queue provided)
        HookExecutor::execute(
            $registry,
            $event,
            STAGE_ASYNC,
            &$context,
            $queue,
            $remaining,
        )?;
    }};
}