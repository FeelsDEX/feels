//! Floor helpers (MVP)

/// Compute candidate floor tick given current tick and buffer
pub fn candidate_floor_tick(current_tick: i32, floor_buffer_ticks: i32) -> i32 {
    current_tick.saturating_sub(floor_buffer_ticks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_floor_tick_monotonic() {
        let b = 100;
        assert_eq!(candidate_floor_tick(10_000, b), 9_900);
        assert!(candidate_floor_tick(11_000, b) > candidate_floor_tick(10_000, b));
    }
}
