use anchor_lang::prelude::*;
use crate::state::{FeelsProtocolError, KeeperRegistry, ProtocolState};

// ============================================================================
// Initialize Keeper Registry
// ============================================================================

#[derive(Accounts)]
pub struct InitializeKeeperRegistry<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// Protocol state to ensure proper initialization order
    #[account(
        seeds = [b"protocol"],
        bump,
        has_one = authority,
    )]
    pub protocol: Account<'info, ProtocolState>,
    
    /// Keeper registry to initialize
    #[account(
        init,
        payer = authority,
        space = 8 + KeeperRegistry::SIZE,
        seeds = [b"keeper_registry"],
        bump,
    )]
    pub keeper_registry: AccountLoader<'info, KeeperRegistry>,
    
    pub system_program: Program<'info, System>,
}

pub fn initialize_keeper_registry(ctx: Context<InitializeKeeperRegistry>) -> Result<()> {
    let mut keeper_registry = ctx.accounts.keeper_registry.load_init()?;
    
    keeper_registry.authority = ctx.accounts.authority.key();
    keeper_registry.keeper_count = 0;
    keeper_registry.max_keepers = 32;
    
    // Initialize arrays
    keeper_registry.keepers = [Pubkey::default(); 32];
    keeper_registry.keeper_active = [0; 32];
    keeper_registry.keeper_added_at = [0; 32];
    keeper_registry._reserved = [0; 256];
    
    msg!("Initialized keeper registry with authority: {}", keeper_registry.authority);
    
    Ok(())
}

// ============================================================================
// Add Keeper
// ============================================================================

#[derive(Accounts)]
pub struct AddKeeper<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// Keeper registry
    #[account(
        mut,
        seeds = [b"keeper_registry"],
        bump,
        has_one = authority,
    )]
    pub keeper_registry: AccountLoader<'info, KeeperRegistry>,
    
    pub clock: Sysvar<'info, Clock>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AddKeeperParams {
    /// Keeper pubkey to add
    pub keeper: Pubkey,
}

pub fn add_keeper(ctx: Context<AddKeeper>, params: AddKeeperParams) -> Result<()> {
    let mut keeper_registry = ctx.accounts.keeper_registry.load_mut()?;
    let timestamp = ctx.accounts.clock.unix_timestamp;
    
    keeper_registry.add_keeper(params.keeper, timestamp)?;
    
    msg!("Added keeper {} to registry", params.keeper);
    
    Ok(())
}

// ============================================================================
// Remove Keeper
// ============================================================================

#[derive(Accounts)]
pub struct RemoveKeeper<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// Keeper registry
    #[account(
        mut,
        seeds = [b"keeper_registry"],
        bump,
        has_one = authority,
    )]
    pub keeper_registry: AccountLoader<'info, KeeperRegistry>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RemoveKeeperParams {
    /// Keeper pubkey to remove
    pub keeper: Pubkey,
}

pub fn remove_keeper(ctx: Context<RemoveKeeper>, params: RemoveKeeperParams) -> Result<()> {
    let mut keeper_registry = ctx.accounts.keeper_registry.load_mut()?;
    
    keeper_registry.remove_keeper(&params.keeper)?;
    
    msg!("Removed keeper {} from registry", params.keeper);
    
    Ok(())
}