//! Ray-vs-shape intersection helpers.
//!
//! Used by [`crate::AoiWorld::raycast`]. Rays are parametrized as
//! `origin + t * dir` where `dir` is normalized internally; a hit is
//! reported only when `0 <= t <= max_dist`.

use glam::Vec2;

use crate::shape::Shape;

/// Closest non-negative `t` at which `origin + t * dir_normalized`
/// intersects `shape`. Returns `None` if no hit within `max_dist`.
///
/// Caller is responsible for normalizing `dir` (we don't do it here so
/// the world can compute `dir.normalize_or_zero()` once and reuse it).
pub(crate) fn ray_vs_shape(
    origin: Vec2,
    dir_normalized: Vec2,
    max_dist: f32,
    shape: &Shape,
) -> Option<f32> {
    match *shape {
        Shape::Point(p) => ray_vs_point(origin, dir_normalized, max_dist, p),
        Shape::Circle { center, radius } => {
            ray_vs_circle(origin, dir_normalized, max_dist, center, radius)
        }
        Shape::Aabb {
            center,
            half_extents,
        } => ray_vs_aabb(origin, dir_normalized, max_dist, center, half_extents),
    }
}

/// A point hit only matches when the ray passes exactly through it.
/// Floating-point makes this almost never true; we treat points as
/// having a sub-pixel epsilon radius so `query_point`-style raycasts
/// still work for projectile-vs-particle scenarios.
fn ray_vs_point(origin: Vec2, dir: Vec2, max_dist: f32, p: Vec2) -> Option<f32> {
    const EPSILON: f32 = 0.5;
    ray_vs_circle(origin, dir, max_dist, p, EPSILON)
}

fn ray_vs_circle(origin: Vec2, dir: Vec2, max_dist: f32, center: Vec2, radius: f32) -> Option<f32> {
    // Standard analytic ray-vs-circle: solve |origin + t*dir - center|^2 = r^2
    // for the smallest non-negative t.
    let m = origin - center;
    let b = m.dot(dir);
    let c = m.length_squared() - radius * radius;
    if c > 0.0 && b > 0.0 {
        // Ray origin is outside and pointing away.
        return None;
    }
    let discr = b * b - c;
    if discr < 0.0 {
        return None;
    }
    let sqrt_d = discr.sqrt();
    // Take the near intersection. If t < 0, the origin is *inside* the
    // circle — clamp to 0 so we report a hit at the origin distance.
    let t = (-b - sqrt_d).max(0.0);
    if t <= max_dist { Some(t) } else { None }
}

fn ray_vs_aabb(
    origin: Vec2,
    dir: Vec2,
    max_dist: f32,
    center: Vec2,
    half_extents: Vec2,
) -> Option<f32> {
    // Slab method.
    let min = center - half_extents;
    let max = center + half_extents;
    let mut tmin: f32 = 0.0;
    let mut tmax: f32 = max_dist;

    for axis in 0..2 {
        let o = if axis == 0 { origin.x } else { origin.y };
        let d = if axis == 0 { dir.x } else { dir.y };
        let lo = if axis == 0 { min.x } else { min.y };
        let hi = if axis == 0 { max.x } else { max.y };

        if d.abs() < 1e-8 {
            // Ray is parallel to this slab — must already be inside.
            if o < lo || o > hi {
                return None;
            }
        } else {
            let inv_d = 1.0 / d;
            let mut t1 = (lo - o) * inv_d;
            let mut t2 = (hi - o) * inv_d;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }
    }
    Some(tmin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_misses_distant_circle() {
        // Ray going +x, circle far above — must miss.
        let hit = ray_vs_shape(
            Vec2::ZERO,
            Vec2::X,
            100.0,
            &Shape::circle(Vec2::new(50.0, 100.0), 5.0),
        );
        assert!(hit.is_none());
    }

    #[test]
    fn ray_hits_circle_directly_in_front() {
        let hit = ray_vs_shape(
            Vec2::ZERO,
            Vec2::X,
            100.0,
            &Shape::circle(Vec2::new(50.0, 0.0), 5.0),
        );
        // Near intersection at x=45.
        assert!(hit.is_some());
        let t = hit.unwrap();
        assert!((t - 45.0).abs() < 0.01, "expected ≈45, got {t}");
    }

    #[test]
    fn ray_origin_inside_circle_hits_at_zero() {
        let hit = ray_vs_shape(
            Vec2::new(50.0, 0.0),
            Vec2::X,
            100.0,
            &Shape::circle(Vec2::new(50.0, 0.0), 5.0),
        );
        assert_eq!(hit, Some(0.0));
    }

    #[test]
    fn ray_respects_max_dist() {
        let hit = ray_vs_shape(
            Vec2::ZERO,
            Vec2::X,
            10.0, // too short to reach
            &Shape::circle(Vec2::new(50.0, 0.0), 5.0),
        );
        assert!(hit.is_none());
    }

    #[test]
    fn ray_hits_aabb_face() {
        let hit = ray_vs_shape(
            Vec2::ZERO,
            Vec2::X,
            100.0,
            &Shape::aabb(Vec2::new(50.0, 0.0), Vec2::splat(5.0)),
        );
        assert_eq!(hit, Some(45.0));
    }

    #[test]
    fn ray_parallel_to_slab_misses_when_outside() {
        // Ray at y=100 going +x, AABB centered at y=0 with half-height 5.
        let hit = ray_vs_shape(
            Vec2::new(0.0, 100.0),
            Vec2::X,
            1000.0,
            &Shape::aabb(Vec2::new(50.0, 0.0), Vec2::splat(5.0)),
        );
        assert!(hit.is_none());
    }
}
