/// Optimal path search for minimizing fees in the market physics model.
/// Uses gradient descent and A* search for finding minimum-work paths.
use anchor_lang::prelude::*;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;
use crate::state::GradientCache;
use crate::logic::core::path_integration::{PathSegment, Position3D, CellIndex3D, TradeDimension};
use crate::logic::market_physics::potential::FixedPoint;
use crate::logic::market_physics::gradient::calculate_gradient_3d;
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Maximum iterations for gradient descent
pub const MAX_GRADIENT_DESCENT_STEPS: usize = 100;

/// Convergence threshold for gradient descent
pub const GRADIENT_DESCENT_THRESHOLD: i128 = 1000; // ~1e-15

/// Maximum nodes to explore in A* search
pub const MAX_ASTAR_NODES: usize = 10000;

/// Step size for gradient descent (learning rate)
pub const GRADIENT_STEP_SIZE: i128 = (1i128 << 64) / 100; // 0.01

/// Minimum step size before termination
pub const MIN_STEP_SIZE: i128 = (1i128 << 64) / 1_000_000; // 1e-6

// ============================================================================
// Path Search Node
// ============================================================================

/// Node in A* search
#[derive(Clone, Debug)]
struct SearchNode {
    /// Current position
    position: Position3D,
    
    /// Cell index
    cell: CellIndex3D,
    
    /// Cost to reach this node (actual work done)
    g_cost: FixedPoint,
    
    /// Heuristic cost to goal
    h_cost: FixedPoint,
    
    /// Parent node for path reconstruction
    parent: Option<Box<SearchNode>>,
    
    /// Dimension used to reach this node
    dimension: TradeDimension,
}

impl SearchNode {
    /// Total cost (f = g + h)
    fn f_cost(&self) -> FixedPoint {
        self.g_cost.add(self.h_cost).unwrap_or(FixedPoint::MAX)
    }
}

impl Eq for SearchNode {}

impl PartialEq for SearchNode {
    fn eq(&self, other: &Self) -> bool {
        self.cell.spot_cell == other.cell.spot_cell &&
        self.cell.time_cell == other.cell.time_cell &&
        self.cell.leverage_cell == other.cell.leverage_cell
    }
}

impl Ord for SearchNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min heap - smaller f_cost is better
        other.f_cost().cmp(&self.f_cost())
    }
}

impl PartialOrd for SearchNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ============================================================================
// Gradient Descent Path Optimization
// ============================================================================

/// Optimize path using gradient descent
pub fn optimize_path_gradient_descent(
    from: Position3D,
    to: Position3D,
    gradient_cache: &GradientCache,
    constraints: &PathConstraints,
) -> Result<Vec<Position3D>> {
    let mut path = vec![from];
    let mut current = from;
    let mut step_size = GRADIENT_STEP_SIZE;
    
    for _ in 0..MAX_GRADIENT_DESCENT_STEPS {
        // Check if reached destination
        if position_distance(&current, &to) < constraints.arrival_tolerance {
            path.push(to);
            break;
        }
        
        // Get gradient at current position
        let gradient = get_interpolated_gradient(gradient_cache, &current)?;
        
        // Calculate descent direction (opposite of gradient for minimum)
        let direction = calculate_descent_direction(&gradient, &current, &to)?;
        
        // Take step
        let next = take_gradient_step(&current, &direction, step_size, constraints)?;
        
        // Check if made progress
        let current_dist = position_distance(&current, &to);
        let next_dist = position_distance(&next, &to);
        
        if next_dist >= current_dist {
            // Reduce step size if not making progress
            step_size = (step_size * 3) / 4;
            if step_size < MIN_STEP_SIZE {
                break;
            }
            continue;
        }
        
        // Accept step
        path.push(next);
        current = next;
        
        // Adaptive step size
        if next_dist < current_dist / 2 {
            step_size = (step_size * 5) / 4; // Increase if making good progress
        }
    }
    
    // Ensure we end at destination
    if !path_ends_at(&path, &to, constraints.arrival_tolerance) {
        path.push(to);
    }
    
    Ok(path)
}

/// Calculate descent direction combining gradient and goal direction
fn calculate_descent_direction(
    gradient: &Gradient3D,
    current: &Position3D,
    goal: &Position3D,
) -> Result<Direction3D> {
    // Negative gradient (descent direction)
    let grad_dir = Direction3D {
        ds: -gradient.dV_dS as i128,
        dt: -gradient.dV_dT as i128,
        dl: -gradient.dV_dL as i128,
    };
    
    // Direction to goal
    let goal_dir = Direction3D {
        ds: goal.s.value - current.s.value,
        dt: goal.t.value - current.t.value,
        dl: goal.l.value - current.l.value,
    };
    
    // Weighted combination: mostly gradient with some goal bias
    let alpha = FixedPoint::from_scaled((9 * FixedPoint::SCALE) / 10); // 0.9
    let beta = FixedPoint::from_scaled(FixedPoint::SCALE / 10); // 0.1
    
    Ok(Direction3D {
        ds: (alpha.mul_i128(grad_dir.ds)? + beta.mul_i128(goal_dir.ds)?).value,
        dt: (alpha.mul_i128(grad_dir.dt)? + beta.mul_i128(goal_dir.dt)?).value,
        dl: (alpha.mul_i128(grad_dir.dl)? + beta.mul_i128(goal_dir.dl)?).value,
    })
}

/// Take a step in the given direction
fn take_gradient_step(
    current: &Position3D,
    direction: &Direction3D,
    step_size: i128,
    constraints: &PathConstraints,
) -> Result<Position3D> {
    // Normalize direction
    let norm = direction.norm()?;
    if norm == 0 {
        return Ok(*current);
    }
    
    let normalized = direction.normalize(norm)?;
    
    // Calculate new position
    let mut next = Position3D {
        s: FixedPoint::from_scaled(current.s.value + (step_size * normalized.ds) / FixedPoint::SCALE),
        t: FixedPoint::from_scaled(current.t.value + (step_size * normalized.dt) / FixedPoint::SCALE),
        l: FixedPoint::from_scaled(current.l.value + (step_size * normalized.dl) / FixedPoint::SCALE),
    };
    
    // Apply constraints
    next = apply_position_constraints(next, constraints)?;
    
    Ok(next)
}

// ============================================================================
// A* Path Search
// ============================================================================

/// Find optimal path using A* search through tick cells
pub fn find_optimal_path_astar(
    from: Position3D,
    to: Position3D,
    gradient_cache: &GradientCache,
    tick_spacing: i32,
) -> Result<Vec<PathSegment>> {
    let start_cell = position_to_cell(&from, tick_spacing)?;
    let goal_cell = position_to_cell(&to, tick_spacing)?;
    
    // Priority queue for nodes to explore
    let mut open_set = BinaryHeap::new();
    let mut closed_set = HashSet::new();
    let mut best_costs: HashMap<CellIndex3D, FixedPoint> = HashMap::new();
    
    // Start node
    let start_node = SearchNode {
        position: from,
        cell: start_cell,
        g_cost: FixedPoint::ZERO,
        h_cost: estimate_work_heuristic(&from, &to, gradient_cache)?,
        parent: None,
        dimension: TradeDimension::Mixed,
    };
    
    open_set.push(start_node);
    best_costs.insert(start_cell, FixedPoint::ZERO);
    
    let mut nodes_explored = 0;
    
    while let Some(current) = open_set.pop() {
        nodes_explored += 1;
        if nodes_explored > MAX_ASTAR_NODES {
            break;
        }
        
        // Check if reached goal
        if current.cell == goal_cell {
            return reconstruct_path(current, to);
        }
        
        // Skip if already explored
        if !closed_set.insert(current.cell) {
            continue;
        }
        
        // Explore neighbors
        for neighbor in get_cell_neighbors(&current.cell, tick_spacing)? {
            let (next_cell, next_pos, dimension) = neighbor;
            
            // Calculate cost to reach neighbor
            let segment = PathSegment {
                start: current.position,
                end: next_pos,
                cell_index: current.cell,
                dimension,
            };
            
            let segment_work = estimate_segment_work(&segment, gradient_cache)?;
            let new_g_cost = current.g_cost.add(segment_work)?;
            
            // Check if this is a better path to the neighbor
            if let Some(&best_cost) = best_costs.get(&next_cell) {
                if new_g_cost >= best_cost {
                    continue;
                }
            }
            
            best_costs.insert(next_cell, new_g_cost);
            
            // Create neighbor node
            let neighbor_node = SearchNode {
                position: next_pos,
                cell: next_cell,
                g_cost: new_g_cost,
                h_cost: estimate_work_heuristic(&next_pos, &to, gradient_cache)?,
                parent: Some(Box::new(current.clone())),
                dimension,
            };
            
            open_set.push(neighbor_node);
        }
    }
    
    // If no path found, return direct path
    Ok(vec![PathSegment {
        start: from,
        end: to,
        cell_index: start_cell,
        dimension: TradeDimension::Mixed,
    }])
}

/// Reconstruct path from A* search result
fn reconstruct_path(
    final_node: SearchNode,
    destination: Position3D,
) -> Result<Vec<PathSegment>> {
    let mut segments = Vec::new();
    let mut current = Some(Box::new(final_node));
    
    // Build path backwards
    let mut positions = vec![destination];
    while let Some(node) = current {
        positions.push(node.position);
        current = node.parent;
    }
    positions.reverse();
    
    // Convert to segments
    for i in 0..positions.len() - 1 {
        segments.push(PathSegment {
            start: positions[i],
            end: positions[i + 1],
            cell_index: position_to_cell(&positions[i], 10)?, // Default tick spacing
            dimension: TradeDimension::Mixed,
        });
    }
    
    Ok(segments)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 3D direction vector
#[derive(Clone, Copy, Debug)]
struct Direction3D {
    ds: i128,
    dt: i128,
    dl: i128,
}

impl Direction3D {
    /// Calculate norm (magnitude)
    fn norm(&self) -> Result<i128> {
        let sum_sq = (self.ds / 1000).saturating_pow(2) +
                     (self.dt / 1000).saturating_pow(2) +
                     (self.dl / 1000).saturating_pow(2);
        
        Ok(crate::utils::math::sqrt_u64(sum_sq as u64)? as i128 * 1000)
    }
    
    /// Normalize to unit vector
    fn normalize(&self, norm: i128) -> Result<Self> {
        if norm == 0 {
            return Ok(*self);
        }
        
        Ok(Self {
            ds: (self.ds * FixedPoint::SCALE) / norm,
            dt: (self.dt * FixedPoint::SCALE) / norm,
            dl: (self.dl * FixedPoint::SCALE) / norm,
        })
    }
}

/// 3D gradient (simplified)
#[derive(Clone, Copy, Debug)]
struct Gradient3D {
    dV_dS: u128,
    dV_dT: u128,
    dV_dL: u128,
}

/// Path constraints
pub struct PathConstraints {
    /// Minimum/maximum values for each dimension
    pub bounds: [(FixedPoint, FixedPoint); 3],
    
    /// Tolerance for arrival at destination
    pub arrival_tolerance: i128,
    
    /// Maximum path length
    pub max_segments: usize,
}

impl Default for PathConstraints {
    fn default() -> Self {
        Self {
            bounds: [
                (FixedPoint::from_int(1), FixedPoint::from_int(1_000_000)),
                (FixedPoint::from_int(1), FixedPoint::from_int(1_000_000)),
                (FixedPoint::from_int(1), FixedPoint::from_int(1_000_000)),
            ],
            arrival_tolerance: FixedPoint::SCALE / 1000, // 0.001
            max_segments: 100,
        }
    }
}

/// Calculate distance between positions
fn position_distance(p1: &Position3D, p2: &Position3D) -> i128 {
    let ds = (p1.s.value - p2.s.value) / 1000;
    let dt = (p1.t.value - p2.t.value) / 1000;
    let dl = (p1.l.value - p2.l.value) / 1000;
    
    ((ds.abs().pow(2) + dt.abs().pow(2) + dl.abs().pow(2)) as u64)
        .saturating_mul(1000) as i128
}

/// Check if path ends at destination
fn path_ends_at(path: &[Position3D], destination: &Position3D, tolerance: i128) -> bool {
    if let Some(last) = path.last() {
        position_distance(last, destination) <= tolerance
    } else {
        false
    }
}

/// Apply position constraints
fn apply_position_constraints(
    pos: Position3D,
    constraints: &PathConstraints,
) -> Result<Position3D> {
    Ok(Position3D {
        s: pos.s.clamp(constraints.bounds[0].0, constraints.bounds[0].1),
        t: pos.t.clamp(constraints.bounds[1].0, constraints.bounds[1].1),
        l: pos.l.clamp(constraints.bounds[2].0, constraints.bounds[2].1),
    })
}

/// Get interpolated gradient at arbitrary position
fn get_interpolated_gradient(
    cache: &GradientCache,
    position: &Position3D,
) -> Result<Gradient3D> {
    // Simplified: return gradient at nearest tick
    // In production, would do trilinear interpolation
    
    let cell = position_to_cell(position, 10)?; // Default tick spacing
    
    if let Some(gradient) = cache.get_gradient(0, cell.spot_cell) {
        Ok(Gradient3D {
            dV_dS: gradient.dV_dS,
            dV_dT: gradient.dV_dT,
            dV_dL: gradient.dV_dL,
        })
    } else {
        // Return neutral gradient if not found
        Ok(Gradient3D {
            dV_dS: 1 << 64,
            dV_dT: 1 << 64,
            dV_dL: 1 << 64,
        })
    }
}

/// Convert position to cell index
fn position_to_cell(pos: &Position3D, tick_spacing: i32) -> Result<CellIndex3D> {
    Ok(CellIndex3D {
        spot_cell: ((pos.s.to_i64() / (1 << 32)) / tick_spacing as i64) as usize,
        time_cell: ((pos.t.to_i64() / (1 << 32)) / tick_spacing as i64) as usize,
        leverage_cell: ((pos.l.to_i64() / (1 << 32)) / tick_spacing as i64) as usize,
    })
}

/// Get neighboring cells
fn get_cell_neighbors(
    cell: &CellIndex3D,
    tick_spacing: i32,
) -> Result<Vec<(CellIndex3D, Position3D, TradeDimension)>> {
    let mut neighbors = Vec::new();
    
    // Spot dimension neighbors
    if cell.spot_cell > 0 {
        let mut neighbor = *cell;
        neighbor.spot_cell -= 1;
        let pos = cell_to_position(&neighbor, tick_spacing)?;
        neighbors.push((neighbor, pos, TradeDimension::Spot));
    }
    if cell.spot_cell < crate::constant::MAX_TICKS - 1 {
        let mut neighbor = *cell;
        neighbor.spot_cell += 1;
        let pos = cell_to_position(&neighbor, tick_spacing)?;
        neighbors.push((neighbor, pos, TradeDimension::Spot));
    }
    
    // Time dimension neighbors
    if cell.time_cell > 0 {
        let mut neighbor = *cell;
        neighbor.time_cell -= 1;
        let pos = cell_to_position(&neighbor, tick_spacing)?;
        neighbors.push((neighbor, pos, TradeDimension::Time));
    }
    if cell.time_cell < crate::constant::MAX_TICKS - 1 {
        let mut neighbor = *cell;
        neighbor.time_cell += 1;
        let pos = cell_to_position(&neighbor, tick_spacing)?;
        neighbors.push((neighbor, pos, TradeDimension::Time));
    }
    
    // Leverage dimension neighbors
    if cell.leverage_cell > 0 {
        let mut neighbor = *cell;
        neighbor.leverage_cell -= 1;
        let pos = cell_to_position(&neighbor, tick_spacing)?;
        neighbors.push((neighbor, pos, TradeDimension::Leverage));
    }
    if cell.leverage_cell < crate::constant::MAX_TICKS - 1 {
        let mut neighbor = *cell;
        neighbor.leverage_cell += 1;
        let pos = cell_to_position(&neighbor, tick_spacing)?;
        neighbors.push((neighbor, pos, TradeDimension::Leverage));
    }
    
    Ok(neighbors)
}

/// Convert cell to position (center of cell)
fn cell_to_position(cell: &CellIndex3D, tick_spacing: i32) -> Result<Position3D> {
    Ok(Position3D {
        s: FixedPoint::from_int((cell.spot_cell as i64 * tick_spacing as i64) + tick_spacing as i64 / 2),
        t: FixedPoint::from_int((cell.time_cell as i64 * tick_spacing as i64) + tick_spacing as i64 / 2),
        l: FixedPoint::from_int((cell.leverage_cell as i64 * tick_spacing as i64) + tick_spacing as i64 / 2),
    })
}

/// Estimate work for a segment
fn estimate_segment_work(
    segment: &PathSegment,
    cache: &GradientCache,
) -> Result<FixedPoint> {
    // Get gradient for this segment
    if let Some(gradient) = segment.get_gradient(cache) {
        // Simplified work calculation
        let delta = segment.delta()?;
        let work = FixedPoint::from_scaled(
            ((gradient.dV_dS as i128 * delta.dS.value) +
             (gradient.dV_dT as i128 * delta.dT.value) +
             (gradient.dV_dL as i128 * delta.dL.value)) / FixedPoint::SCALE
        );
        Ok(work.abs())
    } else {
        // Default work estimate
        Ok(FixedPoint::from_int(1))
    }
}

/// Estimate work heuristic for A*
fn estimate_work_heuristic(
    from: &Position3D,
    to: &Position3D,
    cache: &GradientCache,
) -> Result<FixedPoint> {
    // Simple heuristic: distance * average gradient
    let distance = position_distance(from, to);
    let avg_gradient = FixedPoint::from_int(1); // Simplified
    
    Ok(FixedPoint::from_scaled((distance * avg_gradient.value) / FixedPoint::SCALE))
}

// ============================================================================
// Cell Index Hashing
// ============================================================================

impl std::hash::Hash for CellIndex3D {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.spot_cell.hash(state);
        self.time_cell.hash(state);
        self.leverage_cell.hash(state);
    }
}

impl Eq for CellIndex3D {}

impl PartialEq for CellIndex3D {
    fn eq(&self, other: &Self) -> bool {
        self.spot_cell == other.spot_cell &&
        self.time_cell == other.time_cell &&
        self.leverage_cell == other.leverage_cell
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_direction_normalization() {
        let dir = Direction3D {
            ds: 3 * FixedPoint::SCALE,
            dt: 4 * FixedPoint::SCALE,
            dl: 0,
        };
        
        let norm = dir.norm().unwrap();
        let normalized = dir.normalize(norm).unwrap();
        
        // Should have unit length
        let unit_norm = normalized.norm().unwrap();
        assert!((unit_norm - FixedPoint::SCALE).abs() < FixedPoint::SCALE / 100);
    }
    
    #[test]
    fn test_position_distance() {
        let p1 = Position3D {
            s: FixedPoint::from_int(100),
            t: FixedPoint::from_int(100),
            l: FixedPoint::from_int(100),
        };
        
        let p2 = Position3D {
            s: FixedPoint::from_int(103),
            t: FixedPoint::from_int(104),
            l: FixedPoint::from_int(100),
        };
        
        let dist = position_distance(&p1, &p2);
        // Should be approximately 5 (3-4-5 triangle)
        assert!(dist > 4 * FixedPoint::SCALE && dist < 6 * FixedPoint::SCALE);
    }
}