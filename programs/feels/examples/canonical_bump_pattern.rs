//! Example of canonical bump storage pattern
//! 
//! This demonstrates the best practice for storing PDA bumps in account state
//! to avoid recomputation and ensure consistency.
//!
//! ## Pattern Overview
//!
//! When creating PDA-controlled accounts, always store the canonical bump seed
//! in the account's state. This provides several benefits:
//!
//! 1. **Performance**: No need to call `find_program_address` repeatedly
//! 2. **Security**: Ensures consistent bump usage across the program
//! 3. **Clarity**: Self-documenting which bump to use for each PDA
//! 4. **Gas efficiency**: Reduces compute unit usage in transactions
//!
//! ## Example: Market Authority Pattern
//!
//! ```rust,no_run
//! // In your state struct:
//! pub struct Market {
//!     // ... other fields ...
//!     
//!     /// Canonical bump for market authority PDA
//!     pub market_authority_bump: u8,
//!     
//!     /// Canonical bump for vault 0
//!     pub vault_0_bump: u8,
//!     
//!     /// Canonical bump for vault 1
//!     pub vault_1_bump: u8,
//! }
//!
//! // During initialization:
//! market.market_authority_bump = ctx.bumps.market_authority;
//! market.vault_0_bump = ctx.bumps.vault_0;
//! market.vault_1_bump = ctx.bumps.vault_1;
//!
//! // When using as a signer:
//! let seeds = &[
//!     b"market_authority",
//!     market.key().as_ref(),
//!     &[market.market_authority_bump], // Use stored bump
//! ];
//! ```
//!
//! ## Anti-pattern to Avoid
//!
//! ```rust,no_run
//! // DON'T DO THIS - expensive and can lead to inconsistencies
//! let (pda, bump) = Pubkey::find_program_address(
//!     &[b"market_authority", market.key().as_ref()],
//!     program_id,
//! );
//! ```

fn main() {
    println!("This example demonstrates the canonical bump pattern for PDAs");
    println!("See the source code comments for implementation details");
}