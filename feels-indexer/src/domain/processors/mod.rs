//! Domain processors
//!
//! Business logic for processing account and transaction updates.
//! Processors transform raw blockchain data into domain models.

mod market;
mod position;
mod registry;

pub use market::MarketAccountProcessor;
pub use position::PositionAccountProcessor;
pub use registry::{ProcessorRegistry, Discriminators};

