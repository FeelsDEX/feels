//! Minimal lib for testing without database dependencies

pub mod config;
pub mod sdk_types;
pub mod error;
pub mod models;

// Mock modules for testing
pub mod database {
    pub mod rocksdb;
    pub mod rocksdb_operations;
    pub mod redis;
    pub mod redis_operations;
    pub mod tantivy;
    pub mod mod_stub {
        pub use super::*;
    }
    
    pub use self::rocksdb::{RocksDBManager, ColumnFamilies};
    pub use self::redis::RedisManager;
    pub use self::tantivy::TantivyManager;
}

pub mod geyser {
    pub mod client;
    pub mod processor;
}

pub mod processors {
    pub mod market_processor;
    pub mod swap_processor;
    pub mod position_processor;
}

pub mod api {
    pub mod handlers;
    pub mod responses;
    pub mod server;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_compilation() {
        // This test ensures basic compilation works
        assert_eq!(1 + 1, 2);
    }
    
    #[test]
    fn test_config_loading() {
        let config = config::IndexerConfig::default();
        assert!(!config.geyser.endpoint.is_empty());
    }
}