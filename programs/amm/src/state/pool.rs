use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Pool {
    /// Token A mint
    pub token_mint_a: Pubkey,
    /// Token B mint  
    pub token_mint_b: Pubkey,
    /// Token A vault
    pub token_vault_a: Pubkey,
    /// Token B vault
    pub token_vault_b: Pubkey,

    /// Pool fee in basis points
    pub fee_bps: u16,
    /// Protocol fee in basis points (portion of fee_bps that goes to protocol)
    pub protocol_fee_bps: u16,

    /// Tick spacing
    pub tick_spacing: i32,

    /// Current sqrt price (Q64.64)
    pub sqrt_price: u128,
    /// Current tick
    pub tick: i32,
    /// Current liquidity
    pub liquidity: u128,

    /// Fee growth per unit of liquidity
    pub fee_growth_global_a: u128,
    pub fee_growth_global_b: u128,
}
