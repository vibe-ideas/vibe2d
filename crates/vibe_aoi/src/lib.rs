//! `vibe_aoi` — Area of Interest (spatial queries) for Vibe2D.
//!
//! This crate answers "who is where" questions: mouse picking, viewport
//! culling, proximity queries, enter/leave triggers and raycasts. It does
//! not perform physics resolution (velocity integration, contact solving,
//! constraints) — that responsibility belongs to `vibe_physics`, which
//! will eventually depend on this crate for broadphase and basic geometry.
//!
//! See `docs/aoi.md` for the full design rationale.
//!
//! # Quick example
//!
//! ```
//! use glam::Vec2;
//! use vibe_aoi::{AoiWorld, Shape};
//!
//! let mut aoi = AoiWorld::with_bruteforce();
//! let player = aoi.insert(Shape::circle(Vec2::ZERO, 16.0));
//! let enemy = aoi.insert(Shape::aabb(Vec2::new(50.0, 0.0), Vec2::splat(8.0)));
//!
//! // Find everyone within 100 units of the origin.
//! let nearby = aoi.query_circle(Vec2::ZERO, 100.0);
//! assert!(nearby.contains(&player));
//! assert!(nearby.contains(&enemy));
//! ```

mod bruteforce;
mod grid;
mod observer;
mod raycast;
mod shape;
mod world;

pub use observer::{AoiEvent, ObserverId};
pub use shape::Shape;
pub use world::{AoiFilter, AoiStats, AoiWorld, EntityId, RaycastHit};
