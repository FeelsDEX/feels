/// 3D geometry operations for the physics model

use feels_types::{Vector3D, FeelsResult, FeelsProtocolError};
use feels_types::math::{Hessian3D, Position3D};

// ============================================================================
// 3D Vector Operations
// ============================================================================

/// Calculate the angle between two 3D vectors in radians
pub fn angle_between(v1: &Vector3D, v2: &Vector3D) -> FeelsResult<f64> {
    let dot_product = v1.dot(v2);
    let mag1 = v1.magnitude();
    let mag2 = v2.magnitude();
    
    if mag1 == 0.0 || mag2 == 0.0 {
        return Err(FeelsProtocolError::InvalidMathOperation {
            operation: "angle_between".to_string(),
            reason: "Cannot calculate angle with zero-magnitude vector".to_string(),
        });
    }
    
    let cos_angle = dot_product / (mag1 * mag2);
    // Clamp to [-1, 1] to handle floating point errors
    let cos_angle_clamped = cos_angle.max(-1.0).min(1.0);
    
    Ok(cos_angle_clamped.acos())
}

/// Project vector a onto vector b
pub fn project_onto(a: &Vector3D, b: &Vector3D) -> FeelsResult<Vector3D> {
    let b_mag_sq = b.magnitude_squared();
    
    if b_mag_sq == 0.0 {
        return Err(FeelsProtocolError::InvalidMathOperation {
            operation: "project_onto".to_string(),
            reason: "Cannot project onto zero vector".to_string(),
        });
    }
    
    let scalar = a.dot(b) / b_mag_sq;
    Ok(*b * scalar)
}

/// Get the component of vector a perpendicular to vector b
pub fn perpendicular_component(a: &Vector3D, b: &Vector3D) -> FeelsResult<Vector3D> {
    let projection = project_onto(a, b)?;
    Ok(*a - projection)
}

/// Calculate the triple scalar product (a · (b × c))
pub fn triple_scalar_product(a: &Vector3D, b: &Vector3D, c: &Vector3D) -> f64 {
    let cross_bc = b.cross(c);
    a.dot(&cross_bc)
}

// ============================================================================
// Coordinate System Transformations
// ============================================================================

/// Convert from Cartesian (x, y, z) to spherical coordinates (r, θ, φ)
/// Returns (radius, theta, phi) where theta is azimuthal angle, phi is polar angle
pub fn cartesian_to_spherical(pos: &Vector3D) -> (f64, f64, f64) {
    let r = pos.magnitude();
    
    if r == 0.0 {
        return (0.0, 0.0, 0.0);
    }
    
    let theta = pos.y.atan2(pos.x);  // Azimuthal angle [-π, π]
    let phi = (pos.z / r).acos();    // Polar angle [0, π]
    
    (r, theta, phi)
}

/// Convert from spherical (r, θ, φ) to Cartesian coordinates
pub fn spherical_to_cartesian(r: f64, theta: f64, phi: f64) -> Vector3D {
    let sin_phi = phi.sin();
    
    Vector3D {
        x: r * sin_phi * theta.cos(),
        y: r * sin_phi * theta.sin(),
        z: r * phi.cos(),
    }
}

/// Convert from Cartesian to cylindrical coordinates (ρ, φ, z)
pub fn cartesian_to_cylindrical(pos: &Vector3D) -> (f64, f64, f64) {
    let rho = (pos.x * pos.x + pos.y * pos.y).sqrt();
    let phi = pos.y.atan2(pos.x);
    
    (rho, phi, pos.z)
}

/// Convert from cylindrical (ρ, φ, z) to Cartesian coordinates
pub fn cylindrical_to_cartesian(rho: f64, phi: f64, z: f64) -> Vector3D {
    Vector3D {
        x: rho * phi.cos(),
        y: rho * phi.sin(),
        z,
    }
}

// ============================================================================
// Distance and Proximity Functions
// ============================================================================

/// Calculate distance from point to line defined by two points
pub fn point_to_line_distance(point: &Vector3D, line_start: &Vector3D, line_end: &Vector3D) -> f64 {
    let line_vec = *line_end - *line_start;
    let point_vec = *point - *line_start;
    
    if line_vec.magnitude_squared() == 0.0 {
        // Line is actually a point
        return point_vec.magnitude();
    }
    
    let cross_product = point_vec.cross(&line_vec);
    cross_product.magnitude() / line_vec.magnitude()
}

/// Calculate distance from point to plane defined by three points
pub fn point_to_plane_distance(
    point: &Vector3D,
    plane_p1: &Vector3D,
    plane_p2: &Vector3D,
    plane_p3: &Vector3D,
) -> FeelsResult<f64> {
    // Calculate plane normal using cross product
    let v1 = *plane_p2 - *plane_p1;
    let v2 = *plane_p3 - *plane_p1;
    let normal = v1.cross(&v2);
    
    let normal_mag = normal.magnitude();
    if normal_mag == 0.0 {
        return Err(FeelsProtocolError::InvalidMathOperation {
            operation: "point_to_plane_distance".to_string(),
            reason: "Plane points are collinear".to_string(),
        });
    }
    
    let unit_normal = normal / normal_mag;
    let point_to_plane = *point - *plane_p1;
    
    Ok(point_to_plane.dot(&unit_normal).abs())
}

/// Find the closest point on a line segment to a given point
pub fn closest_point_on_segment(
    point: &Vector3D,
    segment_start: &Vector3D,
    segment_end: &Vector3D,
) -> Vector3D {
    let segment_vec = *segment_end - *segment_start;
    let point_vec = *point - *segment_start;
    
    let segment_length_sq = segment_vec.magnitude_squared();
    
    if segment_length_sq == 0.0 {
        // Segment is actually a point
        return *segment_start;
    }
    
    // Parameter t for the closest point: P = start + t * (end - start)
    let t = point_vec.dot(&segment_vec) / segment_length_sq;
    
    // Clamp t to [0, 1] to stay within the segment
    let t_clamped = t.max(0.0).min(1.0);
    
    *segment_start + segment_vec * t_clamped
}

// ============================================================================
// Bounding Box Operations
// ============================================================================

/// Axis-aligned bounding box
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min: Vector3D,
    pub max: Vector3D,
}

impl BoundingBox {
    /// Create a new bounding box
    pub fn new(min: Vector3D, max: Vector3D) -> Self {
        Self { min, max }
    }
    
    /// Create bounding box from a set of points
    pub fn from_points(points: &[Vector3D]) -> FeelsResult<Self> {
        if points.is_empty() {
            return Err(FeelsProtocolError::InvalidParameter {
                parameter: "points".to_string(),
                value: "empty array".to_string(),
                expected: "non-empty array".to_string(),
            });
        }
        
        let mut min = points[0];
        let mut max = points[0];
        
        for point in points.iter().skip(1) {
            min = min.min(point);
            max = max.max(point);
        }
        
        Ok(Self { min, max })
    }
    
    /// Check if a point is inside the bounding box
    pub fn contains(&self, point: &Vector3D) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }
    
    /// Calculate the center of the bounding box
    pub fn center(&self) -> Vector3D {
        Vector3D {
            x: (self.min.x + self.max.x) * 0.5,
            y: (self.min.y + self.max.y) * 0.5,
            z: (self.min.z + self.max.z) * 0.5,
        }
    }
    
    /// Calculate the size (dimensions) of the bounding box
    pub fn size(&self) -> Vector3D {
        Vector3D {
            x: self.max.x - self.min.x,
            y: self.max.y - self.min.y,
            z: self.max.z - self.min.z,
        }
    }
    
    /// Calculate the volume of the bounding box
    pub fn volume(&self) -> f64 {
        let size = self.size();
        size.x * size.y * size.z
    }
    
    /// Expand the bounding box by a margin in all directions
    pub fn expand(&self, margin: f64) -> Self {
        let margin_vec = Vector3D::new(margin, margin, margin);
        Self {
            min: self.min - margin_vec,
            max: self.max + margin_vec,
        }
    }
    
    /// Check if this bounding box intersects with another
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x &&
        self.min.y <= other.max.y && self.max.y >= other.min.y &&
        self.min.z <= other.max.z && self.max.z >= other.min.z
    }
}

// ============================================================================
// Convex Hull Operations
// ============================================================================

/// Find the convex hull of a set of 3D points (simplified implementation)
/// Returns indices of points that form the convex hull
pub fn convex_hull_3d(points: &[Vector3D]) -> FeelsResult<Vec<usize>> {
    if points.len() < 4 {
        return Err(FeelsProtocolError::InvalidParameter {
            parameter: "points".to_string(),
            value: format!("{} points", points.len()),
            expected: "at least 4 points".to_string(),
        });
    }
    
    // This is a simplified implementation
    // A full 3D convex hull algorithm (like QuickHull) would be more complex
    
    // For now, return the indices of points that form the bounding box vertices
    let bbox = BoundingBox::from_points(points)?;
    let mut hull_indices = Vec::new();
    
    // Find points closest to each corner of the bounding box
    let corners = [
        Vector3D::new(bbox.min.x, bbox.min.y, bbox.min.z),
        Vector3D::new(bbox.max.x, bbox.min.y, bbox.min.z),
        Vector3D::new(bbox.min.x, bbox.max.y, bbox.min.z),
        Vector3D::new(bbox.max.x, bbox.max.y, bbox.min.z),
        Vector3D::new(bbox.min.x, bbox.min.y, bbox.max.z),
        Vector3D::new(bbox.max.x, bbox.min.y, bbox.max.z),
        Vector3D::new(bbox.min.x, bbox.max.y, bbox.max.z),
        Vector3D::new(bbox.max.x, bbox.max.y, bbox.max.z),
    ];
    
    for corner in &corners {
        let mut closest_idx = 0;
        let mut closest_dist = corner.distance_to(&points[0]);
        
        for (i, point) in points.iter().enumerate() {
            let dist = corner.distance_to(point);
            if dist < closest_dist {
                closest_dist = dist;
                closest_idx = i;
            }
        }
        
        if !hull_indices.contains(&closest_idx) {
            hull_indices.push(closest_idx);
        }
    }
    
    Ok(hull_indices)
}

// ============================================================================
// Volume and Surface Area Calculations
// ============================================================================

/// Calculate volume of tetrahedron defined by four points
pub fn tetrahedron_volume(p1: &Vector3D, p2: &Vector3D, p3: &Vector3D, p4: &Vector3D) -> f64 {
    let v1 = *p2 - *p1;
    let v2 = *p3 - *p1;
    let v3 = *p4 - *p1;
    
    let scalar_triple = triple_scalar_product(&v1, &v2, &v3);
    scalar_triple.abs() / 6.0
}

/// Calculate area of triangle defined by three points
pub fn triangle_area(p1: &Vector3D, p2: &Vector3D, p3: &Vector3D) -> f64 {
    let v1 = *p2 - *p1;
    let v2 = *p3 - *p1;
    let cross = v1.cross(&v2);
    cross.magnitude() * 0.5
}

// ============================================================================
// Interpolation Functions
// ============================================================================

/// Barycentric coordinates for a point relative to a triangle
pub fn barycentric_coordinates(
    point: &Vector3D,
    triangle_a: &Vector3D,
    triangle_b: &Vector3D,
    triangle_c: &Vector3D,
) -> FeelsResult<(f64, f64, f64)> {
    let v0 = *triangle_c - *triangle_a;
    let v1 = *triangle_b - *triangle_a;
    let v2 = *point - *triangle_a;
    
    let dot00 = v0.dot(&v0);
    let dot01 = v0.dot(&v1);
    let dot02 = v0.dot(&v2);
    let dot11 = v1.dot(&v1);
    let dot12 = v1.dot(&v2);
    
    let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
    
    if !inv_denom.is_finite() {
        return Err(FeelsProtocolError::InvalidMathOperation {
            operation: "barycentric_coordinates".to_string(),
            reason: "Triangle is degenerate".to_string(),
        });
    }
    
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    let w = 1.0 - u - v;
    
    Ok((u, v, w))
}

/// Trilinear interpolation in a 3D grid
pub fn trilinear_interpolation(
    point: &Vector3D,
    grid_origin: &Vector3D,
    grid_size: &Vector3D,
    values: &[[[f64; 2]; 2]; 2], // 2x2x2 grid of values
) -> FeelsResult<f64> {
    // Normalize point to [0, 1] range within the grid
    let t = Vector3D {
        x: (point.x - grid_origin.x) / grid_size.x,
        y: (point.y - grid_origin.y) / grid_size.y,
        z: (point.z - grid_origin.z) / grid_size.z,
    };
    
    // Clamp to [0, 1] range
    let t = Vector3D {
        x: t.x.max(0.0).min(1.0),
        y: t.y.max(0.0).min(1.0),
        z: t.z.max(0.0).min(1.0),
    };
    
    // Interpolate along x-axis for each y,z combination
    let c00 = values[0][0][0] * (1.0 - t.x) + values[1][0][0] * t.x;
    let c01 = values[0][0][1] * (1.0 - t.x) + values[1][0][1] * t.x;
    let c10 = values[0][1][0] * (1.0 - t.x) + values[1][1][0] * t.x;
    let c11 = values[0][1][1] * (1.0 - t.x) + values[1][1][1] * t.x;
    
    // Interpolate along y-axis
    let c0 = c00 * (1.0 - t.y) + c10 * t.y;
    let c1 = c01 * (1.0 - t.y) + c11 * t.y;
    
    // Interpolate along z-axis
    let result = c0 * (1.0 - t.z) + c1 * t.z;
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vector_operations() {
        let v1 = Vector3D::new(1.0, 0.0, 0.0);
        let v2 = Vector3D::new(0.0, 1.0, 0.0);
        
        // Test angle calculation
        let angle = angle_between(&v1, &v2).unwrap();
        assert!((angle - std::f64::consts::PI / 2.0).abs() < 1e-10);
        
        // Test projection
        let v3 = Vector3D::new(1.0, 1.0, 0.0);
        let proj = project_onto(&v3, &v1).unwrap();
        assert!((proj - Vector3D::new(1.0, 0.0, 0.0)).magnitude() < 1e-10);
        
        // Test perpendicular component
        let perp = perpendicular_component(&v3, &v1).unwrap();
        assert!((perp - Vector3D::new(0.0, 1.0, 0.0)).magnitude() < 1e-10);
    }
    
    #[test]
    fn test_coordinate_transformations() {
        let cartesian = Vector3D::new(1.0, 1.0, 1.0);
        
        // Test spherical conversion
        let (r, theta, phi) = cartesian_to_spherical(&cartesian);
        let back_to_cartesian = spherical_to_cartesian(r, theta, phi);
        assert!(cartesian.distance_to(&back_to_cartesian) < 1e-10);
        
        // Test cylindrical conversion
        let (rho, phi_cyl, z) = cartesian_to_cylindrical(&cartesian);
        let back_to_cartesian_cyl = cylindrical_to_cartesian(rho, phi_cyl, z);
        assert!(cartesian.distance_to(&back_to_cartesian_cyl) < 1e-10);
    }
    
    #[test]
    fn test_distance_calculations() {
        let point = Vector3D::new(1.0, 1.0, 1.0);
        let line_start = Vector3D::new(0.0, 0.0, 0.0);
        let line_end = Vector3D::new(2.0, 0.0, 0.0);
        
        let distance = point_to_line_distance(&point, &line_start, &line_end);
        // Distance should be sqrt(2) since point projects to (1,0,0) on line
        let expected = (1.0 * 1.0 + 1.0 * 1.0).sqrt();
        assert!((distance - expected).abs() < 1e-10);
    }
    
    #[test]
    fn test_bounding_box() {
        let points = vec![
            Vector3D::new(0.0, 0.0, 0.0),
            Vector3D::new(1.0, 2.0, 3.0),
            Vector3D::new(-1.0, -2.0, -3.0),
        ];
        
        let bbox = BoundingBox::from_points(&points).unwrap();
        
        assert_eq!(bbox.min, Vector3D::new(-1.0, -2.0, -3.0));
        assert_eq!(bbox.max, Vector3D::new(1.0, 2.0, 3.0));
        assert!(bbox.contains(&Vector3D::new(0.0, 0.0, 0.0)));
        assert!(!bbox.contains(&Vector3D::new(2.0, 0.0, 0.0)));
        
        let center = bbox.center();
        assert_eq!(center, Vector3D::new(0.0, 0.0, 0.0));
        
        let volume = bbox.volume();
        assert_eq!(volume, 2.0 * 4.0 * 6.0); // 2×4×6 = 48
    }
    
    #[test]
    fn test_triangle_area() {
        let p1 = Vector3D::new(0.0, 0.0, 0.0);
        let p2 = Vector3D::new(1.0, 0.0, 0.0);
        let p3 = Vector3D::new(0.0, 1.0, 0.0);
        
        let area = triangle_area(&p1, &p2, &p3);
        assert!((area - 0.5).abs() < 1e-10); // Right triangle with legs 1,1
    }
    
    #[test]
    fn test_tetrahedron_volume() {
        let p1 = Vector3D::new(0.0, 0.0, 0.0);
        let p2 = Vector3D::new(1.0, 0.0, 0.0);
        let p3 = Vector3D::new(0.0, 1.0, 0.0);
        let p4 = Vector3D::new(0.0, 0.0, 1.0);
        
        let volume = tetrahedron_volume(&p1, &p2, &p3, &p4);
        assert!((volume - 1.0/6.0).abs() < 1e-10); // Volume of unit tetrahedron
    }
}