//! Geometric primitives and intersection tests.
//!
//! All intersection helpers are defined here so that `vibe_physics` can
//! eventually depend on `vibe_aoi` for these primitives instead of
//! duplicating them.

use glam::Vec2;

/// The geometric footprint of an entity, observer region, or query area.
///
/// Kept intentionally small — 2D pixel games rarely need OBB or polygon
/// shapes, and adding them would balloon both the API surface and the
/// intersection matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// A zero-extent point. Useful for particles, projectiles, and tile
    /// markers. Two points are considered intersecting only when exactly
    /// equal — for fuzzy point picking, query with a small circle instead.
    Point(Vec2),
    /// A circle defined by its center and radius.
    Circle { center: Vec2, radius: f32 },
    /// An axis-aligned bounding box defined by center and half-extents
    /// (so the full size is `2 * half_extents`).
    Aabb { center: Vec2, half_extents: Vec2 },
}

// Hand-rolled serialization so the wire format is a flat
// `{"type": "circle", "center": [..], "radius": ..}` instead of serde's
// default newtype-variant encoding `{"Point": [..]}`. We can't use
// `#[serde(tag = "type")]` here because newtype variants holding a
// non-map value (like `Point(Vec2)`) are rejected by serde at compile
// time when an internal tag is requested.
#[cfg(feature = "vdp")]
impl serde::Serialize for Shape {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        match *self {
            Shape::Point(p) => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("type", "point")?;
                m.serialize_entry("position", &[p.x, p.y])?;
                m.end()
            }
            Shape::Circle { center, radius } => {
                let mut m = serializer.serialize_map(Some(3))?;
                m.serialize_entry("type", "circle")?;
                m.serialize_entry("center", &[center.x, center.y])?;
                m.serialize_entry("radius", &radius)?;
                m.end()
            }
            Shape::Aabb {
                center,
                half_extents,
            } => {
                let mut m = serializer.serialize_map(Some(3))?;
                m.serialize_entry("type", "aabb")?;
                m.serialize_entry("center", &[center.x, center.y])?;
                m.serialize_entry("halfExtents", &[half_extents.x, half_extents.y])?;
                m.end()
            }
        }
    }
}

impl Shape {
    pub fn point(p: Vec2) -> Self {
        Self::Point(p)
    }

    pub fn circle(center: Vec2, radius: f32) -> Self {
        Self::Circle { center, radius }
    }

    pub fn aabb(center: Vec2, half_extents: Vec2) -> Self {
        Self::Aabb {
            center,
            half_extents,
        }
    }

    /// Conservative AABB enclosing this shape. Used by spatial backends
    /// to decide which cells the shape touches.
    pub fn aabb_bounds(&self) -> (Vec2, Vec2) {
        match *self {
            Shape::Point(p) => (p, p),
            Shape::Circle { center, radius } => {
                let r = Vec2::splat(radius);
                (center - r, center + r)
            }
            Shape::Aabb {
                center,
                half_extents,
            } => (center - half_extents, center + half_extents),
        }
    }

    /// Returns true if this shape intersects with `other`. Symmetric:
    /// `a.intersects(b) == b.intersects(a)`.
    pub fn intersects(&self, other: &Shape) -> bool {
        match (*self, *other) {
            (Shape::Point(a), Shape::Point(b)) => a == b,
            (Shape::Point(p), Shape::Circle { center, radius })
            | (Shape::Circle { center, radius }, Shape::Point(p)) => {
                point_in_circle(p, center, radius)
            }
            (
                Shape::Point(p),
                Shape::Aabb {
                    center,
                    half_extents,
                },
            )
            | (
                Shape::Aabb {
                    center,
                    half_extents,
                },
                Shape::Point(p),
            ) => point_in_aabb(p, center, half_extents),
            (
                Shape::Circle {
                    center: c1,
                    radius: r1,
                },
                Shape::Circle {
                    center: c2,
                    radius: r2,
                },
            ) => circle_vs_circle(c1, r1, c2, r2),
            (
                Shape::Aabb {
                    center: c1,
                    half_extents: he1,
                },
                Shape::Aabb {
                    center: c2,
                    half_extents: he2,
                },
            ) => aabb_vs_aabb(c1, he1, c2, he2),
            (
                Shape::Circle { center: cc, radius },
                Shape::Aabb {
                    center: ac,
                    half_extents,
                },
            )
            | (
                Shape::Aabb {
                    center: ac,
                    half_extents,
                },
                Shape::Circle { center: cc, radius },
            ) => circle_vs_aabb(cc, radius, ac, half_extents),
        }
    }

    /// Returns true if `point` lies within this shape (inclusive on the
    /// boundary). Equivalent to `self.intersects(&Shape::Point(point))`.
    pub fn contains_point(&self, point: Vec2) -> bool {
        match *self {
            Shape::Point(p) => p == point,
            Shape::Circle { center, radius } => point_in_circle(point, center, radius),
            Shape::Aabb {
                center,
                half_extents,
            } => point_in_aabb(point, center, half_extents),
        }
    }
}

fn point_in_circle(p: Vec2, center: Vec2, radius: f32) -> bool {
    (p - center).length_squared() <= radius * radius
}

fn point_in_aabb(p: Vec2, center: Vec2, half_extents: Vec2) -> bool {
    let d = (p - center).abs();
    d.x <= half_extents.x && d.y <= half_extents.y
}

fn circle_vs_circle(c1: Vec2, r1: f32, c2: Vec2, r2: f32) -> bool {
    let r = r1 + r2;
    (c1 - c2).length_squared() <= r * r
}

fn aabb_vs_aabb(c1: Vec2, he1: Vec2, c2: Vec2, he2: Vec2) -> bool {
    let d = (c1 - c2).abs();
    let r = he1 + he2;
    d.x <= r.x && d.y <= r.y
}

fn circle_vs_aabb(circle_center: Vec2, radius: f32, box_center: Vec2, half_extents: Vec2) -> bool {
    // Closest point on AABB to circle center.
    let d = circle_center - box_center;
    let clamped = d.clamp(-half_extents, half_extents);
    let closest = box_center + clamped;
    (circle_center - closest).length_squared() <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_self_intersects() {
        let p = Shape::point(Vec2::new(1.0, 2.0));
        assert!(p.intersects(&p));
    }

    #[test]
    fn distinct_points_do_not_intersect() {
        let a = Shape::point(Vec2::ZERO);
        let b = Shape::point(Vec2::new(0.001, 0.0));
        assert!(!a.intersects(&b));
    }

    #[test]
    fn circle_circle_overlap() {
        let a = Shape::circle(Vec2::ZERO, 5.0);
        let b = Shape::circle(Vec2::new(7.0, 0.0), 3.0);
        assert!(a.intersects(&b));
    }

    #[test]
    fn circle_circle_disjoint() {
        let a = Shape::circle(Vec2::ZERO, 5.0);
        let b = Shape::circle(Vec2::new(20.0, 0.0), 3.0);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn aabb_touching_at_edge_intersects() {
        // Edge contact is intentionally inclusive: a tile-based game where
        // two unit cells share an edge should report a hit.
        let a = Shape::aabb(Vec2::ZERO, Vec2::splat(1.0));
        let b = Shape::aabb(Vec2::new(2.0, 0.0), Vec2::splat(1.0));
        assert!(a.intersects(&b));
    }

    #[test]
    fn circle_aabb_corner_case() {
        // Circle just barely overlapping the corner of a box.
        let circle = Shape::circle(Vec2::new(2.0, 2.0), 1.5);
        let aabb = Shape::aabb(Vec2::ZERO, Vec2::splat(1.0));
        assert!(circle.intersects(&aabb));
    }

    #[test]
    fn intersects_is_symmetric() {
        let cases: Vec<(Shape, Shape)> = vec![
            (Shape::point(Vec2::ZERO), Shape::circle(Vec2::ZERO, 5.0)),
            (
                Shape::circle(Vec2::new(3.0, 4.0), 2.0),
                Shape::aabb(Vec2::new(5.0, 5.0), Vec2::splat(2.0)),
            ),
            (
                Shape::aabb(Vec2::ZERO, Vec2::splat(3.0)),
                Shape::aabb(Vec2::new(5.0, 0.0), Vec2::splat(2.5)),
            ),
        ];
        for (a, b) in cases {
            assert_eq!(
                a.intersects(&b),
                b.intersects(&a),
                "asymmetry: {a:?} vs {b:?}"
            );
        }
    }

    #[test]
    fn aabb_bounds_are_conservative() {
        let circle = Shape::circle(Vec2::new(10.0, 5.0), 3.0);
        let (min, max) = circle.aabb_bounds();
        assert_eq!(min, Vec2::new(7.0, 2.0));
        assert_eq!(max, Vec2::new(13.0, 8.0));
    }
}
