use solana_program::pubkey::Pubkey;
use std::collections::HashMap;

/// Market state for swap simulation
#[derive(Clone, Debug)]
pub struct MarketState {
    pub market_key: Pubkey,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub sqrt_price: u128,
    pub current_tick: i32,
    pub liquidity: u128,
    pub fee_bps: u16,
    pub tick_spacing: u16,
    pub global_lower_tick: i32,
    pub global_upper_tick: i32,
    pub fee_growth_global_0: u128,
    pub fee_growth_global_1: u128,
}

/// Tick array view for Jupiter integration
#[derive(Clone, Debug)]
pub struct TickArrayView {
    pub start_tick_index: i32,
    pub ticks: Vec<TickData>,
    pub initialized_bitmap: Vec<bool>,
}

impl TickArrayView {
    pub fn new(start_tick_index: i32) -> Self {
        Self {
            start_tick_index,
            ticks: vec![TickData::default(); 64], // TICK_ARRAY_SIZE
            initialized_bitmap: vec![false; 64],
        }
    }

    pub fn from(parsed: ParsedTickArray) -> Self {
        let mut view = Self::new(parsed.start_tick_index);

        // Convert initialized ticks to view format
        for (tick_index, liquidity_net) in parsed.initialized_ticks {
            if let Some(array_index) = view.get_array_index(tick_index) {
                view.ticks[array_index].liquidity_net = liquidity_net;
                view.initialized_bitmap[array_index] = true;
            }
        }

        view
    }

    fn get_array_index(&self, tick_index: i32) -> Option<usize> {
        let relative_tick = tick_index - self.start_tick_index;
        if relative_tick >= 0 && relative_tick < 64 {
            Some(relative_tick as usize)
        } else {
            None
        }
    }
}

/// Individual tick data
#[derive(Clone, Debug, Default)]
pub struct TickData {
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_0_x64: u128,
    pub fee_growth_outside_1_x64: u128,
}

/// Tick array loader for managing multiple tick arrays
#[derive(Clone, Debug)]
pub struct TickArrayLoader {
    pub tick_arrays: HashMap<i32, TickArrayView>,
}

impl TickArrayLoader {
    pub fn new() -> Self {
        Self {
            tick_arrays: HashMap::new(),
        }
    }

    pub fn add_parsed_array(&mut self, parsed: ParsedTickArray) {
        let view = TickArrayView::from(parsed);
        self.tick_arrays.insert(view.start_tick_index, view);
    }

    pub fn get_tick(&self, tick_index: i32) -> Option<&TickData> {
        // Find which array contains this tick
        for (start_index, array) in &self.tick_arrays {
            let relative_tick = tick_index - start_index;
            if relative_tick >= 0 && relative_tick < 64 {
                let array_index = relative_tick as usize;
                if array.initialized_bitmap[array_index] {
                    return Some(&array.ticks[array_index]);
                }
            }
        }
        None
    }
}

/// Parsed tick array data
#[derive(Clone, Debug)]
pub struct ParsedTickArray {
    pub format: TickArrayFormat,
    pub market: Pubkey,
    pub start_tick_index: i32,
    pub initialized_ticks: HashMap<i32, i128>,
    pub initialized_count: Option<u16>,
}

/// Tick array format versions
#[derive(Clone, Debug, PartialEq)]
pub struct TickArrayFormat {
    pub version: u8,
    pub array_size: u16,
    pub discriminator: [u8; 8],
}

impl TickArrayFormat {
    /// V1 format - standard 64 tick array
    pub const V1: Self = Self {
        version: 1,
        array_size: 64,
        discriminator: [0xf0, 0x2f, 0x4e, 0xbd, 0x94, 0x8a, 0x8d, 0xd9],
    };

    pub fn calculate_total_size(&self) -> usize {
        // Discriminator (8) + market (32) + start_tick_index (4) + bump (1) + reserved (11)
        let header_size = 8 + 32 + 4 + 1 + 11;

        // Each tick: liquidity_net (16) + liquidity_gross (16) +
        // fee_growth_outside_0 (16) + fee_growth_outside_1 (16) +
        // initialized (1) + padding (15)
        let tick_size = 16 + 16 + 16 + 16 + 1 + 15;

        header_size + (self.array_size as usize * tick_size)
    }
}
