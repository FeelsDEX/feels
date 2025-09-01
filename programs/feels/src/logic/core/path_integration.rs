/// Path integration for the market physics model.
/// Clients compute optimal paths by integrating work along piecewise segments.
use anchor_lang::prelude::*;
use crate::state::{GradientCache, gradient_cache::{Gradient3D, Hessian3x3}};
use crate::logic::market_physics::gradient::{Position3D, PositionDelta3D};
use crate::logic::market_physics::potential::FixedPoint;
use crate::logic::market_physics::work::{calculate_linear_work, calculate_quadratic_work};
use crate::error::FeelsProtocolError;

// ============================================================================
// Path Structures
// ============================================================================

/// A single segment of a path through 3D market space
#[derive(Clone, Debug)]
pub struct PathSegment {
    /// Starting position
    pub start: Position3D,
    
    /// Ending position
    pub end: Position3D,
    
    /// Cell index in each dimension
    pub cell_index: CellIndex3D,
    
    /// Dimension being traversed (for tick boundary crossings)
    pub dimension: TradeDimension,
}

/// Cell index in 3D tick space
#[derive(Clone, Copy, Debug, Default)]
pub struct CellIndex3D {
    /// Spot dimension cell
    pub spot_cell: usize,
    
    /// Time dimension cell
    pub time_cell: usize,
    
    /// Leverage dimension cell
    pub leverage_cell: usize,
}

/// Trade dimension
#[derive(Clone, Copy, Debug)]
pub enum TradeDimension {
    Spot = 0,
    Time = 1,
    Leverage = 2,
    Mixed,
}

impl PathSegment {
    /// Calculate position delta for this segment
    pub fn delta(&self) -> Result<PositionDelta3D> {
        PositionDelta3D::between(&self.start, &self.end)
    }
    
    /// Get gradient from cache for this segment
    pub fn get_gradient<'a>(&self, cache: &'a GradientCache) -> Option<&'a Gradient3D> {
        let dim_index = match self.dimension {
            TradeDimension::Spot => 0,
            TradeDimension::Time => 1,
            TradeDimension::Leverage => 2,
            TradeDimension::Mixed => 0, // Default to spot for mixed
        };
        
        let cell_index = match self.dimension {
            TradeDimension::Spot => self.cell_index.spot_cell,
            TradeDimension::Time => self.cell_index.time_cell,
            TradeDimension::Leverage => self.cell_index.leverage_cell,
            TradeDimension::Mixed => self.cell_index.spot_cell,
        };
        
        cache.get_gradient(dim_index, cell_index)
    }
    
    /// Get Hessian from cache for this segment
    pub fn get_hessian<'a>(&self, cache: &'a GradientCache) -> Option<&'a Hessian3x3> {
        let dim_index = match self.dimension {
            TradeDimension::Spot => 0,
            TradeDimension::Time => 1,
            TradeDimension::Leverage => 2,
            TradeDimension::Mixed => 0,
        };
        
        let cell_index = match self.dimension {
            TradeDimension::Spot => self.cell_index.spot_cell,
            TradeDimension::Time => self.cell_index.time_cell,
            TradeDimension::Leverage => self.cell_index.leverage_cell,
            TradeDimension::Mixed => self.cell_index.spot_cell,
        };
        
        cache.get_hessian(dim_index, cell_index)
    }
}

// ============================================================================
// Path Integration
// ============================================================================

/// Integrate work along a path with multiple segments
pub fn integrate_path_work(
    segments: &[PathSegment],
    gradient_cache: &GradientCache,
) -> Result<FixedPoint> {
    let mut total_work = FixedPoint::ZERO;
    
    for segment in segments {
        // Get gradient and Hessian for this cell
        let gradient = segment.get_gradient(gradient_cache)
            .ok_or(FeelsProtocolError::InvalidInput)?;
        let hessian = segment.get_hessian(gradient_cache)
            .ok_or(FeelsProtocolError::InvalidInput)?;
        
        // Calculate position delta
        let delta = segment.delta()?;
        
        // Use closed-form quadratic integration
        let segment_work = calculate_quadratic_work(gradient, hessian, &delta)?;
        
        total_work = total_work.add(segment_work)?;
    }
    
    Ok(total_work)
}

/// Integrate work using only linear approximation (faster but less accurate)
pub fn integrate_path_work_linear(
    segments: &[PathSegment],
    gradient_cache: &GradientCache,
) -> Result<FixedPoint> {
    let mut total_work = FixedPoint::ZERO;
    
    for segment in segments {
        let gradient = segment.get_gradient(gradient_cache)
            .ok_or(FeelsProtocolError::InvalidInput)?;
        
        let delta = segment.delta()?;
        let segment_work = calculate_linear_work(gradient, &delta)?;
        
        total_work = total_work.add(segment_work)?;
    }
    
    Ok(total_work)
}

// ============================================================================
// Path Planning
// ============================================================================

/// Plan a path through 3D space respecting tick boundaries
pub fn plan_path_by_cells(
    from: Position3D,
    to: Position3D,
    tick_spacing: i32,
) -> Result<Vec<PathSegment>> {
    let mut segments = Vec::new();
    let mut current = from;
    
    // Determine which dimensions need traversal
    let needs_spot = (to.s.value - from.s.value).abs() > FixedPoint::SCALE / 100;
    let needs_time = (to.t.value - from.t.value).abs() > FixedPoint::SCALE / 100;
    let needs_leverage = (to.l.value - from.l.value).abs() > FixedPoint::SCALE / 100;
    
    // Simple strategy: traverse each dimension sequentially
    // In production, would optimize path based on gradients
    
    // Traverse spot dimension
    if needs_spot {
        let spot_segments = plan_dimension_traverse(
            current,
            Position3D { s: to.s, t: current.t, l: current.l },
            TradeDimension::Spot,
            tick_spacing,
        )?;
        
        for seg in spot_segments {
            current = seg.end;
            segments.push(seg);
        }
    }
    
    // Traverse time dimension
    if needs_time {
        let time_segments = plan_dimension_traverse(
            current,
            Position3D { s: current.s, t: to.t, l: current.l },
            TradeDimension::Time,
            tick_spacing,
        )?;
        
        for seg in time_segments {
            current = seg.end;
            segments.push(seg);
        }
    }
    
    // Traverse leverage dimension
    if needs_leverage {
        let leverage_segments = plan_dimension_traverse(
            current,
            to,
            TradeDimension::Leverage,
            tick_spacing,
        )?;
        
        for seg in leverage_segments {
            segments.push(seg);
        }
    }
    
    // If no movement needed, create single segment
    if segments.is_empty() {
        segments.push(PathSegment {
            start: from,
            end: to,
            cell_index: position_to_cell_index(&from, tick_spacing)?,
            dimension: TradeDimension::Mixed,
        });
    }
    
    Ok(segments)
}

/// Plan traversal along a single dimension
fn plan_dimension_traverse(
    from: Position3D,
    to: Position3D,
    dimension: TradeDimension,
    tick_spacing: i32,
) -> Result<Vec<PathSegment>> {
    let mut segments = Vec::new();
    let mut current = from;
    
    // Get start and end values for the dimension
    let (start_val, end_val) = match dimension {
        TradeDimension::Spot => (from.s.value, to.s.value),
        TradeDimension::Time => (from.t.value, to.t.value),
        TradeDimension::Leverage => (from.l.value, to.l.value),
        TradeDimension::Mixed => return Err(FeelsProtocolError::InvalidInput.into()),
    };
    
    // Determine direction
    let ascending = end_val > start_val;
    
    // Find tick boundaries to cross
    let start_tick = value_to_tick(start_val, tick_spacing)?;
    let end_tick = value_to_tick(end_val, tick_spacing)?;
    
    // Create segments for each tick boundary crossing
    let tick_range = if ascending {
        (start_tick + 1)..=end_tick
    } else {
        end_tick..=(start_tick - 1)
    };
    
    for tick in tick_range {
        let boundary_value = tick_to_value(tick, tick_spacing)?;
        
        let next = match dimension {
            TradeDimension::Spot => Position3D {
                s: FixedPoint::from_scaled(boundary_value),
                t: current.t,
                l: current.l,
            },
            TradeDimension::Time => Position3D {
                s: current.s,
                t: FixedPoint::from_scaled(boundary_value),
                l: current.l,
            },
            TradeDimension::Leverage => Position3D {
                s: current.s,
                t: current.t,
                l: FixedPoint::from_scaled(boundary_value),
            },
            _ => unreachable!(),
        };
        
        segments.push(PathSegment {
            start: current,
            end: next,
            cell_index: position_to_cell_index(&current, tick_spacing)?,
            dimension,
        });
        
        current = next;
    }
    
    // Final segment to destination
    if current.s.value != to.s.value || 
       current.t.value != to.t.value || 
       current.l.value != to.l.value {
        segments.push(PathSegment {
            start: current,
            end: to,
            cell_index: position_to_cell_index(&current, tick_spacing)?,
            dimension,
        });
    }
    
    Ok(segments)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert position to cell index
fn position_to_cell_index(position: &Position3D, tick_spacing: i32) -> Result<CellIndex3D> {
    Ok(CellIndex3D {
        spot_cell: value_to_tick(position.s.value, tick_spacing)? as usize,
        time_cell: value_to_tick(position.t.value, tick_spacing)? as usize,
        leverage_cell: value_to_tick(position.l.value, tick_spacing)? as usize,
    })
}

/// Convert fixed-point value to tick index
fn value_to_tick(value: i128, tick_spacing: i32) -> Result<i32> {
    // Simplified - in production would use proper tick math
    let normalized = value / (FixedPoint::SCALE / 1000); // Scale down
    Ok((normalized / tick_spacing as i128) as i32)
}

/// Convert tick index to fixed-point value
fn tick_to_value(tick: i32, tick_spacing: i32) -> Result<i128> {
    // Simplified - in production would use proper tick math
    Ok((tick as i128 * tick_spacing as i128) * (FixedPoint::SCALE / 1000))
}

// ============================================================================
// Path Optimization
// ============================================================================

/// Find the optimal path that minimizes work (fees)
/// This is a placeholder - full implementation would use gradient descent or A*
pub fn find_optimal_path(
    from: Position3D,
    to: Position3D,
    gradient_cache: &GradientCache,
    tick_spacing: i32,
) -> Result<Vec<PathSegment>> {
    // For now, use simple cell-by-cell traversal
    // TODO: Full implementation would:
    // 1. Use gradients to determine optimal direction
    // 2. Consider multiple paths and choose minimum work
    // 3. Apply constraints (liquidity availability, tick boundaries)
    
    plan_path_by_cells(from, to, tick_spacing)
}

/// Validate a path is continuous and valid
pub fn validate_path(segments: &[PathSegment]) -> Result<bool> {
    if segments.is_empty() {
        return Ok(false);
    }
    
    // Check continuity
    for i in 0..segments.len() - 1 {
        let end = &segments[i].end;
        let next_start = &segments[i + 1].start;
        
        let continuous = 
            (end.s.value - next_start.s.value).abs() < FixedPoint::SCALE / 1_000_000 &&
            (end.t.value - next_start.t.value).abs() < FixedPoint::SCALE / 1_000_000 &&
            (end.l.value - next_start.l.value).abs() < FixedPoint::SCALE / 1_000_000;
        
        if !continuous {
            return Ok(false);
        }
    }
    
    Ok(true)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_path_planning() {
        let from = Position3D {
            s: FixedPoint::from_int(100),
            t: FixedPoint::from_int(50),
            l: FixedPoint::from_int(10),
        };
        
        let to = Position3D {
            s: FixedPoint::from_int(120),
            t: FixedPoint::from_int(60),
            l: FixedPoint::from_int(10), // No leverage change
        };
        
        let segments = plan_path_by_cells(from, to, 10).unwrap();
        
        // Should have segments for spot and time dimensions
        assert!(segments.len() >= 2);
        
        // Validate path continuity
        assert!(validate_path(&segments).unwrap());
    }
    
    #[test]
    fn test_single_dimension_traverse() {
        let from = Position3D {
            s: FixedPoint::from_int(100),
            t: FixedPoint::from_int(50),
            l: FixedPoint::from_int(10),
        };
        
        let to = Position3D {
            s: FixedPoint::from_int(150), // Only spot changes
            t: FixedPoint::from_int(50),
            l: FixedPoint::from_int(10),
        };
        
        let segments = plan_dimension_traverse(from, to, TradeDimension::Spot, 10).unwrap();
        
        // Should create segments for each tick boundary
        assert!(segments.len() > 1);
        
        // All segments should be in spot dimension
        for seg in &segments {
            assert!(matches!(seg.dimension, TradeDimension::Spot));
        }
    }
}