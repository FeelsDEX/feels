/// Instruction module organizing all protocol operations into logical groups.
/// Initialization instructions set up protocol and pool infrastructure,
/// liquidity instructions manage LP positions, swap instructions handle trading,
/// and fee/keeper instructions manage protocol revenue and optimizations.
pub mod initialize_protocol;
pub mod initialize_feelssol;
pub mod initialize_pool;
pub mod token_create;
pub mod liquidity_add;
pub mod liquidity_remove;
pub mod fee_collect_pool;
pub mod fee_collect_protocol;
pub mod tick_cleanup;
pub mod swap_execute;
pub mod swap_compute_tick;
pub mod keeper_update_tick;
