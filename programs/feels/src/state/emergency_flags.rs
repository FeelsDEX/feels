use anchor_lang::prelude::*;

/// Emergency operation flags for protocol safety
#[account(zero_copy)]
#[repr(C)]
pub struct EmergencyFlags {
    /// Market this applies to
    pub market: Pubkey,
    
    /// Authority that can modify flags
    pub emergency_authority: Pubkey,
    
    /// Pause all swaps (0 = false, 1 = true)
    pub pause_swaps: u8,
    
    /// Pause liquidity operations (0 = false, 1 = true)
    pub pause_liquidity: u8,
    
    /// Pause leverage operations (0 = false, 1 = true)
    pub pause_leverage: u8,
    
    /// Pause rebates (0 = false, 1 = true)
    pub pause_rebates: u8,
    
    /// Force maximum fees (0 = false, 1 = true)
    pub force_max_fees: u8,
    
    /// Emergency mode active (0 = false, 1 = true)
    pub emergency_mode: u8,
    
    /// Padding to align to 8-byte boundary
    pub _padding: [u8; 2],
    
    /// Time emergency was activated
    pub emergency_activated_at: i64,
    
    /// Reason for emergency (max 64 chars)
    pub emergency_reason: [u8; 64],
    
    /// Reserved flags for future use
    pub _reserved_flags: [u8; 8],
    
    /// Reserved space
    pub _reserved: [u8; 128],
}

impl EmergencyFlags {
    pub const SIZE: usize = 32 + 32 + 1 + 1 + 1 + 1 + 1 + 1 + 2 + 8 + 64 + 8 + 128;
    
    /// Check if any operations are paused
    pub fn is_operational(&self) -> bool {
        self.emergency_mode == 0 && self.pause_swaps == 0
    }
    
    /// Activate emergency mode
    pub fn activate_emergency(&mut self, reason: &str, timestamp: i64) {
        self.emergency_mode = 1;
        self.emergency_activated_at = timestamp;
        
        // Copy reason (truncate if needed)
        let reason_bytes = reason.as_bytes();
        let len = reason_bytes.len().min(64);
        self.emergency_reason[..len].copy_from_slice(&reason_bytes[..len]);
        
        // Set all pause flags
        self.pause_swaps = 1;
        self.pause_liquidity = 1;
        self.pause_leverage = 1;
        self.pause_rebates = 1;
        self.force_max_fees = 1;
        
        msg!("Emergency mode activated: {}", reason);
    }
    
    /// Deactivate emergency mode
    pub fn deactivate_emergency(&mut self) {
        self.emergency_mode = 0;
        self.pause_swaps = 0;
        self.pause_liquidity = 0;
        self.pause_leverage = 0;
        self.pause_rebates = 0;
        self.force_max_fees = 0;
        self.emergency_reason = [0; 64];
        
        msg!("Emergency mode deactivated");
    }
}

// Initialize emergency flags instruction
#[derive(Accounts)]
pub struct InitializeEmergencyFlags<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// Protocol state
    #[account(
        seeds = [b"protocol"],
        bump,
        has_one = authority,
    )]
    pub protocol: Account<'info, crate::state::ProtocolState>,
    
    /// Market field
    pub market_field: Account<'info, crate::state::MarketField>,
    
    /// Emergency flags to initialize
    #[account(
        init,
        payer = authority,
        space = 8 + EmergencyFlags::SIZE,
        seeds = [b"emergency_flags", market_field.pool.as_ref()],
        bump,
    )]
    pub emergency_flags: AccountLoader<'info, EmergencyFlags>,
    
    pub system_program: Program<'info, System>,
}

pub fn initialize_emergency_flags(
    ctx: Context<InitializeEmergencyFlags>,
    emergency_authority: Pubkey,
) -> Result<()> {
    let mut flags = ctx.accounts.emergency_flags.load_init()?;
    
    flags.market = ctx.accounts.market_field.pool;
    flags.emergency_authority = emergency_authority;
    flags.pause_swaps = 0;
    flags.pause_liquidity = 0;
    flags.pause_leverage = 0;
    flags.pause_rebates = 0;
    flags.force_max_fees = 0;
    flags.emergency_mode = 0;
    flags._padding = [0; 2];
    flags.emergency_activated_at = 0;
    flags.emergency_reason = [0; 64];
    flags._reserved_flags = [0; 8];
    flags._reserved = [0; 128];
    
    msg!("Initialized emergency flags for market {}", flags.market);
    
    Ok(())
}

// Toggle emergency mode instruction
#[derive(Accounts)]
pub struct ToggleEmergencyMode<'info> {
    /// Emergency authority
    #[account(
        constraint = authority.key() == emergency_flags.load()?.emergency_authority
            @ crate::error::FeelsProtocolError::Unauthorized
    )]
    pub authority: Signer<'info>,
    
    /// Emergency flags
    #[account(mut)]
    pub emergency_flags: AccountLoader<'info, EmergencyFlags>,
    
    pub clock: Sysvar<'info, Clock>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct EmergencyModeParams {
    pub activate: bool,
    pub reason: String,
}

pub fn toggle_emergency_mode(
    ctx: Context<ToggleEmergencyMode>,
    params: EmergencyModeParams,
) -> Result<()> {
    let mut flags = ctx.accounts.emergency_flags.load_mut()?;
    let timestamp = ctx.accounts.clock.unix_timestamp;
    
    if params.activate {
        flags.activate_emergency(&params.reason, timestamp);
    } else {
        flags.deactivate_emergency();
    }
    
    Ok(())
}