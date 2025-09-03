//! Hysteresis-based dynamic fee controller for market stress management.
//! 
//! This controller adjusts base fees based on composite market stress signals
//! while preventing oscillation through hysteresis bands and directional memory.

use crate::error::KeeperError;

/// Direction of the last fee adjustment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    None,
    Up,
    Down,
}

/// Stress components across market dimensions
#[derive(Debug, Clone, Copy)]
pub struct StressComponents {
    /// Spot dimension stress: |price - twap| / twap × 10000
    pub spot_stress: u64,
    
    /// Time dimension stress: utilization × 10000
    pub time_stress: u64,
    
    /// Leverage dimension stress: |L_long - L_short| / (L_long + L_short) × 10000
    pub leverage_stress: u64,
}

/// Domain weights for stress calculation
#[derive(Debug, Clone, Copy)]
pub struct DomainWeights {
    /// Spot weight (basis points)
    pub w_s: u32,
    /// Time weight (basis points)
    pub w_t: u32,
    /// Leverage weight (basis points)
    pub w_l: u32,
}

impl DomainWeights {
    /// Validate weights sum to 10000 (excluding buffer)
    pub fn validate(&self) -> Result<(), KeeperError> {
        let sum = self.w_s + self.w_t + self.w_l;
        if sum != 10000 {
            return Err(KeeperError::InvalidWeights(format!(
                "Domain weights must sum to 10000, got {}",
                sum
            )));
        }
        Ok(())
    }
}

/// Hysteresis controller for dynamic base fee adjustment
#[derive(Debug, Clone)]
pub struct HysteresisController {
    // Current state
    /// Current base fee in basis points
    pub current_fee: u64,
    /// Exponentially weighted moving average of stress signal (basis points)
    pub stress_ewma: u64,
    /// Last fee adjustment timestamp (unix seconds)
    pub last_update: i64,
    /// Last EWMA update timestamp (unix seconds)
    pub last_ewma_update: i64,
    /// Direction of last adjustment
    pub last_direction: Direction,
    
    // Hysteresis bands (in stress basis points)
    /// Lower trigger threshold (e.g., 2000 = 20%)
    pub outer_down: u64,
    /// Lower release threshold (e.g., 3000 = 30%)
    pub inner_down: u64,
    /// Upper release threshold (e.g., 7000 = 70%)
    pub inner_up: u64,
    /// Upper trigger threshold (e.g., 8000 = 80%)
    pub outer_up: u64,
    
    // Adjustment parameters
    /// Fee increase step in basis points (e.g., 5 bps)
    pub step_up: u64,
    /// Fee decrease step in basis points (e.g., 3 bps)
    pub step_down: u64,
    /// Minimum allowed fee in basis points (e.g., 10 bps)
    pub min_fee: u64,
    /// Maximum allowed fee in basis points (e.g., 150 bps)
    pub max_fee: u64,
    
    // Timing controls
    /// Minimum seconds between fee updates
    pub min_interval: i64,
    /// EWMA smoothing halflife in seconds
    pub ewma_halflife: i64,
    
    // Domain weights for stress calculation
    pub domain_weights: DomainWeights,
}

impl HysteresisController {
    /// Create a new hysteresis controller with default parameters
    pub fn new(domain_weights: DomainWeights) -> Result<Self, KeeperError> {
        domain_weights.validate()?;
        
        Ok(Self {
            // Start at moderate fee
            current_fee: 25,
            stress_ewma: 5000, // Start at 50% (neutral)
            last_update: 0,
            last_ewma_update: 0,
            last_direction: Direction::None,
            
            // Default hysteresis bands
            outer_down: 2000,  // 20%
            inner_down: 3000,  // 30%
            inner_up: 7000,    // 70%
            outer_up: 8000,    // 80%
            
            // Default adjustment parameters
            step_up: 5,
            step_down: 3,
            min_fee: 10,
            max_fee: 150,
            
            // Default timing
            min_interval: 300,  // 5 minutes
            ewma_halflife: 1800, // 30 minutes
            
            domain_weights,
        })
    }
    
    /// Compute weighted composite stress from components
    pub fn compute_stress(&self, components: &StressComponents) -> u64 {
        // Each component normalized to 0-10000 bps
        // Weights sum to 10000 (excluding buffer)
        let weighted_sum = 
            (components.spot_stress as u128 * self.domain_weights.w_s as u128 +
             components.time_stress as u128 * self.domain_weights.w_t as u128 +
             components.leverage_stress as u128 * self.domain_weights.w_l as u128) / 10000;
        
        // Result is normalized stress in basis points
        weighted_sum as u64
    }
    
    /// Update EWMA with exponential decay
    pub fn update_ewma(&mut self, new_value: u64, current_time: i64) -> u64 {
        if self.last_ewma_update == 0 {
            self.last_ewma_update = current_time;
            self.stress_ewma = new_value;
            return new_value;
        }
        
        let dt = current_time - self.last_ewma_update;
        if dt <= 0 {
            return self.stress_ewma;
        }
        
        // Calculate decay factor: alpha = 1 - 0.5^(dt/halflife)
        // Using fixed-point approximation to avoid floats
        let ratio = (dt * 10000) / self.ewma_halflife;
        
        // For small ratios, use linear approximation: 1 - 0.5^x ≈ 0.693 * x for x << 1
        let alpha_bp = if ratio < 1000 {
            (693 * ratio) / 1000 // 0.693 * ratio
        } else {
            // For larger ratios, use bounded approximation
            let shifts = ratio / 10000; // Number of halflife periods
            let remainder = ratio % 10000;
            
            // Each halflife reduces weight by half
            let base_alpha = 10000u64.saturating_sub(5000u64 >> shifts.min(13));
            
            // Add linear correction for remainder
            let correction = (693 * remainder) / 1000 * (5000u64 >> shifts.min(13)) / 10000;
            
            base_alpha.saturating_add(correction).min(10000)
        };
        
        // Apply EWMA update: new_ewma = alpha * new + (1 - alpha) * old
        let new_ewma = (new_value * alpha_bp + self.stress_ewma * (10000 - alpha_bp)) / 10000;
        
        self.last_ewma_update = current_time;
        self.stress_ewma = new_ewma;
        
        new_ewma
    }
    
    /// Main update function - calculates new base fee based on stress
    pub fn update(&mut self, stress_components: &StressComponents, current_time: i64) -> Result<u64, KeeperError> {
        // Update EWMA stress
        let raw_stress = self.compute_stress(stress_components);
        let smoothed_stress = self.update_ewma(raw_stress, current_time);
        
        // Check minimum interval
        if self.last_update > 0 && current_time - self.last_update < self.min_interval {
            return Ok(self.current_fee);
        }
        
        // Hysteresis logic
        let mut new_fee = self.current_fee;
        let mut direction_changed = false;
        
        // Check upper threshold
        if smoothed_stress > self.outer_up && 
           (self.last_direction != Direction::Down || smoothed_stress > self.inner_up) {
            // Increase fee
            let increase = self.step_up.min(self.max_fee.saturating_sub(self.current_fee));
            new_fee = self.current_fee + increase;
            
            if self.last_direction != Direction::Up {
                direction_changed = true;
            }
            self.last_direction = Direction::Up;
            self.last_update = current_time;
        }
        // Check lower threshold
        else if smoothed_stress < self.outer_down && 
                (self.last_direction != Direction::Up || smoothed_stress < self.inner_down) {
            // Decrease fee
            let decrease = self.step_down.min(self.current_fee.saturating_sub(self.min_fee));
            new_fee = self.current_fee.saturating_sub(decrease);
            
            if self.last_direction != Direction::Down {
                direction_changed = true;
            }
            self.last_direction = Direction::Down;
            self.last_update = current_time;
        }
        
        // Apply bounds
        new_fee = new_fee.clamp(self.min_fee, self.max_fee);
        
        // Log significant events
        if new_fee != self.current_fee {
            log::info!(
                "Hysteresis fee adjustment: {} -> {} bps (stress: {} bps, direction: {:?}{})",
                self.current_fee,
                new_fee,
                smoothed_stress,
                self.last_direction,
                if direction_changed { ", direction changed" } else { "" }
            );
        }
        
        self.current_fee = new_fee;
        Ok(new_fee)
    }
    
    /// Get current controller state for monitoring
    pub fn get_state(&self) -> ControllerState {
        ControllerState {
            current_fee: self.current_fee,
            stress_ewma: self.stress_ewma,
            last_direction: self.last_direction,
            last_update: self.last_update,
            in_dead_zone: self.is_in_dead_zone(),
        }
    }
    
    /// Check if stress is in the dead zone (between inner bands)
    pub fn is_in_dead_zone(&self) -> bool {
        self.stress_ewma >= self.inner_down && self.stress_ewma <= self.inner_up
    }
}

/// Controller state snapshot for monitoring
#[derive(Debug, Clone)]
pub struct ControllerState {
    pub current_fee: u64,
    pub stress_ewma: u64,
    pub last_direction: Direction,
    pub last_update: i64,
    pub in_dead_zone: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn default_weights() -> DomainWeights {
        DomainWeights {
            w_s: 7000,  // 70% spot
            w_t: 2000,  // 20% time
            w_l: 1000,  // 10% leverage
        }
    }
    
    fn high_stress() -> StressComponents {
        StressComponents {
            spot_stress: 9000,      // 90% stress
            time_stress: 8000,      // 80% stress
            leverage_stress: 7000,  // 70% stress
        }
    }
    
    fn low_stress() -> StressComponents {
        StressComponents {
            spot_stress: 1000,      // 10% stress
            time_stress: 2000,      // 20% stress
            leverage_stress: 1500,  // 15% stress
        }
    }
    
    fn medium_stress() -> StressComponents {
        StressComponents {
            spot_stress: 5000,      // 50% stress
            time_stress: 5000,      // 50% stress
            leverage_stress: 5000,  // 50% stress
        }
    }
    
    #[test]
    fn test_controller_creation() {
        let controller = HysteresisController::new(default_weights()).unwrap();
        assert_eq!(controller.current_fee, 25);
        assert_eq!(controller.stress_ewma, 5000);
        assert_eq!(controller.last_direction, Direction::None);
    }
    
    #[test]
    fn test_stress_computation() {
        let controller = HysteresisController::new(default_weights()).unwrap();
        
        let stress = StressComponents {
            spot_stress: 8000,      // 80% stress
            time_stress: 6000,      // 60% stress
            leverage_stress: 4000,  // 40% stress
        };
        
        // Expected: 0.7 * 8000 + 0.2 * 6000 + 0.1 * 4000 = 5600 + 1200 + 400 = 7200
        let computed = controller.compute_stress(&stress);
        assert_eq!(computed, 7200);
    }
    
    #[test]
    fn test_hysteresis_up_direction() {
        let mut controller = HysteresisController::new(default_weights()).unwrap();
        controller.min_interval = 0; // Disable time check for testing
        
        let base_time = 1000;
        
        // Start with high stress - should increase fee
        let new_fee = controller.update(&high_stress(), base_time).unwrap();
        assert_eq!(new_fee, 30); // 25 + 5
        assert_eq!(controller.last_direction, Direction::Up);
        
        // Stress still high but within dead zone - no change
        let medium_high = StressComponents {
            spot_stress: 7500,
            time_stress: 7500,
            leverage_stress: 7500,
        };
        let new_fee = controller.update(&medium_high, base_time + 1).unwrap();
        assert_eq!(new_fee, 30); // No change
        
        // Stress drops below inner_down - can now decrease
        let new_fee = controller.update(&low_stress(), base_time + 2).unwrap();
        assert_eq!(new_fee, 27); // 30 - 3
        assert_eq!(controller.last_direction, Direction::Down);
    }
    
    #[test]
    fn test_hysteresis_down_direction() {
        let mut controller = HysteresisController::new(default_weights()).unwrap();
        controller.min_interval = 0; // Disable time check for testing
        controller.stress_ewma = 1500; // Start with low stress
        
        let base_time = 1000;
        
        // Start with low stress - should decrease fee
        let new_fee = controller.update(&low_stress(), base_time).unwrap();
        assert_eq!(new_fee, 22); // 25 - 3
        assert_eq!(controller.last_direction, Direction::Down);
        
        // Stress increases but below inner_up - no change
        let new_fee = controller.update(&medium_stress(), base_time + 1).unwrap();
        assert_eq!(new_fee, 22); // No change
        
        // Stress above inner_up - can now increase
        let new_fee = controller.update(&high_stress(), base_time + 2).unwrap();
        assert_eq!(new_fee, 27); // 22 + 5
        assert_eq!(controller.last_direction, Direction::Up);
    }
    
    #[test]
    fn test_fee_bounds() {
        let mut controller = HysteresisController::new(default_weights()).unwrap();
        controller.min_interval = 0;
        controller.current_fee = 148; // Near max
        
        // Try to increase beyond max
        let new_fee = controller.update(&high_stress(), 1000).unwrap();
        assert_eq!(new_fee, 150); // Clamped to max
        
        // Set near min and try to decrease
        controller.current_fee = 12;
        controller.last_direction = Direction::None;
        controller.stress_ewma = 1000;
        
        let new_fee = controller.update(&low_stress(), 2000).unwrap();
        assert_eq!(new_fee, 10); // Clamped to min
    }
    
    #[test]
    fn test_min_interval_enforcement() {
        let mut controller = HysteresisController::new(default_weights()).unwrap();
        controller.last_update = 1000;
        
        // Try to update too soon
        let new_fee = controller.update(&high_stress(), 1100).unwrap();
        assert_eq!(new_fee, 25); // No change due to min interval
        
        // Update after min interval
        let new_fee = controller.update(&high_stress(), 1301).unwrap();
        assert_eq!(new_fee, 30); // Now updates
    }
    
    #[test]
    fn test_ewma_smoothing() {
        let mut controller = HysteresisController::new(default_weights()).unwrap();
        controller.ewma_halflife = 100; // Short halflife for testing
        
        // Initial update
        controller.update_ewma(8000, 0);
        assert_eq!(controller.stress_ewma, 8000);
        
        // Quick drop in stress - EWMA should smooth it
        let smoothed = controller.update_ewma(2000, 50);
        // After 0.5 halflife, weight should be ~0.707
        // Expected: 0.707 * 2000 + 0.293 * 8000 ≈ 3758
        assert!(smoothed > 3500 && smoothed < 4000);
        
        // After full halflife
        let smoothed = controller.update_ewma(2000, 100);
        // After 1 halflife from previous, weight should be ~0.5
        // But we need to account for the previous smoothing
        assert!(smoothed > 2500 && smoothed < 3500);
    }
    
    #[test]
    fn test_dead_zone_detection() {
        let controller = HysteresisController::new(default_weights()).unwrap();
        
        assert!(controller.is_in_dead_zone()); // Starts at 5000, which is in dead zone
        
        let mut controller_high = controller.clone();
        controller_high.stress_ewma = 8500;
        assert!(!controller_high.is_in_dead_zone());
        
        let mut controller_low = controller.clone();
        controller_low.stress_ewma = 2500;
        assert!(!controller_low.is_in_dead_zone());
    }
}