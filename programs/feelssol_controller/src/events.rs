use anchor_lang::prelude::*;

#[event]
pub struct FeelsSolInitialized {
    pub underlying_mint: Pubkey,
    pub feels_protocol: Pubkey,
}

#[event]
pub struct DepositEvent {
    pub user: Pubkey,
    pub lst_deposited: u64,
    pub feelssol_minted: u64,
    pub current_lst_amount_wrapped: u64,
}

#[event]
pub struct WithdrawEvent {
    pub user: Pubkey,
    pub feelssol_burned: u64,
    pub lst_withdrawn: u64,
    pub current_lst_amount_wrapped: u64,
}
