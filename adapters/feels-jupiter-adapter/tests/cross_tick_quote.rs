use anchor_lang::prelude::*;
use feels_jupiter_adapter::amm::{
    parse_tick_array,
    next_initialized_tick,
    derive_tick_array,
    ticks_per_array,
    array_start_for_tick,
};
use ahash::AHashMap;

#[test]
fn test_parse_tick_array_and_next_initialized_tick() {
    // Build a minimal tick array bytes with one initialized tick at +1 spacing from start
    let tick_spacing: u16 = 1;
    let start_tick_index: i32 = 0;

    // Discriminator + market(32) + start_tick_index(4) + pad0(12) + ticks(80*64) + initialized_count(2) + pad1(14) + reserved(32)
    let mut data = vec![0u8; 8 + 32 + 4 + 12 + 80*64 + 2 + 14 + 32];
    // Write Anchor discriminator for TickArray: we don't know it here, zero is fine for parser
    // Market pubkey (skip)
    // start_tick_index
    let mut offs = 8 + 32;
    data[offs..offs+4].copy_from_slice(&start_tick_index.to_le_bytes());

    // Initialize tick at offset 1 (tick index = 1 * spacing)
    // Tick struct is 80 bytes; initialized byte is at offset 16+16+16+16 = 64
    let ticks_base = 8 + 32 + 4 + 12; // after header
    let idx = 1usize;
    let tick_off = ticks_base + idx * 80 + 64;
    data[tick_off] = 1u8; // initialized
    // liquidity_net (i128) at start of tick struct: write small positive
    let ln: i128 = 12345;
    let ln_off = ticks_base + idx * 80;
    data[ln_off..ln_off+16].copy_from_slice(&ln.to_le_bytes());

    // Parse
    let view = parse_tick_array(&data, tick_spacing).expect("parse tick array");
    assert_eq!(view.start_tick_index, start_tick_index);
    assert!(view.inits.contains_key(&1));
    assert_eq!(view.inits.get(&1).cloned().unwrap(), ln);

    // Prepare arrays cache and find next tick upward from 0
    let mut arrays = AHashMap::new();
    arrays.insert(start_tick_index, view);
    let next = next_initialized_tick(&arrays, 0, tick_spacing as i32, false)
        .expect("next initialized tick up");
    assert_eq!(next, 1);
}

#[test]
fn test_derive_helpers() {
    let market = Pubkey::new_unique();
    let tick_spacing: u16 = 10;
    let cur = 1234;
    let start = array_start_for_tick(cur, tick_spacing);
    assert_eq!(start % ticks_per_array(tick_spacing), 0);
    let arrays = derive_tick_array(&market, start);
    // PDA is deterministic
    let arrays2 = derive_tick_array(&market, start);
    assert_eq!(arrays, arrays2);
}

