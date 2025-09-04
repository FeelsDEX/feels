use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Keeper {
    pub authority: Pubkey, // Authority that can update the keeper settings
    pub feelssol_to_lst_rate_numerator: u64, // Numerator for the exchange rate between FeelsSOL and LST
    pub feelssol_to_lst_rate_denominator: u64, // Denominator for the exchange rate between FeelsSOL and LST
    pub _reserved: [u8; 64],
}
