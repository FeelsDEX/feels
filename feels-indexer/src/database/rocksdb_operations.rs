//! RocksDB operations for raw blockchain data storage

use super::rocksdb::{RocksDBManager, ColumnFamilies};
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

impl RocksDBManager {
    /// Store raw account data
    pub async fn store_account(&self, pubkey: &Pubkey, data: &[u8], slot: u64) -> Result<()> {
        let key = format!("account:{}:{}", pubkey, slot);
        self.put_raw(ColumnFamilies::ACCOUNTS, key.as_bytes(), data)?;
        
        // Also store latest account state
        let latest_key = format!("account:{}:latest", pubkey);
        self.put_raw(ColumnFamilies::ACCOUNTS, latest_key.as_bytes(), data)?;
        
        Ok(())
    }

    /// Get account data at specific slot
    pub async fn get_account_at_slot(&self, pubkey: &Pubkey, slot: u64) -> Result<Option<Vec<u8>>> {
        let key = format!("account:{}:{}", pubkey, slot);
        self.get(ColumnFamilies::ACCOUNTS, key.as_bytes())
    }

    /// Get latest account data
    pub async fn get_latest_account(&self, pubkey: &Pubkey) -> Result<Option<Vec<u8>>> {
        let key = format!("account:{}:latest", pubkey);
        self.get(ColumnFamilies::ACCOUNTS, key.as_bytes())
    }

    /// Store raw transaction data
    pub async fn store_transaction(&self, signature: &str, data: &[u8], slot: u64) -> Result<()> {
        let key = format!("tx:{}", signature);
        
        // Create metadata
        let metadata = TransactionMetadata {
            slot,
            timestamp: chrono::Utc::now().timestamp(),
            size: data.len(),
        };
        
        // Store transaction data
        self.put_raw(ColumnFamilies::TRANSACTIONS, key.as_bytes(), data)?;
        
        // Store metadata
        let meta_key = format!("tx:{}:meta", signature);
        self.put(ColumnFamilies::TRANSACTIONS, meta_key.as_bytes(), &metadata)?;
        
        Ok(())
    }

    /// Get transaction data
    pub async fn get_transaction(&self, signature: &str) -> Result<Option<Vec<u8>>> {
        let key = format!("tx:{}", signature);
        self.get(ColumnFamilies::TRANSACTIONS, key.as_bytes())
    }

    /// Store block metadata
    pub async fn store_block_metadata(&self, slot: u64, block_hash: &str, parent_slot: u64) -> Result<()> {
        let key = format!("block:{}", slot);
        
        let metadata = BlockMetadata {
            slot,
            block_hash: block_hash.to_string(),
            parent_slot,
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        self.put(ColumnFamilies::BLOCKS, key.as_bytes(), &metadata)?;
        
        Ok(())
    }

    /// Get accounts modified in a slot range
    pub async fn get_accounts_in_slot_range(&self, start_slot: u64, end_slot: u64) -> Result<Vec<(Pubkey, u64)>> {
        let start_key = "account:".to_string();
        let end_key = "account:~".to_string(); // ~ is after all valid pubkeys
        
        let mut accounts = Vec::new();
        
        // Iterate through accounts column family
        let iter = self.iter_range(ColumnFamilies::ACCOUNTS, start_key.as_bytes(), end_key.into_bytes())?;
        
        for (key, _value) in iter {
            let key_str = String::from_utf8_lossy(&key);
            if let Some(parts) = parse_account_key(&key_str) {
                if parts.1 >= start_slot && parts.1 <= end_slot {
                    accounts.push((parts.0, parts.1));
                }
            }
        }
        
        Ok(accounts)
    }

    /// Compact database
    pub async fn compact(&self) -> Result<()> {
        // Compact each column family
        for cf in &[ColumnFamilies::ACCOUNTS, ColumnFamilies::TRANSACTIONS, ColumnFamilies::BLOCKS, ColumnFamilies::SNAPSHOTS] {
            self.compact_range(cf, None, None)?;
        }
        Ok(())
    }
    
    /// Batch write accounts
    pub async fn batch_write_accounts(&self, accounts: Vec<(Pubkey, Vec<u8>, u64)>) -> Result<()> {
        let mut batch = HashMap::new();
        
        for (pubkey, data, slot) in accounts {
            // Historical key
            let key = format!("account:{}:{}", pubkey, slot);
            batch.insert(key.into_bytes(), data.clone());
            
            // Latest key
            let latest_key = format!("account:{}:latest", pubkey);
            batch.insert(latest_key.into_bytes(), data);
        }
        
        self.batch_write(ColumnFamilies::ACCOUNTS, batch)
    }
    
    /// Batch write transactions
    pub async fn batch_write_transactions(&self, transactions: Vec<(String, Vec<u8>, u64)>) -> Result<()> {
        let mut batch = HashMap::new();
        
        for (signature, data, slot) in transactions {
            // Transaction data
            let key = format!("tx:{}", signature);
            batch.insert(key.into_bytes(), data.clone());
            
            // Transaction metadata
            let metadata = TransactionMetadata {
                slot,
                timestamp: chrono::Utc::now().timestamp(),
                size: data.len(),
            };
            let meta_key = format!("tx:{}:meta", signature);
            let meta_data = bincode::serialize(&metadata)?;
            batch.insert(meta_key.into_bytes(), meta_data);
        }
        
        self.batch_write(ColumnFamilies::TRANSACTIONS, batch)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TransactionMetadata {
    slot: u64,
    timestamp: i64,
    size: usize,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct BlockMetadata {
    slot: u64,
    block_hash: String,
    parent_slot: u64,
    timestamp: i64,
}

/// Parse account key to extract pubkey and slot
fn parse_account_key(key: &str) -> Option<(Pubkey, u64)> {
    let parts: Vec<&str> = key.split(':').collect();
    if parts.len() >= 3 && parts[0] == "account" && parts[2] != "latest" {
        if let Ok(pubkey) = parts[1].parse::<Pubkey>() {
            if let Ok(slot) = parts[2].parse::<u64>() {
                return Some((pubkey, slot));
            }
        }
    }
    None
}