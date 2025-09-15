use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct TrancheEntry {
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
}

#[account]
pub struct TranchePlan {
    pub market: Pubkey,
    pub applied: bool,
    pub count: u8,
    pub entries: Vec<TrancheEntry>,
}

impl TranchePlan {
    pub const SEED: &'static [u8] = b"tranche_plan";

    pub fn space_for(n: usize) -> usize {
        8 + // disc
        32 + // market
        1 + // applied
        1 + // count
        4 + // vec len
        n * (4 + 4 + 16) // entries
    }
}

