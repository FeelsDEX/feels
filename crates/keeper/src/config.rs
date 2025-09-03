use std::fs;
use serde::{Deserialize, Serialize};
use feels_types::{FeelsResult, FeelsProtocolError};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Keeper configuration loaded from TOML file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeeperConfig {
    /// Solana cluster to connect to
    pub cluster: String,
    
    /// Feels program ID
    #[serde(with = "pubkey_serde")]
    pub program_id: Pubkey,
    
    /// Minimum SOL balance to maintain (in lamports)
    pub min_balance_lamports: u64,
    
    /// Default update interval in seconds
    pub default_update_interval: u64,
    
    /// Maximum number of markets to update per batch
    pub max_batch_size: usize,
    
    /// Retry configuration
    pub retry: RetryConfig,
    
    /// List of markets to monitor and update
    pub markets: Vec<MarketConfig>,
}

/// Configuration for individual market
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketConfig {
    /// Market name for logging
    pub name: String,
    
    /// Market field account pubkey
    #[serde(with = "pubkey_serde")]
    pub market_pubkey: Pubkey,
    
    /// Minimum update interval for this market (seconds)
    pub min_update_interval: i64,
    
    /// Maximum staleness before forced update (seconds)
    pub max_staleness: i64,
    
    /// Change threshold to trigger updates (basis points)
    pub change_threshold_bps: u32,
    
    /// Priority level (higher = more frequent updates)
    pub priority: u8,
    
    /// Whether this market is enabled
    pub enabled: bool,
}

/// Retry configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetryConfig {
    /// Maximum number of retries for failed operations
    pub max_retries: u32,
    
    /// Base delay between retries in milliseconds
    pub base_delay_ms: u64,
    
    /// Maximum delay between retries in milliseconds  
    pub max_delay_ms: u64,
    
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
}

impl KeeperConfig {
    /// Load configuration from TOML file
    pub fn load(path: &str) -> FeelsResult<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| FeelsProtocolError::generic(&format!("Failed to read config file {}: {}", path, e)))?;
        
        let config: KeeperConfig = toml::from_str(&content)
            .map_err(|e| FeelsProtocolError::parse_error(&format!("Failed to parse config file {}: {}", path, e), None))?;
        
        config.validate()?;
        
        Ok(config)
    }
    
    /// Save configuration to TOML file
    pub fn save(&self, path: &str) -> FeelsResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| FeelsProtocolError::parse_error(&format!("Failed to serialize config: {}", e), None))?;
        fs::write(path, content)
            .map_err(|e| FeelsProtocolError::generic(&format!("Failed to write config file {}: {}", path, e)))?;
        Ok(())
    }
    
    /// Validate configuration
    fn validate(&self) -> FeelsResult<()> {
        if self.markets.is_empty() {
            return Err(FeelsProtocolError::invalid_parameter("markets", "empty", "at least one market"));
        }
        
        if self.min_balance_lamports < 1_000_000 {
            return Err(FeelsProtocolError::invalid_parameter("min_balance_lamports", &self.min_balance_lamports.to_string(), "at least 1000000 (0.001 SOL)"));
        }
        
        if self.default_update_interval == 0 {
            return Err(FeelsProtocolError::invalid_parameter("default_update_interval", "0", "greater than 0"));
        }
        
        if self.max_batch_size == 0 {
            return Err(FeelsProtocolError::invalid_parameter("max_batch_size", "0", "greater than 0"));
        }
        
        for market in &self.markets {
            market.validate()?;
        }
        
        self.retry.validate()?;
        
        Ok(())
    }
    
    /// Get enabled markets sorted by priority
    pub fn get_enabled_markets(&self) -> Vec<&MarketConfig> {
        let mut markets: Vec<_> = self.markets
            .iter()
            .filter(|m| m.enabled)
            .collect();
        
        markets.sort_by(|a, b| b.priority.cmp(&a.priority));
        markets
    }
}

impl MarketConfig {
    /// Validate market configuration
    fn validate(&self) -> FeelsResult<()> {
        if self.name.is_empty() {
            return Err(FeelsProtocolError::invalid_parameter("market_name", "empty", "non-empty string"));
        }
        
        if self.min_update_interval <= 0 {
            return Err(FeelsProtocolError::invalid_parameter("min_update_interval", &self.min_update_interval.to_string(), "greater than 0"));
        }
        
        if self.max_staleness <= self.min_update_interval {
            return Err(FeelsProtocolError::invalid_parameter("max_staleness", &self.max_staleness.to_string(), &format!("greater than min_update_interval ({})", self.min_update_interval)));
        }
        
        if self.change_threshold_bps > 10000 {
            return Err(FeelsProtocolError::invalid_parameter("change_threshold_bps", &self.change_threshold_bps.to_string(), "at most 10000 (100%)"));
        }
        
        Ok(())
    }
}

impl RetryConfig {
    /// Validate retry configuration
    fn validate(&self) -> FeelsResult<()> {
        if self.max_retries == 0 {
            return Err(FeelsProtocolError::invalid_parameter("max_retries", "0", "greater than 0"));
        }
        
        if self.base_delay_ms == 0 {
            return Err(FeelsProtocolError::invalid_parameter("base_delay_ms", "0", "greater than 0"));
        }
        
        if self.max_delay_ms < self.base_delay_ms {
            return Err(FeelsProtocolError::invalid_parameter("max_delay_ms", &self.max_delay_ms.to_string(), &format!("greater than or equal to base_delay_ms ({})", self.base_delay_ms)));
        }
        
        if self.backoff_multiplier <= 1.0 {
            return Err(FeelsProtocolError::invalid_parameter("backoff_multiplier", &self.backoff_multiplier.to_string(), "greater than 1.0"));
        }
        
        Ok(())
    }
    
    /// Calculate delay for retry attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        if attempt == 0 {
            return self.base_delay_ms;
        }
        
        let exponential_delay = self.base_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32);
        (exponential_delay as u64).min(self.max_delay_ms)
    }
}

impl Default for KeeperConfig {
    fn default() -> Self {
        Self {
            cluster: "mainnet".to_string(),
            program_id: Pubkey::from_str("Fee1sProtoco11111111111111111111111111111111").unwrap(),
            min_balance_lamports: 10_000_000, // 0.01 SOL
            default_update_interval: 60, // 1 minute
            max_batch_size: 10,
            retry: RetryConfig::default(),
            markets: vec![],
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30_000,
            backoff_multiplier: 2.0,
        }
    }
}

impl Default for MarketConfig {
    fn default() -> Self {
        Self {
            name: "Default Market".to_string(),
            market_pubkey: Pubkey::default(),
            min_update_interval: 30,
            max_staleness: 300,
            change_threshold_bps: 100, // 1%
            priority: 1,
            enabled: true,
        }
    }
}

/// Create example configuration file
pub fn create_example_config(path: &str) -> FeelsResult<()> {
    let example_config = KeeperConfig {
        cluster: "devnet".to_string(),
        program_id: Pubkey::from_str("Fee1sProtoco11111111111111111111111111111111").unwrap(),
        min_balance_lamports: 10_000_000,
        default_update_interval: 30,
        max_batch_size: 5,
        retry: RetryConfig::default(),
        markets: vec![
            MarketConfig {
                name: "SOL/USDC".to_string(),
                market_pubkey: Pubkey::from_str("11111111111111111111111111111112").unwrap(),
                min_update_interval: 30,
                max_staleness: 300,
                change_threshold_bps: 100,
                priority: 10,
                enabled: true,
            },
            MarketConfig {
                name: "ETH/USDC".to_string(), 
                market_pubkey: Pubkey::from_str("11111111111111111111111111111113").unwrap(),
                min_update_interval: 60,
                max_staleness: 600,
                change_threshold_bps: 150,
                priority: 8,
                enabled: true,
            },
        ],
    };
    
    example_config.save(path)?;
    Ok(())
}

// Custom serde module for Pubkey
mod pubkey_serde {
    use super::*;
    use serde::{Deserializer, Serializer};
    
    pub fn serialize<S>(pubkey: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&pubkey.to_string())
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Pubkey::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_validation() {
        let mut config = KeeperConfig::default();
        config.markets.push(MarketConfig::default());
        assert!(config.validate().is_ok());
        
        // Test invalid balance
        config.min_balance_lamports = 0;
        assert!(config.validate().is_err());
    }
    
    #[test] 
    fn test_retry_delay_calculation() {
        let retry_config = RetryConfig::default();
        
        assert_eq!(retry_config.delay_for_attempt(0), 1000);
        assert_eq!(retry_config.delay_for_attempt(1), 2000);
        assert_eq!(retry_config.delay_for_attempt(2), 4000);
        
        // Should cap at max_delay_ms
        assert_eq!(retry_config.delay_for_attempt(10), 30_000);
    }
}