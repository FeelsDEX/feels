use anchor_lang::prelude::*;

#[event]
pub struct FeelsKeepersInitialized {
    pub feelssol_to_lst_rate_numerator: u64,
    pub feelssol_to_lst_rate_denominator: u64,
}
