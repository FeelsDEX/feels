pub mod authority_transfer;
pub mod create_token;
pub mod deposit;
pub mod initialize;
pub mod update_protocol;

pub use authority_transfer::*;
pub use create_token::*;
pub use deposit::*;
pub use initialize::*;
pub use update_protocol::*;

pub const MAX_PROTOCOL_FEE_RATE: u16 = 5000; // 50%
pub const MAX_POOL_FEE_RATE: u16 = 10000; // 100%
pub const AUTHORITY_TRANSFER_DELAY: i64 = 24 * 60 * 60; // 24 hours in seconds
