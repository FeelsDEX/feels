use anchor_client::solana_sdk::{program_pack::Pack, signature::Keypair};
use anchor_lang::prelude::*;
use anchor_spl::{
    token::spl_token,
    token_2022::spl_token_2022::{self, extension::StateWithExtensions},
};

pub fn get_token_balance(
    program: &anchor_client::Program<&Keypair>,
    token_account: &Pubkey,
) -> u64 {
    match program.rpc().get_account(token_account) {
        Ok(account_info) => {
            match spl_token::state::Account::unpack(&account_info.data) {
                Ok(token_account_data) => token_account_data.amount,
                Err(_) => 0, // Account exists but isn't a valid token account
            }
        }
        Err(_) => 0, // Account doesn't exist
    }
}

pub fn get_token2022_balance(
    program: &anchor_client::Program<&Keypair>,
    token_account: &Pubkey,
) -> u64 {
    match program.rpc().get_account(token_account) {
        Ok(account_info) => {
            // First try with extensions (the proper Token2022 way)
            match StateWithExtensions::<spl_token_2022::state::Account>::unpack(&account_info.data)
            {
                Ok(account_state) => account_state.base.amount,
                Err(_) => {
                    // Fallback: try without extensions
                    match spl_token_2022::state::Account::unpack(&account_info.data) {
                        Ok(token_account_data) => token_account_data.amount,
                        Err(_) => 0, // Return 0 instead of panicking
                    }
                }
            }
        }
        Err(_) => 0, // Return 0 if account doesn't exist instead of panicking
    }
}
