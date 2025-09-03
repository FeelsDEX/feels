use anchor_lang::prelude::*;

/// Volume tracking for lending and borrowing activities
#[account(zero_copy)]
#[repr(C, packed)]
pub struct VolumeTracker {
    /// Pool this tracker serves
    pub pool: Pubkey,
    
    /// Current epoch
    pub current_epoch: u64,
    
    /// Lending volume in current epoch (token amounts)
    pub lending_volume_0: u128,
    pub lending_volume_1: u128,
    
    /// Borrowing volume in current epoch (token amounts)
    pub borrowing_volume_0: u128,
    pub borrowing_volume_1: u128,
    
    /// Total lending volume (all-time)
    pub total_lending_volume_0: u128,
    pub total_lending_volume_1: u128,
    
    /// Total borrowing volume (all-time)
    pub total_borrowing_volume_0: u128,
    pub total_borrowing_volume_1: u128,
    
    /// Last update timestamp
    pub last_update: i64,
    
    /// Moving averages (24h)
    pub lending_avg_24h_0: u64,
    pub lending_avg_24h_1: u64,
    pub borrowing_avg_24h_0: u64,
    pub borrowing_avg_24h_1: u64,
    
    /// Reserved for future use
    pub _reserved: [u8; 128],
}

impl VolumeTracker {
    pub const SIZE: usize = 32 + 8 + (16 * 8) + 8 + (8 * 4) + 128;
    
    /// Record lending activity
    pub fn record_lending(&mut self, token_0_amount: u64, token_1_amount: u64) {
        self.lending_volume_0 = self.lending_volume_0.saturating_add(token_0_amount as u128);
        self.lending_volume_1 = self.lending_volume_1.saturating_add(token_1_amount as u128);
        
        self.total_lending_volume_0 = self.total_lending_volume_0.saturating_add(token_0_amount as u128);
        self.total_lending_volume_1 = self.total_lending_volume_1.saturating_add(token_1_amount as u128);
    }
    
    /// Record borrowing activity
    pub fn record_borrowing(&mut self, token_0_amount: u64, token_1_amount: u64) {
        self.borrowing_volume_0 = self.borrowing_volume_0.saturating_add(token_0_amount as u128);
        self.borrowing_volume_1 = self.borrowing_volume_1.saturating_add(token_1_amount as u128);
        
        self.total_borrowing_volume_0 = self.total_borrowing_volume_0.saturating_add(token_0_amount as u128);
        self.total_borrowing_volume_1 = self.total_borrowing_volume_1.saturating_add(token_1_amount as u128);
    }
    
    /// Update moving averages
    pub fn update_averages(&mut self, current_time: i64) {
        let time_delta = current_time.saturating_sub(self.last_update);
        if time_delta >= 86400 { // 24 hours
            // Simple moving average update
            self.lending_avg_24h_0 = (self.lending_volume_0 / 24).min(u64::MAX as u128) as u64;
            self.lending_avg_24h_1 = (self.lending_volume_1 / 24).min(u64::MAX as u128) as u64;
            self.borrowing_avg_24h_0 = (self.borrowing_volume_0 / 24).min(u64::MAX as u128) as u64;
            self.borrowing_avg_24h_1 = (self.borrowing_volume_1 / 24).min(u64::MAX as u128) as u64;
        }
        self.last_update = current_time;
    }
    
    /// Get utilization rate (borrowing / lending)
    pub fn get_utilization_rate(&self) -> (u64, u64) {
        let util_0 = if self.total_lending_volume_0 > 0 {
            ((self.total_borrowing_volume_0 * 10000) / self.total_lending_volume_0)
                .min(10000)
                .min(u64::MAX as u128) as u64
        } else {
            0
        };
        
        let util_1 = if self.total_lending_volume_1 > 0 {
            ((self.total_borrowing_volume_1 * 10000) / self.total_lending_volume_1)
                .min(10000)
                .min(u64::MAX as u128) as u64
        } else {
            0
        };
        
        (util_0, util_1)
    }
}

// Initialize volume tracker instruction
#[derive(Accounts)]
pub struct InitializeVolumeTracker<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// Market field
    pub market_field: Account<'info, crate::state::MarketField>,
    
    /// Volume tracker to initialize
    #[account(
        init,
        payer = authority,
        space = 8 + VolumeTracker::SIZE,
        seeds = [b"volume_tracker", market_field.pool.as_ref()],
        bump,
    )]
    pub volume_tracker: AccountLoader<'info, VolumeTracker>,
    
    pub system_program: Program<'info, System>,
}

pub fn initialize_volume_tracker(ctx: Context<InitializeVolumeTracker>) -> Result<()> {
    let mut tracker = ctx.accounts.volume_tracker.load_init()?;
    
    tracker.pool = ctx.accounts.market_field.pool;
    tracker.current_epoch = 0;
    tracker.lending_volume_0 = 0;
    tracker.lending_volume_1 = 0;
    tracker.borrowing_volume_0 = 0;
    tracker.borrowing_volume_1 = 0;
    tracker.total_lending_volume_0 = 0;
    tracker.total_lending_volume_1 = 0;
    tracker.total_borrowing_volume_0 = 0;
    tracker.total_borrowing_volume_1 = 0;
    tracker.last_update = Clock::get()?.unix_timestamp;
    tracker.lending_avg_24h_0 = 0;
    tracker.lending_avg_24h_1 = 0;
    tracker.borrowing_avg_24h_0 = 0;
    tracker.borrowing_avg_24h_1 = 0;
    tracker._reserved = [0; 128];
    
    msg!("Initialized volume tracker for pool {}", tracker.pool);
    
    Ok(())
}