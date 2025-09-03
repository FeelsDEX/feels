/// Unified market data source interface consolidating keeper field commitments and oracle feeds.
/// Single entry point for all market data updates - replaces separate keeper/oracle systems.
use anchor_lang::prelude::*;
// use crate::state::{MarketField, FieldCommitment}; // Unused imports
use crate::error::FeelsProtocolError;

// ============================================================================
// Data Source Configuration
// ============================================================================

/// Data source type and configuration (zero-copy compatible)
#[zero_copy]
#[derive(Default)]
#[repr(C, packed)]
pub struct DataSourceConfig {
    /// Type of data source (0=Keeper, 1=Oracle, 2=Hybrid)
    pub source_type: u8,
    
    /// Keeper configuration
    pub keeper_config: KeeperConfig,
    
    /// Oracle configuration  
    pub oracle_config: OracleConfig,
    
    /// Fallback configuration enabled (0 = false, 1 = true)
    pub fallback_enabled: u8,
    
    /// Reserved for future use
    pub _reserved: [u8; 32],
}

/// Data source types as constants
pub const DATA_SOURCE_TYPE_KEEPER: u8 = 0;
pub const DATA_SOURCE_TYPE_ORACLE: u8 = 1;
pub const DATA_SOURCE_TYPE_HYBRID: u8 = 2;

/// Keeper-specific configuration (zero-copy compatible)
#[zero_copy]
#[derive(Default)]
#[repr(C, packed)]
pub struct KeeperConfig {
    /// Maximum staleness for field commitments (seconds)
    pub max_staleness: i64,
    /// Maximum rate of change per update (basis points)
    pub max_change_bps: u32,
    /// Sequence number validation enabled (0 = false, 1 = true)
    pub sequence_validation: u8,
    /// Reserved for future use
    pub _reserved: [u8; 16],
}

/// Oracle-specific configuration (zero-copy compatible)
#[zero_copy]
#[derive(Default)]
#[repr(C, packed)]
pub struct OracleConfig {
    /// Oracle account providing data
    pub oracle_account: Pubkey,
    /// Confidence threshold (basis points)
    pub confidence_threshold: u16,
    /// Maximum price staleness (seconds)
    pub max_price_staleness: i64,
    /// Reserved for future use
    pub _reserved: [u8; 16],
}

// ============================================================================
// Unified Market Data Source
// ============================================================================

/// Unified market data source - single interface for all data providers
#[account(zero_copy)]
#[derive(Default)]
#[repr(C, packed)]
pub struct MarketDataSource {
    /// Market field this source provides data for
    pub market_field: Pubkey,
    
    /// Pool reference (alias for market_field for compatibility)
    pub pool: Pubkey,
    
    /// Primary data provider (keeper or oracle)
    pub primary_provider: Pubkey,
    
    /// Secondary provider for redundancy
    pub secondary_provider: Pubkey,
    
    /// Data source configuration
    pub config: DataSourceConfig,
    
    /// Minimum time between updates (seconds)
    pub update_frequency: i64,
    
    /// Last update timestamp
    pub last_update: i64,
    
    /// Update counter for monitoring
    pub update_count: u64,
    
    /// Source is active and accepting updates
    pub is_active: u8, // 0 = false, 1 = true
    
    /// Current commitment root for verification
    pub commitment_root: [u8; 32],
    
    /// Last verified sequence number (monotonic)
    pub last_sequence: u64,
    
    /// Reserved for future use
    pub _reserved: [u8; 24],
}

impl MarketDataSource {
    pub const SIZE: usize = 8 +   // discriminator
        32 +                       // market_field
        32 +                       // pool
        32 +                       // primary_provider
        32 +                       // secondary_provider
        128 +                      // config (fixed size now)
        8 +                        // update_frequency
        8 +                        // last_update
        8 +                        // update_count
        1 +                        // is_active
        32 +                       // commitment_root
        8 +                        // last_sequence
        24;                        // reserved
    
    /// Check if provider is authorized to update this source
    pub fn is_authorized(&self, provider: &Pubkey) -> bool {
        self.is_active != 0 && (
            provider == &self.primary_provider ||
            provider == &self.secondary_provider
        )
    }
    
    /// Check if update is allowed based on frequency limits
    pub fn can_update(&self, current_time: i64) -> bool {
        current_time - self.last_update >= self.update_frequency
    }
    
    /// Check staleness with detailed error information
    pub fn check_staleness(&self, current_time: i64) -> Result<()> {
        // Check update frequency
        let time_since_last = current_time - self.last_update;
        if time_since_last < self.update_frequency {
            msg!("Cannot update: frequency violation");
            msg!("  Time since last: {} seconds", time_since_last);
            let freq = self.update_frequency;
            msg!("  Required interval: {} seconds", freq);
            return Err(FeelsProtocolError::UpdateTooFrequent.into());
        }
        
        // Check if data source is active
        if self.is_active == 0 {
            msg!("Cannot update: data source is inactive");
            return Err(FeelsProtocolError::StateError.into());
        }
        
        Ok(())
    }
    
    /// Get the appropriate field commitment source
    pub fn get_field_commitment_source(&self) -> Result<Pubkey> {
        match self.config.source_type {
            DATA_SOURCE_TYPE_KEEPER | DATA_SOURCE_TYPE_HYBRID => {
                // Return field commitment PDA
                let (field_commitment_pubkey, _) = Pubkey::find_program_address(
                    &[b"field_commitment", self.market_field.as_ref()],
                    &crate::ID
                );
                Ok(field_commitment_pubkey)
            },
            DATA_SOURCE_TYPE_ORACLE => {
                if self.config.fallback_enabled == 1 {
                    // Fallback to basic calculation
                    Ok(self.market_field)
                } else {
                    Err(FeelsProtocolError::NotInitialized.into())
                }
            },
            _ => Err(FeelsProtocolError::InvalidInput.into()),
        }
    }
    
    /// Update commitment root and sequence number
    pub fn update_commitment(&mut self, new_root: [u8; 32], sequence: u64) -> Result<()> {
        // Ensure sequence number is monotonically increasing
        require!(
            sequence > self.last_sequence,
            FeelsProtocolError::InvalidInput
        );
        
        self.commitment_root = new_root;
        self.last_sequence = sequence;
        
        Ok(())
    }
    
    /// Validate update sequence number
    pub fn validate_sequence(&self, sequence: u64) -> bool {
        sequence == self.last_sequence + 1
    }
}

// ============================================================================
// Market Update Types (Unified)
// ============================================================================

/// Unified market update that can come from keeper or oracle
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UnifiedMarketUpdate {
    /// Source of the update
    pub source: u8,
    
    /// Keeper field commitment data (if applicable)
    pub field_commitment: Option<FieldCommitmentData>,
    
    /// Oracle price data (if applicable)
    pub price_data: Option<OraclePriceData>,
    
    /// Update timestamp
    pub timestamp: i64,
    
    /// Update sequence number
    pub sequence: u64,
}

/// Field commitment data from keeper
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct FieldCommitmentData {
    /// Market scalars
    pub S: u128,
    pub T: u128,
    pub L: u128,
    
    /// Domain weights
    pub w_s: u32,
    pub w_t: u32,
    pub w_l: u32,
    pub w_tau: u32,
    
    /// Spot weights
    pub omega_0: u32,
    pub omega_1: u32,
    
    /// TWAPs
    pub twap_0: u128,
    pub twap_1: u128,
    
    /// Maximum staleness for this commitment
    pub max_staleness: i64,
}

/// Price status constants
pub const PRICE_STATUS_VALID: u8 = 0;
pub const PRICE_STATUS_STALE: u8 = 1;
pub const PRICE_STATUS_LOW_CONFIDENCE: u8 = 2;
pub const PRICE_STATUS_OFFLINE: u8 = 3;

/// Oracle price data
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct OraclePriceData {
    /// Price value
    pub price: u64,
    
    /// Confidence interval
    pub confidence: u64,
    
    /// Price status
    pub status: u8,
}

// ============================================================================
// Update Validation
// ============================================================================

/// Validate unified market update with enhanced checks
pub fn verify_market_update(
    update: &UnifiedMarketUpdate,
    data_source: &MarketDataSource,
    current_time: i64,
) -> Result<()> {
    // Check source authorization
    require!(
        data_source.config.source_type == update.source ||
        data_source.config.source_type == DATA_SOURCE_TYPE_HYBRID,
        FeelsProtocolError::Unauthorized
    );
    
    // Validate sequence number is monotonic
    require!(
        update.sequence > data_source.last_sequence,
        FeelsProtocolError::InvalidSequence
    );
    
    // Validate update frequency
    let time_since_last = current_time - data_source.last_update;
    if time_since_last < data_source.update_frequency {
        msg!("UPDATE FREQUENCY VIOLATION");
        msg!("  Time since last update: {} seconds", time_since_last);
        let update_frequency = data_source.update_frequency;
        msg!("  Required interval: {} seconds", update_frequency);
        msg!("  Too frequent by: {} seconds", data_source.update_frequency - time_since_last);
        return Err(FeelsProtocolError::UpdateTooFrequent.into());
    }
    
    // Validate timestamp freshness
    let age = current_time - update.timestamp;
    require!(
        age >= 0 && age <= 300, // Max 5 minutes old
        FeelsProtocolError::StaleData
    );
    
    // Validate based on source type
    match update.source {
        DATA_SOURCE_TYPE_KEEPER => {
            if let Some(field_data) = &update.field_commitment {
                validate_keeper_update(field_data, &data_source.config.keeper_config)?;
                
                // Additional staleness check for keeper data
                let keeper_age = current_time - update.timestamp;
                if keeper_age > data_source.config.keeper_config.max_staleness {
                    msg!("KEEPER DATA STALENESS VIOLATION");
                    msg!("  Keeper data age: {} seconds", keeper_age);
                    let max_staleness = data_source.config.keeper_config.max_staleness;
                    msg!("  Maximum allowed: {} seconds", max_staleness);
                    msg!("  Stale by: {} seconds", keeper_age - data_source.config.keeper_config.max_staleness);
                    return Err(FeelsProtocolError::StaleData.into());
                }
                
                // Check commitment expiration
                if current_time > field_data.max_staleness {
                    msg!("COMMITMENT EXPIRED");
                    msg!("  Current time: {}", current_time);
                    msg!("  Expired at: {}", field_data.max_staleness);
                    msg!("  Expired by: {} seconds", current_time - field_data.max_staleness);
                    return Err(FeelsProtocolError::CommitmentExpired.into());
                }
            } else {
                return Err(FeelsProtocolError::NotInitialized.into());
            }
        },
        DATA_SOURCE_TYPE_ORACLE => {
            if let Some(price_data) = &update.price_data {
                validate_oracle_update(price_data, &data_source.config.oracle_config, current_time)?;
                
                // Enhanced confidence check
                validate_price_confidence(price_data, &data_source.config.oracle_config)?;
            } else {
                return Err(FeelsProtocolError::NotInitialized.into());
            }
        },
        DATA_SOURCE_TYPE_HYBRID => {
            // Both sources should be present
            require!(
                update.field_commitment.is_some() || update.price_data.is_some(),
                FeelsProtocolError::NotInitialized
            );
            
            // Validate both if present
            if let Some(field_data) = &update.field_commitment {
                validate_keeper_update(field_data, &data_source.config.keeper_config)?;
                
                // Check commitment expiration for hybrid mode
                if current_time > field_data.max_staleness {
                    msg!("COMMITMENT EXPIRED (HYBRID MODE)");
                    msg!("  Current time: {}", current_time);
                    msg!("  Expired at: {}", field_data.max_staleness);
                    msg!("  Expired by: {} seconds", current_time - field_data.max_staleness);
                    return Err(FeelsProtocolError::CommitmentExpired.into());
                }
            }
            if let Some(price_data) = &update.price_data {
                validate_oracle_update(price_data, &data_source.config.oracle_config, current_time)?;
            }
        },
        _ => return Err(FeelsProtocolError::InvalidInput.into()),
    }
    
    Ok(())
}

/// Validate keeper field commitment update
fn validate_keeper_update(
    field_data: &FieldCommitmentData,
    keeper_config: &KeeperConfig,
) -> Result<()> {
    // Basic field validation
    require!(
        field_data.S > 0 && field_data.T > 0 && field_data.L > 0,
        FeelsProtocolError::ValidationError
    );
    
    // Weight validation
    require!(
        field_data.w_s + field_data.w_t + field_data.w_l + field_data.w_tau == 10000,
        FeelsProtocolError::InvalidInput
    );
    
    require!(
        field_data.omega_0 + field_data.omega_1 == 10000,
        FeelsProtocolError::InvalidInput
    );
    
    // Additional keeper-specific validation if configured
    if keeper_config.sequence_validation == 1 {
        // Rate of change validation would be implemented here
        // Currently delegated to keeper_update instruction
    }
    
    Ok(())
}

/// Validate oracle price update
fn validate_oracle_update(
    price_data: &OraclePriceData,
    oracle_config: &OracleConfig,
    _current_time: i64,
) -> Result<()> {
    // Check price status
    require!(
        price_data.status == PRICE_STATUS_VALID || price_data.status == PRICE_STATUS_STALE,
        FeelsProtocolError::ValidationError
    );
    
    // Confidence check
    let confidence_bps = (price_data.confidence * 10000 / price_data.price.max(1)) as u16;
    require!(
        confidence_bps <= oracle_config.confidence_threshold,
        FeelsProtocolError::ValidationError
    );
    
    Ok(())
}

/// Enhanced price confidence validation
fn validate_price_confidence(
    price_data: &OraclePriceData,
    oracle_config: &OracleConfig,
) -> Result<()> {
    // Ensure price is positive
    require!(
        price_data.price > 0,
        FeelsProtocolError::InvalidAmount
    );
    
    // Calculate confidence as percentage of price
    let confidence_ratio = (price_data.confidence as u128 * 10000) / (price_data.price as u128);
    
    // Dynamic threshold based on price status
    let adjusted_threshold = match price_data.status {
        PRICE_STATUS_VALID => oracle_config.confidence_threshold,
        PRICE_STATUS_STALE => oracle_config.confidence_threshold * 2, // More lenient for stale prices
        _ => return Err(FeelsProtocolError::ValidationError.into()),
    };
    
    require!(
        confidence_ratio <= adjusted_threshold as u128,
        FeelsProtocolError::ValidationError
    );
    
    // Additional check: absolute confidence bounds
    require!(
        price_data.confidence < price_data.price / 2, // Confidence can't exceed 50% of price
        FeelsProtocolError::ValidationError
    );
    
    Ok(())
}

// ============================================================================
// Default Configurations
// ============================================================================

