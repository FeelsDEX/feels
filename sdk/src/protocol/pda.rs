use anchor_lang::prelude::*;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::core::constants::*;

/// PDA cache to avoid recomputing addresses
pub struct PdaCache {
    cache: RwLock<HashMap<String, (Pubkey, u8)>>,
}

impl PdaCache {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn get_or_compute<F>(&self, key: &str, compute: F) -> (Pubkey, u8)
    where
        F: FnOnce() -> (Pubkey, u8),
    {
        if let Some(cached) = self.cache.read().unwrap().get(key) {
            return *cached;
        }

        let result = compute();
        self.cache.write().unwrap().insert(key.to_string(), result);
        result
    }
}

impl Default for PdaCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified PDA builder for all protocol addresses
pub struct PdaBuilder {
    cache: PdaCache,
    pub program_id: Pubkey,
}

impl PdaBuilder {
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            cache: PdaCache::new(),
            program_id,
        }
    }

    pub fn market(&self, token_0: &Pubkey, token_1: &Pubkey) -> (Pubkey, u8) {
        let key = format!("market:{}:{}", token_0, token_1);
        self.cache.get_or_compute(&key, || {
            Pubkey::find_program_address(
                &[seeds::MARKET, token_0.as_ref(), token_1.as_ref()],
                &self.program_id,
            )
        })
    }

    pub fn buffer(&self, market: &Pubkey) -> (Pubkey, u8) {
        let key = format!("buffer:{}", market);
        self.cache.get_or_compute(&key, || {
            Pubkey::find_program_address(&[seeds::BUFFER, market.as_ref()], &self.program_id)
        })
    }

    pub fn vault_authority(&self, market: &Pubkey) -> (Pubkey, u8) {
        let key = format!("vault_authority:{}", market);
        self.cache.get_or_compute(&key, || {
            Pubkey::find_program_address(
                &[seeds::VAULT_AUTHORITY, market.as_ref()],
                &self.program_id,
            )
        })
    }

    pub fn oracle(&self, market: &Pubkey) -> (Pubkey, u8) {
        let key = format!("oracle:{}", market);
        self.cache.get_or_compute(&key, || {
            Pubkey::find_program_address(&[seeds::ORACLE, market.as_ref()], &self.program_id)
        })
    }

    pub fn tick_array(&self, market: &Pubkey, start_tick: i32) -> (Pubkey, u8) {
        let key = format!("tick_array:{}:{}", market, start_tick);
        self.cache.get_or_compute(&key, || {
            Pubkey::find_program_address(
                &[
                    seeds::TICK_ARRAY,
                    market.as_ref(),
                    &start_tick.to_le_bytes(),
                ],
                &self.program_id,
            )
        })
    }

    pub fn position(&self, owner: &Pubkey, tick_lower: i32, tick_upper: i32) -> (Pubkey, u8) {
        let key = format!("position:{}:{}:{}", owner, tick_lower, tick_upper);
        self.cache.get_or_compute(&key, || {
            Pubkey::find_program_address(
                &[
                    seeds::POSITION,
                    owner.as_ref(),
                    &tick_lower.to_le_bytes(),
                    &tick_upper.to_le_bytes(),
                ],
                &self.program_id,
            )
        })
    }

    pub fn position_metadata(&self, position: &Pubkey) -> (Pubkey, u8) {
        let key = format!("position_metadata:{}", position);
        self.cache.get_or_compute(&key, || {
            Pubkey::find_program_address(
                &[seeds::POSITION_METADATA, position.as_ref()],
                &self.program_id,
            )
        })
    }

    pub fn protocol_config(&self) -> (Pubkey, u8) {
        let key = "protocol_config";
        self.cache.get_or_compute(key, || {
            Pubkey::find_program_address(&[seeds::PROTOCOL_CONFIG], &self.program_id)
        })
    }

    pub fn protocol_oracle(&self) -> (Pubkey, u8) {
        let key = "protocol_oracle";
        self.cache.get_or_compute(key, || {
            Pubkey::find_program_address(&[seeds::PROTOCOL_ORACLE], &self.program_id)
        })
    }

    pub fn feels_hub(&self) -> (Pubkey, u8) {
        let key = "feels_hub";
        self.cache.get_or_compute(key, || {
            Pubkey::find_program_address(&[seeds::FEELS_HUB], &self.program_id)
        })
    }

    pub fn feels_mint(&self) -> (Pubkey, u8) {
        let key = "feels_mint";
        self.cache.get_or_compute(key, || {
            Pubkey::find_program_address(&[seeds::FEELS_MINT], &self.program_id)
        })
    }
}

/// Convenience functions for one-off PDA derivations
pub fn find_market_address(token_0: &Pubkey, token_1: &Pubkey) -> (Pubkey, u8) {
    PdaBuilder::new(program_id()).market(token_0, token_1)
}

pub fn find_buffer_address(market: &Pubkey) -> (Pubkey, u8) {
    PdaBuilder::new(program_id()).buffer(market)
}

pub fn find_vault_authority_address(market: &Pubkey) -> (Pubkey, u8) {
    PdaBuilder::new(program_id()).vault_authority(market)
}