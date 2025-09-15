use anchor_lang::prelude::*;

use crate::{
    error::FeelsError,
    state::{Market, TranchePlan},
};

#[derive(Accounts)]
pub struct CleanupBondingCurve<'info> {
    #[account(mut, constraint = authority.key() == market.authority @ FeelsError::UnauthorizedSigner)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [crate::state::tranche_plan::TranchePlan::SEED, market.key().as_ref()],
        bump,
        close = authority,
        constraint = tranche_plan.market == market.key() @ FeelsError::InvalidAccount,
    )]
    pub tranche_plan: Account<'info, TranchePlan>,
}

pub fn cleanup_bonding_curve(ctx: Context<CleanupBondingCurve>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    market.cleanup_complete = true;
    Ok(())
}
