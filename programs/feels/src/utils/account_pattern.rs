/// Account pattern abstractions for the Feels Protocol
/// 
/// This module provides reusable account patterns that prevent repetition
/// across instruction contexts while maintaining type safety and validation logic.
/// 
/// Common patterns include:
/// - Pool with token vaults validation  
/// - User token account pairs
/// - Tick array pairs for liquidity operations
/// - Authority validation contexts
/// - Program account collections
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{TokenAccount, TokenInterface},
    token_2022::Token2022,
};

use crate::state::{
    FeelsProtocolError, Pool, TickArray, TickPositionMetadata, 
    Oracle, OracleData, FeelsSOL
};

// ============================================================================
// Core Pool Patterns
// ============================================================================

/// Pool with token vaults - the most common pattern in liquidity operations
#[derive(Accounts)]
pub struct PoolWithVaults<'info> {
    /// The liquidity pool
    #[account(
        seeds = [
            b"pool",
            pool.load()?.token_a_mint.as_ref(),
            pool.load()?.token_b_mint.as_ref(),
            &pool.load()?.fee_rate.to_le_bytes()
        ],
        bump
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// Pool's token A vault
    #[account(
        mut,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            pool.load()?.token_a_mint.as_ref()
        ],
        bump,
        token::mint = pool.load()?.token_a_mint,
        token::authority = pool,
        token::token_program = token_program,
    )]
    pub token_vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Pool's token B vault
    #[account(
        mut,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            pool.load()?.token_b_mint.as_ref()
        ],
        bump,
        token::mint = pool.load()?.token_b_mint,
        token::authority = pool,
        token::token_program = token_program,
    )]
    pub token_vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token program for interface compatibility
    pub token_program: Interface<'info, TokenInterface>,
}

/// User token account pair - common in swaps and liquidity operations
#[derive(Accounts)]
pub struct UserTokenAccounts<'info> {
    /// User's token A account
    #[account(
        mut,
        token::token_program = token_program,
    )]
    pub user_token_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// User's token B account  
    #[account(
        mut,
        token::token_program = token_program,
    )]
    pub user_token_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token program for interface compatibility
    pub token_program: Interface<'info, TokenInterface>,
}

/// Tick array pair for liquidity operations
#[derive(Accounts)]
pub struct TickArrayPair<'info> {
    /// Lower tick array
    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_lower.load()?.start_tick_index.to_le_bytes()
        ],
        bump,
        constraint = tick_array_lower.load()?.pool == pool.key() @ FeelsProtocolError::InvalidTickArray
    )]
    pub tick_array_lower: AccountLoader<'info, TickArray>,

    /// Upper tick array
    #[account(
        mut,
        seeds = [
            b"tick_array", 
            pool.key().as_ref(),
            &tick_array_upper.load()?.start_tick_index.to_le_bytes()
        ],
        bump,
        constraint = tick_array_upper.load()?.pool == pool.key() @ FeelsProtocolError::InvalidTickArray
    )]
    pub tick_array_upper: AccountLoader<'info, TickArray>,

    /// Pool reference for validation
    pub pool: AccountLoader<'info, Pool>,
}

// ============================================================================
// Composite Patterns 
// ============================================================================

/// Complete liquidity operation context
#[derive(Accounts)]
pub struct LiquidityOperationContext<'info> {
    /// Pool with token vaults - flattened
    pub pool: AccountLoader<'info, Pool>,
    pub token_vault_a: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// User token accounts - flattened  
    pub user_token_a: Box<InterfaceAccount<'info, TokenAccount>>,
    pub user_token_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Tick arrays - flattened
    pub tick_array_lower: AccountLoader<'info, TickArray>,
    pub tick_array_upper: AccountLoader<'info, TickArray>,

    /// User/payer authority
    pub user: Signer<'info>,

    /// Token program
    pub token_program: Interface<'info, TokenInterface>,

    /// System program for account creation
    pub system_program: Program<'info, System>,
}

/// Swap operation context
#[derive(Accounts)]
pub struct SwapContext<'info> {
    /// Pool account
    pub pool: AccountLoader<'info, Pool>,
    
    /// Pool's token vaults - flattened
    pub token_vault_a: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// User as authority for token accounts
    pub user: Signer<'info>,
    
    /// User's input token account
    #[account(
        mut,
        token::authority = user,
        token::token_program = token_program,
    )]
    pub user_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

    /// User's output token account
    #[account(
        mut,
        token::authority = user,
        token::token_program = token_program,
    )]
    pub user_token_out: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token program
    pub token_program: Interface<'info, TokenInterface>,
}

// ============================================================================
// Authority Patterns
// ============================================================================

/// Pool authority validation
#[derive(Accounts)]
pub struct PoolAuthorityContext<'info> {
    /// Pool being managed
    pub pool: AccountLoader<'info, Pool>,
    
    /// Authority that must match pool authority
    #[account(
        constraint = authority.key() == pool.load()?.authority @ FeelsProtocolError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

/// Protocol authority validation
#[derive(Accounts)]  
pub struct ProtocolAuthorityContext<'info> {
    /// Protocol state account
    #[account(
        seeds = [b"protocol"],
        bump,
    )]
    pub protocol: Account<'info, crate::state::protocol::ProtocolState>,
    
    /// Authority that must match protocol authority
    #[account(
        constraint = authority.key() == protocol.authority @ FeelsProtocolError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

// ============================================================================
// Position Management Patterns  
// ============================================================================

/// Position metadata with validation
#[derive(Accounts)]
pub struct ValidatedPosition<'info> {
    /// Position metadata account
    #[account(
        mut,
        constraint = position.pool == pool.key() @ FeelsProtocolError::InvalidPool,
        constraint = position.owner == owner.key() @ FeelsProtocolError::Unauthorized,
    )]
    pub position: Account<'info, TickPositionMetadata>,
    
    /// Pool the position belongs to
    pub pool: AccountLoader<'info, Pool>,
    
    /// Position owner
    pub owner: Signer<'info>,
}

// ============================================================================
// Phase 2 Extension Patterns
// ============================================================================

/// Oracle accounts for price data
#[derive(Accounts)]
pub struct OracleContext<'info> {
    /// Oracle metadata account
    #[account(
        constraint = oracle.pool == pool.key() @ FeelsProtocolError::InvalidPool
    )]
    pub oracle: Box<Account<'info, Oracle>>,

    /// Oracle data storage account
    #[account(
        // Oracle data is associated with the oracle via seeds/PDA derivation
    )]
    pub oracle_data: AccountLoader<'info, OracleData>,

    /// Pool reference for validation
    pub pool: AccountLoader<'info, Pool>,
}

/// FeelsSOL validation pattern
#[derive(Accounts)]
pub struct FeelsSOLContext<'info> {
    /// FeelsSOL wrapper account
    #[account(
        seeds = [
            b"feelssol", 
            feelssol.feels_mint.as_ref(),
            feelssol.underlying_mint.as_ref()
        ],
        bump,
    )]
    pub feelssol: Account<'info, FeelsSOL>,
}

// ============================================================================
// Standard Program Collections
// ============================================================================

/// Basic program accounts needed for most instructions
#[derive(Accounts)]
pub struct BasicPrograms<'info> {
    /// Token program for SPL operations
    pub token_program: Interface<'info, TokenInterface>,
    
    /// System program for account creation
    pub system_program: Program<'info, System>,
}

/// Extended program accounts for complex operations
#[derive(Accounts)]
pub struct ExtendedPrograms<'info> {
    /// Token program for SPL operations
    pub token_program: Interface<'info, TokenInterface>,
    
    /// Associated token program for ATA operations
    pub associated_token_program: Program<'info, AssociatedToken>,
    
    /// System program for account creation
    pub system_program: Program<'info, System>,
    
    /// Rent sysvar for rent calculations
    pub rent: Sysvar<'info, Rent>,
}

// ============================================================================
// Enhanced Validation Patterns
// ============================================================================

/// Validated tick array pair with pool ownership constraints
/// This pattern encapsulates the common validation that tick arrays belong to a pool
/// and have valid start_tick_index values
#[derive(Accounts)]
pub struct ValidatedTickArrayPair<'info> {
    /// Lower tick array with full validation
    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_lower.load()?.start_tick_index.to_le_bytes()
        ],
        bump,
        constraint = tick_array_lower.load()?.pool == pool.key() @ FeelsProtocolError::InvalidTickArray,
        constraint = tick_array_lower.load()?.start_tick_index < tick_array_upper.load()?.start_tick_index @ FeelsProtocolError::InvalidTickRange
    )]
    pub tick_array_lower: AccountLoader<'info, TickArray>,

    /// Upper tick array with full validation
    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_upper.load()?.start_tick_index.to_le_bytes()
        ],
        bump,
        constraint = tick_array_upper.load()?.pool == pool.key() @ FeelsProtocolError::InvalidTickArray
    )]
    pub tick_array_upper: AccountLoader<'info, TickArray>,

    /// Pool reference for validation (must be provided)
    pub pool: AccountLoader<'info, Pool>,
}

/// User-owned position with complete validation
/// This pattern validates that a position belongs to both the correct pool and user
#[derive(Accounts)]
pub struct UserOwnedPosition<'info> {
    /// Position with pool and owner validation
    #[account(
        mut,
        constraint = position.pool == pool.key() @ FeelsProtocolError::InvalidPool,
        constraint = position.owner == user.key() @ FeelsProtocolError::Unauthorized,
        constraint = position.tick_lower < position.tick_upper @ FeelsProtocolError::InvalidTickRange
    )]
    pub position: Account<'info, TickPositionMetadata>,
    
    /// Pool reference for validation
    pub pool: AccountLoader<'info, Pool>,
    
    /// User/owner signer for validation
    pub user: Signer<'info>,
}

/// Hook execution context for pool-level hooks
/// This pattern provides the standard hook accounts needed for most operations
#[derive(Accounts)]
pub struct HookExecutionContext<'info> {
    /// Hook registry for this pool (optional)
    #[account(
        seeds = [b"hook_registry", pool.key().as_ref()],
        bump
    )]
    pub hook_registry: Option<Account<'info, crate::state::HookRegistry>>,
    
    /// Hook message queue for async processing (optional)
    #[account(
        mut,
        seeds = [b"hook_messages", pool.key().as_ref()],
        bump,
    )]
    pub hook_message_queue: Option<Account<'info, crate::state::HookMessageQueue>>,
    
    /// Pool reference for validation
    pub pool: AccountLoader<'info, Pool>,
}

/// User token pair with authority validation
/// Enhanced version of UserTokenAccounts with explicit authority constraints
#[derive(Accounts)]
pub struct UserTokenPair<'info> {
    /// User's token A account with authority validation
    #[account(
        mut,
        token::authority = user,
        token::token_program = token_program,
    )]
    pub user_token_a: InterfaceAccount<'info, TokenAccount>,
    
    /// User's token B account with authority validation
    #[account(
        mut,
        token::authority = user,
        token::token_program = token_program,
    )]
    pub user_token_b: InterfaceAccount<'info, TokenAccount>,
    
    /// User signer for validation
    pub user: Signer<'info>,
    
    /// Token program
    pub token_program: Program<'info, Token2022>,
}

/// Pool with validated token vaults - enhanced version with stricter constraints
/// This pattern includes the pool PDA validation and vault authority checks
#[derive(Accounts)]
pub struct PoolWithValidatedVaults<'info> {
    /// Pool with full PDA validation
    #[account(
        mut,
        seeds = [
            b"pool",
            pool.load()?.token_a_mint.as_ref(),
            pool.load()?.token_b_mint.as_ref(),
            &pool.load()?.fee_rate.to_le_bytes()
        ],
        bump
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// Token A vault with full validation
    #[account(
        mut,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            pool.load()?.token_a_mint.as_ref()
        ],
        bump,
        token::mint = pool.load()?.token_a_mint,
        token::authority = pool,
        token::token_program = token_program,
        constraint = token_vault_a.key() == pool.load()?.token_a_vault @ FeelsProtocolError::InvalidPool
    )]
    pub token_vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token B vault with full validation
    #[account(
        mut,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            pool.load()?.token_b_mint.as_ref()
        ],
        bump,
        token::mint = pool.load()?.token_b_mint,
        token::authority = pool,
        token::token_program = token_program,
        constraint = token_vault_b.key() == pool.load()?.token_b_vault @ FeelsProtocolError::InvalidPool
    )]
    pub token_vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token program
    pub token_program: Program<'info, Token2022>,
}

/// Complete liquidity operation context with all validation
/// This is the ultimate composite pattern that combines all common validations
/// needed for liquidity operations (add/remove)
#[derive(Accounts)]
pub struct CompleteLiquidityContext<'info> {
    // Pool with validated vaults (flattened for direct access)
    #[account(
        mut,
        seeds = [
            b"pool",
            pool.load()?.token_a_mint.as_ref(),
            pool.load()?.token_b_mint.as_ref(),
            &pool.load()?.fee_rate.to_le_bytes()
        ],
        bump
    )]
    pub pool: AccountLoader<'info, Pool>,
    
    #[account(
        mut,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            pool.load()?.token_a_mint.as_ref()
        ],
        bump,
        token::mint = pool.load()?.token_a_mint,
        token::authority = pool,
        token::token_program = token_program,
    )]
    pub token_vault_a: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(
        mut,
        seeds = [
            b"vault",
            pool.key().as_ref(),
            pool.load()?.token_b_mint.as_ref()
        ],
        bump,
        token::mint = pool.load()?.token_b_mint,
        token::authority = pool,
        token::token_program = token_program,
    )]
    pub token_vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    // User token accounts with authority validation
    #[account(
        mut,
        token::authority = user,
        token::token_program = token_program,
    )]
    pub user_token_a: InterfaceAccount<'info, TokenAccount>,
    
    #[account(
        mut,
        token::authority = user,
        token::token_program = token_program,
    )]
    pub user_token_b: InterfaceAccount<'info, TokenAccount>,

    // Validated tick arrays
    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_lower.load()?.start_tick_index.to_le_bytes()
        ],
        bump,
        constraint = tick_array_lower.load()?.pool == pool.key() @ FeelsProtocolError::InvalidTickArray
    )]
    pub tick_array_lower: AccountLoader<'info, TickArray>,

    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_upper.load()?.start_tick_index.to_le_bytes()
        ],
        bump,
        constraint = tick_array_upper.load()?.pool == pool.key() @ FeelsProtocolError::InvalidTickArray
    )]
    pub tick_array_upper: AccountLoader<'info, TickArray>,

    // Position with full validation
    #[account(
        mut,
        constraint = tick_position_metadata.pool == pool.key() @ FeelsProtocolError::InvalidPool,
        constraint = tick_position_metadata.owner == user.key() @ FeelsProtocolError::Unauthorized,
    )]
    pub tick_position_metadata: Account<'info, TickPositionMetadata>,

    // Hook context (optional)
    #[account(
        seeds = [b"hook_registry", pool.key().as_ref()],
        bump,
    )]
    pub hook_registry: Option<Account<'info, crate::state::HookRegistry>>,
    
    #[account(
        mut,
        seeds = [b"hook_messages", pool.key().as_ref()],
        bump,
    )]
    pub hook_message_queue: Option<Account<'info, crate::state::HookMessageQueue>>,

    // Signers and programs
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(mut)]
    pub payer: Signer<'info>,
    
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

impl<'info> CompleteLiquidityContext<'info> {
    /// Validate that tick arrays have proper ordering
    pub fn validate_tick_array_ordering(&self) -> Result<()> {
        let lower_array = self.tick_array_lower.load()?;
        let upper_array = self.tick_array_upper.load()?;
        
        require!(
            lower_array.start_tick_index < upper_array.start_tick_index,
            FeelsProtocolError::InvalidTickRange
        );
        
        Ok(())
    }
    
    /// Validate that position tick range matches tick arrays
    pub fn validate_position_tick_range(&self) -> Result<()> {
        let lower_array = self.tick_array_lower.load()?;
        let upper_array = self.tick_array_upper.load()?;
        
        // Validate that position ticks fall within the provided tick arrays
        require!(
            self.tick_position_metadata.tick_lower >= lower_array.start_tick_index &&
            self.tick_position_metadata.tick_lower < lower_array.start_tick_index + crate::constant::TICK_ARRAY_SIZE as i32,
            FeelsProtocolError::InvalidTickArray
        );
        
        require!(
            self.tick_position_metadata.tick_upper >= upper_array.start_tick_index &&
            self.tick_position_metadata.tick_upper < upper_array.start_tick_index + crate::constant::TICK_ARRAY_SIZE as i32,
            FeelsProtocolError::InvalidTickArray
        );
        
        Ok(())
    }
}


