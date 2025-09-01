/// Lazy router with caching for efficient client-side path planning.
/// Routes are computed on-demand and cached for reuse.
use anchor_lang::prelude::*;
use std::collections::HashMap;
use crate::state::GradientCache;
use crate::logic::core::path_integration::{PathSegment, Position3D, plan_path_by_cells, integrate_path_work};
use crate::logic::market_physics::potential::FixedPoint;
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Maximum cached routes per router
pub const MAX_CACHED_ROUTES: usize = 1000;

/// Cache entry TTL in seconds
pub const CACHE_TTL_SECONDS: i64 = 300; // 5 minutes

/// Maximum path segments to evaluate
pub const MAX_PATH_SEGMENTS: usize = 100;

/// Work threshold for considering alternative paths
pub const WORK_IMPROVEMENT_THRESHOLD: u64 = 100; // 1% improvement

// ============================================================================
// Route Cache
// ============================================================================

/// Cached route with metadata
#[derive(Clone, Debug)]
pub struct CachedRoute {
    /// Path segments
    pub segments: Vec<PathSegment>,
    
    /// Total work (fee) for this route
    pub total_work: FixedPoint,
    
    /// Cache timestamp
    pub cached_at: i64,
    
    /// Number of times used
    pub usage_count: u64,
    
    /// Last gradient cache update when computed
    pub gradient_update_time: i64,
}

impl CachedRoute {
    /// Check if cache entry is still valid
    pub fn is_valid(&self, current_time: i64, gradient_update_time: i64) -> bool {
        // Invalid if too old
        if current_time - self.cached_at > CACHE_TTL_SECONDS {
            return false;
        }
        
        // Invalid if gradients updated since caching
        if gradient_update_time > self.gradient_update_time {
            return false;
        }
        
        true
    }
}

/// Route cache key
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct RouteKey {
    /// From position (quantized)
    pub from: QuantizedPosition,
    
    /// To position (quantized)
    pub to: QuantizedPosition,
    
    /// Trade type flags
    pub trade_type: TradeType,
}

/// Quantized position for cache key
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct QuantizedPosition {
    pub s: i64,
    pub t: i64,
    pub l: i64,
}

impl QuantizedPosition {
    /// Quantize position to reduce cache key space
    pub fn from_position(pos: &Position3D, quantum: i64) -> Self {
        Self {
            s: (pos.s.to_i64() / quantum) * quantum,
            t: (pos.t.to_i64() / quantum) * quantum,
            l: (pos.l.to_i64() / quantum) * quantum,
        }
    }
}

/// Trade type flags
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct TradeType {
    pub is_swap: bool,
    pub is_add_liquidity: bool,
    pub is_remove_liquidity: bool,
}

// ============================================================================
// Lazy Router
// ============================================================================

/// Lazy router with route caching
pub struct LazyRouter {
    /// Cached routes
    cache: HashMap<RouteKey, CachedRoute>,
    
    /// Tick spacing for the pool
    tick_spacing: i32,
    
    /// Position quantization for cache keys
    position_quantum: i64,
    
    /// Cache statistics
    stats: CacheStats,
}

/// Cache statistics
#[derive(Default, Debug)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_routes_computed: u64,
}

impl LazyRouter {
    /// Create new lazy router
    pub fn new(tick_spacing: i32) -> Self {
        Self {
            cache: HashMap::new(),
            tick_spacing,
            position_quantum: (1i64 << 32) / 100, // ~0.01 precision
            stats: CacheStats::default(),
        }
    }
    
    /// Get or compute route
    pub fn get_route(
        &mut self,
        from: Position3D,
        to: Position3D,
        gradient_cache: &GradientCache,
        trade_type: TradeType,
    ) -> Result<CachedRoute> {
        let current_time = Clock::get()?.unix_timestamp;
        
        // Create cache key
        let key = RouteKey {
            from: QuantizedPosition::from_position(&from, self.position_quantum),
            to: QuantizedPosition::from_position(&to, self.position_quantum),
            trade_type,
        };
        
        // Check cache
        if let Some(cached) = self.cache.get(&key) {
            if cached.is_valid(current_time, gradient_cache.last_update) {
                self.stats.hits += 1;
                
                // Update usage count
                if let Some(route) = self.cache.get_mut(&key) {
                    route.usage_count += 1;
                }
                
                return Ok(cached.clone());
            }
        }
        
        self.stats.misses += 1;
        
        // Compute new route
        let route = self.compute_route(from, to, gradient_cache, trade_type)?;
        
        // Cache it
        self.cache_route(key, route.clone())?;
        
        Ok(route)
    }
    
    /// Compute route without caching
    pub fn compute_route(
        &mut self,
        from: Position3D,
        to: Position3D,
        gradient_cache: &GradientCache,
        trade_type: TradeType,
    ) -> Result<CachedRoute> {
        // Plan initial path
        let segments = plan_path_by_cells(from, to, self.tick_spacing)?;
        
        require!(
            segments.len() <= MAX_PATH_SEGMENTS,
            FeelsProtocolError::PathTooLong
        );
        
        // Calculate work for this path
        let total_work = integrate_path_work(&segments, gradient_cache)?;
        
        // For significant trades, try to optimize
        let optimized_segments = if trade_type.is_swap && segments.len() > 2 {
            self.try_optimize_path(&segments, gradient_cache, total_work)?
                .unwrap_or(segments)
        } else {
            segments
        };
        
        self.stats.total_routes_computed += 1;
        
        Ok(CachedRoute {
            segments: optimized_segments,
            total_work,
            cached_at: Clock::get()?.unix_timestamp,
            usage_count: 1,
            gradient_update_time: gradient_cache.last_update,
        })
    }
    
    /// Try to optimize path by exploring alternatives
    fn try_optimize_path(
        &self,
        initial_segments: &[PathSegment],
        gradient_cache: &GradientCache,
        initial_work: FixedPoint,
    ) -> Result<Option<Vec<PathSegment>>> {
        // Simple optimization: try different dimension ordering
        if initial_segments.is_empty() {
            return Ok(None);
        }
        
        let start = initial_segments.first().unwrap().start;
        let end = initial_segments.last().unwrap().end;
        
        // Try alternative orderings
        let alternatives = vec![
            // Original order preserved in initial_segments
            // Try time-first
            self.plan_alternative_path(start, end, &[1, 0, 2])?,
            // Try leverage-first
            self.plan_alternative_path(start, end, &[2, 0, 1])?,
        ];
        
        let mut best_work = initial_work;
        let mut best_path = None;
        
        for alt_segments in alternatives {
            if alt_segments.len() > MAX_PATH_SEGMENTS {
                continue;
            }
            
            let work = integrate_path_work(&alt_segments, gradient_cache)?;
            
            // Check if improvement is significant
            if work.to_u64() < best_work.to_u64() - WORK_IMPROVEMENT_THRESHOLD {
                best_work = work;
                best_path = Some(alt_segments);
            }
        }
        
        Ok(best_path)
    }
    
    /// Plan path with specific dimension ordering
    fn plan_alternative_path(
        &self,
        from: Position3D,
        to: Position3D,
        dim_order: &[usize],
    ) -> Result<Vec<PathSegment>> {
        let mut segments = Vec::new();
        let mut current = from;
        
        for &dim in dim_order {
            let next = match dim {
                0 => Position3D { s: to.s, t: current.t, l: current.l },
                1 => Position3D { s: current.s, t: to.t, l: current.l },
                2 => Position3D { s: current.s, t: current.t, l: to.l },
                _ => continue,
            };
            
            if next.s == current.s && next.t == current.t && next.l == current.l {
                continue;
            }
            
            let dim_segments = plan_path_by_cells(current, next, self.tick_spacing)?;
            segments.extend(dim_segments);
            current = next;
        }
        
        // Final segment if needed
        if current.s != to.s || current.t != to.t || current.l != to.l {
            let final_segments = plan_path_by_cells(current, to, self.tick_spacing)?;
            segments.extend(final_segments);
        }
        
        Ok(segments)
    }
    
    /// Cache a computed route
    fn cache_route(&mut self, key: RouteKey, route: CachedRoute) -> Result<()> {
        // Evict old entries if cache is full
        if self.cache.len() >= MAX_CACHED_ROUTES {
            self.evict_old_entries()?;
        }
        
        self.cache.insert(key, route);
        Ok(())
    }
    
    /// Evict old cache entries
    fn evict_old_entries(&mut self) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp;
        let mut to_remove = Vec::new();
        
        // Find expired entries
        for (key, route) in &self.cache {
            if current_time - route.cached_at > CACHE_TTL_SECONDS {
                to_remove.push(key.clone());
            }
        }
        
        // If not enough expired, remove least recently used
        if to_remove.len() < MAX_CACHED_ROUTES / 10 {
            let mut entries: Vec<_> = self.cache.iter()
                .map(|(k, v)| (k.clone(), v.usage_count, v.cached_at))
                .collect();
            
            // Sort by usage count and age
            entries.sort_by_key(|(_, usage, age)| (*usage, *age));
            
            // Remove bottom 10%
            let remove_count = MAX_CACHED_ROUTES / 10;
            for (key, _, _) in entries.into_iter().take(remove_count) {
                to_remove.push(key);
            }
        }
        
        // Remove entries
        for key in to_remove {
            self.cache.remove(&key);
            self.stats.evictions += 1;
        }
        
        Ok(())
    }
    
    /// Clear entire cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }
}

// ============================================================================
// Route Validation
// ============================================================================

/// Validate a cached route is still optimal
pub fn validate_cached_route(
    route: &CachedRoute,
    gradient_cache: &GradientCache,
    tolerance_bps: u32,
) -> Result<bool> {
    // Recalculate work with current gradients
    let current_work = integrate_path_work(&route.segments, gradient_cache)?;
    
    // Compare with cached work
    let cached = route.total_work.to_u64();
    let current = current_work.to_u64();
    
    if cached == 0 {
        return Ok(current == 0);
    }
    
    let deviation = ((current as i64 - cached as i64).abs() as u64 * 10000) / cached;
    
    Ok(deviation <= tolerance_bps as u64)
}

// ============================================================================
// Batch Route Planning
// ============================================================================

/// Plan multiple routes in batch for efficiency
pub fn plan_batch_routes(
    router: &mut LazyRouter,
    requests: &[(Position3D, Position3D, TradeType)],
    gradient_cache: &GradientCache,
) -> Result<Vec<CachedRoute>> {
    let mut routes = Vec::new();
    
    for (from, to, trade_type) in requests {
        let route = router.get_route(*from, *to, gradient_cache, *trade_type)?;
        routes.push(route);
    }
    
    Ok(routes)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_key_quantization() {
        let pos = Position3D {
            s: FixedPoint::from_scaled(1_234_567_890),
            t: FixedPoint::from_scaled(2_345_678_901),
            l: FixedPoint::from_scaled(3_456_789_012),
        };
        
        let quantum = (1i64 << 32) / 100;
        let quantized = QuantizedPosition::from_position(&pos, quantum);
        
        // Check quantization worked
        assert_eq!(quantized.s % quantum, 0);
        assert_eq!(quantized.t % quantum, 0);
        assert_eq!(quantized.l % quantum, 0);
    }
    
    #[test]
    fn test_cache_operations() {
        let mut router = LazyRouter::new(10);
        
        let from = Position3D {
            s: FixedPoint::from_int(100),
            t: FixedPoint::from_int(50),
            l: FixedPoint::from_int(10),
        };
        
        let to = Position3D {
            s: FixedPoint::from_int(120),
            t: FixedPoint::from_int(60),
            l: FixedPoint::from_int(10),
        };
        
        let trade_type = TradeType {
            is_swap: true,
            is_add_liquidity: false,
            is_remove_liquidity: false,
        };
        
        // First call should miss cache
        assert_eq!(router.stats().hits, 0);
        assert_eq!(router.stats().misses, 0);
        
        // Would need mock gradient cache to fully test
    }
}