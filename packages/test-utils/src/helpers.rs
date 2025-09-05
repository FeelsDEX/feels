use anchor_client::solana_sdk::{program_pack::Pack, signature::Keypair};
use anchor_lang::prelude::*;
use anchor_spl::token::spl_token;

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
