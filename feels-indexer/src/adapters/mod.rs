//! Adapters layer - Infrastructure implementations
//!
//! This layer contains all the infrastructure adapters that implement
//! the port traits defined in the core layer. These adapters connect
//! the domain logic to external systems like databases, caches, and
//! blockchain nodes.

pub mod solana;
pub mod storage;

// Re-export commonly used adapters
pub use storage::{StorageAdapter, PostgresClient, RedisClient, RocksDBClient, TantivyClient};

