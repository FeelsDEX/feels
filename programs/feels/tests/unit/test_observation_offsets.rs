//! Test to debug Observation struct offsets

#[cfg(test)]
mod tests {
    use feels::state::oracle::Observation;
    use std::mem::{size_of, align_of, offset_of};
    
    #[test]
    fn print_observation_layout() {
        println!("Observation struct layout:");
        println!("  Size: {} bytes", size_of::<Observation>());
        println!("  Alignment: {} bytes", align_of::<Observation>());
        println!("  Field offsets:");
        println!("    block_timestamp: {}", offset_of!(Observation, block_timestamp));
        println!("    tick_cumulative: {}", offset_of!(Observation, tick_cumulative));
        println!("    initialized: {}", offset_of!(Observation, initialized));
        println!("    _padding: {}", offset_of!(Observation, _padding));
        
        // These should help us understand the actual layout
        assert!(size_of::<Observation>() <= 64);
    }
}