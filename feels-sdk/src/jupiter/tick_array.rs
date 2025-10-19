use crate::jupiter::types::*;
use solana_program::pubkey::Pubkey;
use std::collections::HashMap;

/// Parse tick array data automatically detecting format
pub fn parse_tick_array_auto(
    data: &[u8],
    tick_spacing: u16,
) -> Result<ParsedTickArray, crate::core::SdkError> {
    // Check minimum size
    if data.len() < 8 {
        return Err(crate::core::SdkError::InvalidTickArray);
    }

    // Check V1 format
    if data[..8] == TickArrayFormat::V1.discriminator {
        return parse_tick_array_v1(data, tick_spacing);
    }

    Err(crate::core::SdkError::InvalidTickArray)
}

/// Parse V1 format tick array
fn parse_tick_array_v1(
    data: &[u8],
    tick_spacing: u16,
) -> Result<ParsedTickArray, crate::core::SdkError> {
    let format = TickArrayFormat::V1;

    // Verify size is at least the expected size
    if data.len() < format.calculate_total_size() {
        return Err(crate::core::SdkError::InvalidTickArray);
    }

    // Parse header
    let mut offset = 8; // Skip discriminator

    // Market pubkey (32 bytes)
    let market = Pubkey::try_from(&data[offset..offset + 32])
        .map_err(|_| crate::core::SdkError::InvalidTickArray)?;
    offset += 32;

    // Start tick index (4 bytes)
    let start_tick_index = i32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .map_err(|_| crate::core::SdkError::InvalidTickArray)?,
    );
    offset += 4;

    // Skip bump (1 byte) and reserved (11 bytes)
    offset += 12;

    // Parse ticks
    let mut initialized_ticks = HashMap::new();
    let mut initialized_count = 0u16;

    for i in 0..format.array_size {
        let tick_offset = offset + (i as usize * 80); // 80 bytes per tick

        // Check if tick is initialized (at offset 64 in the tick structure)
        let initialized = data[tick_offset + 64] != 0;

        if initialized {
            // Parse liquidity_net (first 16 bytes of tick)
            let liquidity_net = i128::from_le_bytes(
                data[tick_offset..tick_offset + 16]
                    .try_into()
                    .map_err(|_| crate::core::SdkError::InvalidTickArray)?,
            );

            let tick_index = start_tick_index + (i as i32 * tick_spacing as i32);
            initialized_ticks.insert(tick_index, liquidity_net);
            initialized_count += 1;
        }
    }

    Ok(ParsedTickArray {
        format,
        market,
        start_tick_index,
        initialized_ticks,
        initialized_count: Some(initialized_count),
    })
}
