//! Permissionless crank to initialize TickArrays and tranche boundary ticks
//! for bonding-curve stair deployment. Does not move funds; only sets up
//! TickArray headers and marks lower/upper ticks as initialized with correct
//! fee_growth_outside snapshots. Safe to call repeatedly.

use anchor_lang::accounts::account_loader::AccountLoader;
use anchor_lang::prelude::*;

use crate::{
    error::FeelsError,
    state::{Market, TickArray, TranchePlan},
    utils::get_tick_array_start_index,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeTrancheTicksParams {
    pub tick_step_size: i32,
    pub num_steps: u8,
}

#[derive(Accounts)]
#[instruction(params: InitializeTrancheTicksParams)]
pub struct InitializeTrancheTicks<'info> {
    /// Anyone can crank
    pub crank: Signer<'info>,

    /// Market whose tranche ticks to initialize
    #[account(mut, constraint = market.is_initialized @ FeelsError::MarketNotInitialized)]
    pub market: Account<'info, Market>,

    /// Tranche plan produced at deploy time
    #[account(
        mut,
        seeds = [crate::state::tranche_plan::TranchePlan::SEED, market.key().as_ref()],
        bump,
        constraint = tranche_plan.market == market.key() @ FeelsError::InvalidAccount,
    )]
    pub tranche_plan: Account<'info, TranchePlan>,

    /// System program for creating missing TickArrays
    pub system_program: Program<'info, System>,
    // Remaining accounts: TickArray PDAs expected for all lower/upper tranche boundaries
}

pub fn initialize_tranche_ticks<'info>(
    ctx: Context<'_, '_, 'info, 'info, InitializeTrancheTicks<'info>>,
    params: InitializeTrancheTicksParams,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let tick_spacing = market.tick_spacing;

    require!(params.tick_step_size > 0, FeelsError::TickNotSpaced);
    require!(
        params.tick_step_size % tick_spacing as i32 == 0,
        FeelsError::TickNotSpaced
    );

    let market_key = market.key();
    let arrays = ctx.remaining_accounts;

    // Helper: ensure a TickArray exists for the given start index and return a loader
    let ensure_array = |start_index: i32| -> Result<AccountLoader<'info, TickArray>> {
        let (expected_key, bump) =
            crate::utils::derive_tick_array(&market_key, start_index, &crate::id());
        let info = arrays
            .iter()
            .find(|ai| ai.key() == expected_key)
            .ok_or(FeelsError::TickArrayNotFound)?;

        if info.data_is_empty() {
            let lamports = Rent::get()?.minimum_balance(TickArray::LEN);
            let ix = anchor_lang::solana_program::system_instruction::create_account(
                &ctx.accounts.crank.key(),
                &expected_key,
                lamports,
                TickArray::LEN as u64,
                &crate::id(),
            );

            // Create signer seeds for the PDA
            let start_tick_bytes = start_index.to_le_bytes();
            let seeds = &[
                crate::constants::TICK_ARRAY_SEED,
                market_key.as_ref(),
                &start_tick_bytes,
                &[bump],
            ];
            let signer_seeds = &[&seeds[..]];

            anchor_lang::solana_program::program::invoke_signed(
                &ix,
                &[
                    ctx.accounts.crank.to_account_info(),
                    info.clone(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                signer_seeds,
            )?;
            let loader = AccountLoader::<TickArray>::try_from(info)?;
            let mut array = loader.load_init()?;
            array.market = market_key;
            array.start_tick_index = start_index;
            array.initialized_tick_count = 0;
            return AccountLoader::<TickArray>::try_from(info);
        }
        AccountLoader::<TickArray>::try_from(info)
    };

    // Iterate tranche entries; optionally apply liquidity nets if not yet applied
    let apply_liq = !ctx.accounts.tranche_plan.applied;
    let total = core::cmp::min(
        params.num_steps as usize,
        ctx.accounts.tranche_plan.entries.len(),
    );
    for i in 0..total {
        let e = ctx.accounts.tranche_plan.entries[i];
        let tick_lower = e.tick_lower;
        let tick_upper = e.tick_upper;

        // Lower array
        let lower_start = get_tick_array_start_index(tick_lower, tick_spacing);
        let lower_loader = ensure_array(lower_start)?;
        {
            let mut arr = lower_loader.load_mut()?;
            arr.init_tick(
                tick_lower,
                tick_spacing,
                market.current_tick,
                market.fee_growth_global_0_x64,
                market.fee_growth_global_1_x64,
            )?;
        }
        // Upper array
        let upper_start = get_tick_array_start_index(tick_upper, tick_spacing);
        let same = upper_start == lower_start;
        if same {
            let mut arr = lower_loader.load_mut()?;
            arr.init_tick(
                tick_upper,
                tick_spacing,
                market.current_tick,
                market.fee_growth_global_0_x64,
                market.fee_growth_global_1_x64,
            )?;
        } else {
            let upper_loader = ensure_array(upper_start)?;
            let mut arr = upper_loader.load_mut()?;
            arr.init_tick(
                tick_upper,
                tick_spacing,
                market.current_tick,
                market.fee_growth_global_0_x64,
                market.fee_growth_global_1_x64,
            )?;
            if apply_liq && e.liquidity > 0 {
                {
                    let mut arrl = lower_loader.load_mut()?;
                    arrl.update_liquidity(tick_lower, tick_spacing, e.liquidity as i128, false)?;
                }
                {
                    let mut arru = upper_loader.load_mut()?;
                    arru.update_liquidity(tick_upper, tick_spacing, e.liquidity as i128, true)?;
                }
            }
            continue;
        }
        // If same array and apply nets
        if apply_liq && e.liquidity > 0 {
            let mut arr = lower_loader.load_mut()?;
            arr.update_liquidity(tick_lower, tick_spacing, e.liquidity as i128, false)?;
            arr.update_liquidity(tick_upper, tick_spacing, e.liquidity as i128, true)?;
        }
    }

    if apply_liq {
        ctx.accounts.tranche_plan.applied = true;
    }

    Ok(())
}
