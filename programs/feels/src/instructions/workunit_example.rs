//! # Example: WorkUnit-Based Instruction Handler
//! 
//! This module demonstrates the proper pattern for using WorkUnit to ensure
//! all state mutations are atomic. This example shows a swap instruction.

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};
use crate::error::FeelsProtocolError;
use crate::state::*;
use crate::logic::{
    unit_of_work::WorkUnit,
    state_context::create_state_context,
    order_manager::{OrderManager, create_order_manager, HubRoute},
};

// ============================================================================
// Instruction Handler - WorkUnit Pattern
// ============================================================================

/// Example swap instruction using WorkUnit pattern
/// 
/// Key points:
/// 1. Create WorkUnit at the start
/// 2. All state access goes through WorkUnit
/// 3. Commit WorkUnit only after all operations succeed
/// 4. No direct account access after WorkUnit creation
pub fn handle_swap_with_workunit<'info>(
    ctx: Context<'_, '_, '_, 'info, SwapAccounts<'info>>,
    amount_in: u64,
    minimum_amount_out: u64,
    zero_for_one: bool,
) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    // ============================================================
    // STEP 1: Create WorkUnit and Load All Required Accounts
    // ============================================================
    msg!("Creating WorkUnit and loading accounts");
    
    let mut work_unit = WorkUnit::new();
    
    // Load all accounts that will be accessed/modified
    work_unit.load_market_field(&ctx.accounts.market_field)?;
    work_unit.load_buffer(&ctx.accounts.buffer_account)?;
    work_unit.load_market_manager(&ctx.accounts.market_manager)?;
    
    // Load oracle if provided
    if let Some(oracle) = ctx.accounts.oracle.as_ref() {
        work_unit.load_twap_oracle(oracle)?;
    }
    
    // Load tick arrays that might be accessed
    for tick_array in &ctx.remaining_accounts {
        if let Ok(loader) = AccountLoader::<TickArray>::try_from(tick_array) {
            work_unit.load_tick_array(&loader)?;
        }
    }
    
    // ============================================================
    // STEP 2: Create StateContext from WorkUnit
    // ============================================================
    msg!("Creating StateContext from WorkUnit");
    
    // Create state context that provides controlled access to WorkUnit state
    let state_context = create_state_context(
        &mut work_unit,
        &ctx.accounts.market_field,
        &ctx.accounts.buffer_account,
        &ctx.accounts.market_manager,
        ctx.accounts.oracle.as_ref(),
    )?;
    
    // ============================================================
    // STEP 3: Create OrderManager with StateContext
    // ============================================================
    msg!("Creating OrderManager");
    
    // OrderManager now only accesses state through StateContext
    let mut order_manager = OrderManager::new(state_context, current_time);
    
    // ============================================================
    // STEP 4: Execute Business Logic
    // ============================================================
    msg!("Executing swap");
    
    // Create route (for single pool swap, one hop)
    let route = HubRoute {
        pools: vec![ctx.accounts.pool.key()],
        zero_for_one: vec![zero_for_one],
    };
    
    // Execute swap - all state mutations happen within WorkUnit
    let result = order_manager.execute_swap(
        route,
        amount_in,
        minimum_amount_out,
        true, // exact_input
    )?;
    
    msg!("Swap completed: {} -> {}", result.amount_primary, result.amount_secondary);
    msg!("Fees: {}, Rebates: {}", result.fee_amount, result.rebate_amount);
    msg!("Work performed: {}", result.work);
    
    // ============================================================
    // STEP 5: Perform Token Transfers (Outside WorkUnit)
    // ============================================================
    msg!("Performing token transfers");
    
    // Transfer tokens in
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account_a.to_account_info(),
                to: ctx.accounts.vault_a.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_in,
    )?;
    
    // Transfer tokens out
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_b.to_account_info(),
                to: ctx.accounts.user_token_account_b.to_account_info(),
                authority: ctx.accounts.protocol_authority.to_account_info(),
            },
            &[&[b"protocol", &[ctx.bumps.protocol_authority]]],
        ),
        result.amount_secondary,
    )?;
    
    // ============================================================
    // STEP 6: Commit WorkUnit - Atomic Write of All Changes
    // ============================================================
    msg!("Committing WorkUnit changes");
    
    // This is the ONLY place where state is written back to accounts
    // If this fails, all changes are rolled back automatically
    work_unit.commit()?;
    
    // ============================================================
    // STEP 7: Emit Events (After Successful Commit)
    // ============================================================
    
    emit!(SwapEvent {
        user: ctx.accounts.user.key(),
        pool: ctx.accounts.pool.key(),
        amount_in,
        amount_out: result.amount_secondary,
        fee: result.fee_amount,
        rebate: result.rebate_amount,
        work: result.work,
        price: result.final_price,
        timestamp: current_time,
    });
    
    Ok(())
}

// ============================================================================
// Alternative Pattern: Using WorkUnit in Operation Trait
// ============================================================================

/// Example of integrating WorkUnit into the Operation trait pattern
pub struct SwapWithWorkUnitOp {
    pub amount_in: u64,
    pub minimum_amount_out: u64,
    pub zero_for_one: bool,
}

impl Operation for SwapWithWorkUnitOp {
    type Context<'info> = SwapAccounts<'info>;
    type Result = ();
    
    fn validate(&self) -> Result<()> {
        require!(self.amount_in > 0, FeelsProtocolError::InvalidAmount);
        Ok(())
    }
    
    fn execute<'info>(
        &self,
        ctx: Context<'_, '_, '_, 'info, Self::Context<'info>>,
    ) -> Result<Self::Result> {
        // Use the WorkUnit pattern within the operation
        handle_swap_with_workunit(ctx, self.amount_in, self.minimum_amount_out, self.zero_for_one)
    }
}

// ============================================================================
// Account Structures
// ============================================================================

#[derive(Accounts)]
pub struct SwapAccounts<'info> {
    /// User performing the swap
    #[account(mut)]
    pub user: Signer<'info>,
    
    /// User's token accounts
    #[account(mut)]
    pub user_token_account_a: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_account_b: Account<'info, TokenAccount>,
    
    /// Protocol vaults
    #[account(mut)]
    pub vault_a: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vault_b: Account<'info, TokenAccount>,
    
    /// Market accounts
    #[account(mut)]
    pub market_field: Account<'info, MarketField>,
    #[account(mut)]
    pub market_manager: AccountLoader<'info, MarketManager>,
    #[account(mut)]
    pub buffer_account: Account<'info, BufferAccount>,
    
    /// Optional oracle
    pub oracle: Option<AccountLoader<'info, UnifiedOracle>>,
    
    /// Pool reference
    /// CHECK: Used for validation
    pub pool: UncheckedAccount<'info>,
    
    /// Protocol authority (PDA)
    /// CHECK: Seeds validated in handler
    #[account(
        seeds = [b"protocol"],
        bump,
    )]
    pub protocol_authority: UncheckedAccount<'info>,
    
    /// Programs
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct SwapEvent {
    pub user: Pubkey,
    pub pool: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee: u64,
    pub rebate: u64,
    pub work: i128,
    pub price: u128,
    pub timestamp: i64,
}

// ============================================================================
// Best Practices Summary
// ============================================================================

/*
WorkUnit Pattern Best Practices:

1. **Single WorkUnit Per Instruction**
   - Create one WorkUnit at the start of the handler
   - Load all required accounts into it
   - Create StateContext from the WorkUnit

2. **No Direct Account Access After WorkUnit Creation**
   - Once accounts are loaded into WorkUnit, access them only through StateContext
   - This ensures all mutations are tracked

3. **Atomic Commits**
   - Call work_unit.commit() only after all operations succeed
   - This provides automatic rollback on failure

4. **Clear Phases**
   - Phase 1: Create WorkUnit and load accounts
   - Phase 2: Create StateContext
   - Phase 3: Execute business logic
   - Phase 4: Perform external operations (transfers)
   - Phase 5: Commit WorkUnit
   - Phase 6: Emit events

5. **Error Handling**
   - If any operation fails before commit, WorkUnit is dropped
   - Drop handler warns about uncommitted changes
   - No partial state updates possible

6. **Testing**
   - WorkUnit makes testing easier - can verify state changes before commit
   - Can test rollback behavior by not calling commit

7. **Performance**
   - Single write at the end is more efficient than multiple writes
   - Reduces chances of partial updates
*/