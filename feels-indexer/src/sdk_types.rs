//! SDK type definitions for the indexer
//! 
//! These are simplified versions of the Feels SDK types used for indexing.
//! In production, these would come from the actual feels-sdk crate.

use serde::{Serialize, Deserialize};
use solana_sdk::pubkey::Pubkey;

/// Account types in the Feels protocol
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccountType {
    Market,
    Position,
    Buffer,
    ProtocolConfig,
    ProtocolToken,
    Vesting,
    Oracle,
}

impl AccountType {
    /// Determine account type from discriminator
    pub fn from_discriminator(discriminator: &[u8]) -> Option<Self> {
        if discriminator.len() < 8 {
            return None;
        }
        
        // These are placeholder discriminators - real ones come from IDL
        match discriminator {
            [219, 190, 213, 55, 0, 227, 198, 154] => Some(Self::Market),
            [170, 188, 143, 228, 122, 64, 247, 208] => Some(Self::Position),
            [123, 45, 67, 89, 12, 34, 56, 78] => Some(Self::Buffer),
            [234, 56, 78, 90, 123, 45, 67, 89] => Some(Self::ProtocolConfig),
            [111, 222, 33, 44, 55, 66, 77, 88] => Some(Self::ProtocolToken),
            _ => None,
        }
    }
}

/// Decoded market data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub sqrt_price: u128,
    pub liquidity: u128,
    pub current_tick: i32,
    pub tick_spacing: u16,
    pub base_fee_bps: u16,
    pub is_paused: bool,
    pub initial_liquidity_deployed: bool,
    pub global_lower_tick: i32,
    pub global_upper_tick: i32,
    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,
}

/// Decoded position data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionData {
    pub market: Pubkey,
    pub owner: Pubkey,
    pub liquidity: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub fee_growth_inside_0_last_x64: u128,
    pub fee_growth_inside_1_last_x64: u128,
    pub tokens_owed_0: u64,
    pub tokens_owed_1: u64,
}

/// Decoded buffer data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferData {
    pub market: Pubkey,
    pub tau_spot: u64,
    pub tau_time: u64,
    pub tau_leverage: u64,
}

/// Swap instruction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapData {
    pub market: Pubkey,
    pub trader: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub sqrt_price_before: u128,
    pub sqrt_price_after: u128,
    pub tick_before: i32,
    pub tick_after: i32,
    pub liquidity: u128,
    pub fee_amount: u64,
    pub price_impact_bps: u16,
    pub effective_price: f64,
}

/// Transaction instruction types
#[derive(Debug, Clone)]
pub enum Instruction {
    Swap(SwapData),
    OpenPosition(PositionData),
    ClosePosition(Pubkey),
    CollectFees(Pubkey),
    Other,
}

/// Parsed transaction
#[derive(Debug)]
pub struct ParsedTransaction {
    pub signature: String,
    pub instructions: Vec<Instruction>,
}

/// SDK mock functions for decoding
pub mod feels_sdk {
    use super::*;
    
    pub use super::{AccountType, MarketData, PositionData, BufferData, SwapData, Instruction};
    
    /// Decode account data based on discriminator
    pub fn decode_account_data(data: &[u8]) -> Option<AccountType> {
        AccountType::from_discriminator(data)
    }
    
    /// Decode market account
    pub fn decode_market(_data: &[u8]) -> Result<MarketData, String> {
        // In reality, this would use Anchor deserialization
        Ok(MarketData {
            token_0: Pubkey::default(),
            token_1: Pubkey::default(),
            sqrt_price: 79228162514264337593543950336,
            liquidity: 1000000,
            current_tick: 0,
            tick_spacing: 60,
            base_fee_bps: 30,
            is_paused: false,
            initial_liquidity_deployed: true,
            global_lower_tick: -887272,
            global_upper_tick: 887272,
            fee_growth_global_0_x64: 0,
            fee_growth_global_1_x64: 0,
        })
    }
    
    /// Decode position account
    pub fn decode_position(_data: &[u8]) -> Result<PositionData, String> {
        Ok(PositionData {
            market: Pubkey::default(),
            owner: Pubkey::default(),
            liquidity: 0,
            tick_lower: -1000,
            tick_upper: 1000,
            fee_growth_inside_0_last_x64: 0,
            fee_growth_inside_1_last_x64: 0,
            tokens_owed_0: 0,
            tokens_owed_1: 0,
        })
    }
    
    /// Decode buffer account
    pub fn decode_buffer(_data: &[u8]) -> Result<BufferData, String> {
        Ok(BufferData {
            market: Pubkey::default(),
            tau_spot: 0,
            tau_time: 0,
            tau_leverage: 0,
        })
    }
    
    /// Parse transaction
    pub fn parse_transaction(_data: &[u8]) -> Result<ParsedTransaction, String> {
        Ok(ParsedTransaction {
            signature: "mock".to_string(),
            instructions: vec![],
        })
    }
}