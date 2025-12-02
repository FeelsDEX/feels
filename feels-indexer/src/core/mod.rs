//! Core domain abstractions and types
//!
//! This module contains the foundational types, traits, and error definitions
//! that form the core of the indexer's domain model. It's designed to be
//! independent of any specific infrastructure concerns.

pub mod error;
pub mod traits;
pub mod types;

// Re-export commonly used items
pub use error::{IndexerError, IndexerResult, NetworkError, StorageError};
pub use traits::{AccountProcessor, CachePort, EventStreamPort, StoragePort, StorageHealth};
pub use types::{BlockInfo, MarketQuery, ProcessContext, UpdateRecord, UpdateType};

