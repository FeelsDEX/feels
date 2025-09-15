//! Account structure macros

/// Macro for user token account pattern
#[macro_export]
macro_rules! user_token_accounts {
    () => {
        /// User account
        #[account(mut)]
        pub user: Signer<'info>,

        /// User's token account
        #[account(
            mut,
            constraint = user_token.owner == user.key() @ $crate::error::FeelsError::InvalidAuthority,
        )]
        pub user_token: Account<'info, TokenAccount>,
    };
}

/// Macro for validated user token accounts with mint check
#[macro_export]
macro_rules! user_token_accounts_with_mint {
    ($mint_field:expr) => {
        /// User account
        #[account(mut)]
        pub user: Signer<'info>,

        /// User's token account
        #[account(
            mut,
            constraint = user_token.owner == user.key() @ $crate::error::FeelsError::InvalidAuthority,
            constraint = user_token.mint == $mint_field @ $crate::error::FeelsError::InvalidMint,
        )]
        pub user_token: Account<'info, TokenAccount>,
    };
}

/// Macro for market with validation
#[macro_export]
macro_rules! validated_market {
    () => {
        /// Market account
        #[account(
            mut,
            constraint = market.is_initialized @ $crate::error::FeelsError::MarketNotInitialized,
            constraint = !market.is_paused @ $crate::error::FeelsError::MarketPaused,
        )]
        pub market: Account<'info, Market>,
    };
}

/// Macro for market with vaults and authority
/// Note: This macro is deprecated as vaults should be derived on-the-fly
/// Use UncheckedAccount and validate in the handler instead
#[macro_export]
macro_rules! market_with_vaults {
    () => {
        /// Market account
        #[account(
            mut,
            constraint = market.is_initialized @ $crate::error::FeelsError::MarketNotInitialized,
            constraint = !market.is_paused @ $crate::error::FeelsError::MarketPaused,
        )]
        pub market: Account<'info, Market>,

        /// Vault for token 0 - derived from market and token_0
        /// CHECK: Validated in handler
        #[account(mut)]
        pub vault_0: UncheckedAccount<'info>,

        /// Vault for token 1 - derived from market and token_1
        /// CHECK: Validated in handler
        #[account(mut)]
        pub vault_1: UncheckedAccount<'info>,

        /// Market authority PDA (unified authority)
        /// CHECK: PDA signer for vault operations
        #[account(
            seeds = [$crate::constants::MARKET_AUTHORITY_SEED, market.key().as_ref()],
            bump,
        )]
        pub market_authority: AccountInfo<'info>,
    };
}

/// Macro for standard program accounts
#[macro_export]
macro_rules! standard_programs {
    () => {
        /// Token program
        pub token_program: Program<'info, Token>,

        /// System program
        pub system_program: Program<'info, System>,
    };
}

/// Macro for buffer with authority
#[macro_export]
macro_rules! buffer_with_authority {
    () => {
        /// Buffer account
        #[account(
            mut,
            constraint = buffer.market == market.key() @ $crate::error::FeelsError::InvalidAuthority,
        )]
        pub buffer: Account<'info, Buffer>,

        /// Buffer authority PDA
        /// CHECK: PDA that controls buffer
        #[account(
            seeds = [$crate::constants::BUFFER_AUTHORITY_SEED, buffer.key().as_ref()],
            bump,
        )]
        pub buffer_authority: AccountInfo<'info>,
    };
}

/// Macro for vault PDA derivation
#[macro_export]
macro_rules! vault_pda {
    ($market:expr, $mint:expr) => {
        #[account(
            mut,
            seeds = [
                $crate::constants::VAULT_SEED,
                $market.as_ref(),
                $mint.as_ref(),
            ],
            bump,
        )]
    };
}

/// Macro for position with validation
#[macro_export]
macro_rules! validated_position {
    () => {
        /// Position account
        #[account(
            mut,
            constraint = position.is_initialized @ $crate::error::FeelsError::PositionNotInitialized,
            constraint = position.market == market.key() @ $crate::error::FeelsError::InvalidMarket,
        )]
        pub position: Account<'info, Position>,
    };
}

/// Macro for position with owner validation
#[macro_export]
macro_rules! position_with_owner {
    ($owner:expr) => {
        /// Position account
        #[account(
            mut,
            constraint = position.is_initialized @ $crate::error::FeelsError::PositionNotInitialized,
            constraint = position.market == market.key() @ $crate::error::FeelsError::InvalidMarket,
            constraint = position.owner == $owner @ $crate::error::FeelsError::InvalidAuthority,
        )]
        pub position: Account<'info, Position>,
    };
}

/// Macro for tick array validation
#[macro_export]
macro_rules! validated_tick_array {
    () => {
        /// Tick array account
        #[account(
            mut,
            constraint = tick_array.market == market.key() @ $crate::error::FeelsError::InvalidMarket,
            constraint = tick_array.start_tick_index % (market.tick_spacing as i32 * $crate::state::TICK_ARRAY_SIZE as i32) == 0
                @ $crate::error::FeelsError::InvalidTickArrayStartIndex,
        )]
        pub tick_array: Account<'info, TickArray>,
    };
}

/// Macro for oracle account validation
#[macro_export]
macro_rules! market_with_oracle {
    () => {
        /// Market account with oracle
        #[account(
            mut,
            constraint = market.is_initialized @ $crate::error::FeelsError::MarketNotInitialized,
            constraint = !market.is_paused @ $crate::error::FeelsError::MarketPaused,
            constraint = market.oracle_observation_cardinality > 0 @ $crate::error::FeelsError::OracleNotInitialized,
        )]
        pub market: Account<'info, Market>,
    };
}

/// Macro for position mint PDA
#[macro_export]
macro_rules! position_mint_pda {
    ($market:expr, $position_id:expr) => {
        #[account(
            init,
            payer = provider,
            mint::decimals = 0,
            mint::authority = position,
            seeds = [
                $crate::constants::POSITION_SEED,
                $market.as_ref(),
                &$position_id.to_le_bytes(),
            ],
            bump,
        )]
    };
}

/// Macro for buffer vault validation
#[macro_export]
macro_rules! buffer_vault {
    ($mint:expr) => {
        /// Buffer's token vault
        #[account(
            mut,
            constraint = buffer_vault.owner == buffer_authority.key() @ $crate::error::FeelsError::InvalidAuthority,
            constraint = buffer_vault.mint == $mint @ $crate::error::FeelsError::InvalidMint,
        )]
        pub buffer_vault: Account<'info, TokenAccount>,
    };
}

/// Macro for clock sysvar
#[macro_export]
macro_rules! clock_sysvar {
    () => {
        /// Clock sysvar
        pub clock: Sysvar<'info, Clock>,
    };
}
