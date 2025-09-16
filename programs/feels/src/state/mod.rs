//! State structures
//!
//! State for Phase 1 implementation

pub mod buffer;
pub mod epoch_params;
pub mod escrow;
pub mod feels_hub;
pub mod liquidity_commitment;
pub mod market;
pub mod oracle;
pub mod phase;
pub mod pool_registry;
pub mod position;
pub mod protocol_config;
pub mod protocol_oracle;
pub mod safety_controller;
pub mod tick;
pub mod token_metadata;
pub mod tranche_plan;

pub use buffer::*;
pub use epoch_params::*;
pub use escrow::*;
pub use feels_hub::*;
pub use liquidity_commitment::*;
pub use market::*;
pub use oracle::*;
pub use phase::*;
pub use pool_registry::*;
pub use position::*;
pub use protocol_config::*;
pub use protocol_oracle::*;
pub use safety_controller::*;
pub use tick::*;
pub use token_metadata::*;
pub use tranche_plan::*;

// Compile-time assertions for account struct sizes
// These ensure our structs maintain expected memory layout
#[cfg(test)]
mod size_assertions {
    use super::*;
    
    // Regular account size checks
    #[test]
    fn test_account_sizes() {
        let mut all_passed = true;
        
        // Check each account struct
        let checks = vec![
            ("Market", std::mem::size_of::<Market>(), Market::LEN - 8),
            ("Position", std::mem::size_of::<Position>(), Position::LEN - 8),
            ("Buffer", std::mem::size_of::<Buffer>(), Buffer::LEN - 8),
            ("PreLaunchEscrow", std::mem::size_of::<PreLaunchEscrow>(), PreLaunchEscrow::LEN - 8),
            ("SafetyController", std::mem::size_of::<SafetyController>(), SafetyController::LEN - 8),
            ("EpochParams", std::mem::size_of::<EpochParams>(), EpochParams::LEN - 8),
            ("FeelsHub", std::mem::size_of::<FeelsHub>(), FeelsHub::LEN - 8),
            ("ProtocolToken", std::mem::size_of::<ProtocolToken>(), ProtocolToken::LEN - 8),
            ("ProtocolConfig", std::mem::size_of::<ProtocolConfig>(), ProtocolConfig::LEN - 8),
            ("ProtocolOracle", std::mem::size_of::<ProtocolOracle>(), ProtocolOracle::LEN - 8),
            ("OracleState", std::mem::size_of::<OracleState>(), OracleState::LEN - 8),
        ];
        
        for (name, actual, expected) in checks {
            if actual != expected {
                eprintln!("{} size mismatch: actual {} vs expected {} (diff: {})", 
                    name, actual, expected, actual as i32 - expected as i32);
                all_passed = false;
            }
        }
        
        assert!(all_passed, "Size mismatches found - see output above");
    }
    
    // Zero-copy struct size checks (already handled in tick.rs with const assertions)
    #[test]
    fn test_zero_copy_sizes() {
        // Tick struct
        assert_eq!(std::mem::size_of::<Tick>(), 80);
        
        // TickArray (without discriminator)
        assert_eq!(std::mem::size_of::<TickArray>() + 8, TickArray::LEN);
    }
}
