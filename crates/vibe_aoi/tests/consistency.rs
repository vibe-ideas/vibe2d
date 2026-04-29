//! Cross-backend consistency suite.
//!
//! Both `BruteForce` and `UniformGrid` must yield **the same set of
//! results** for any deterministic workload. The brute-force backend is
//! treated as the oracle: anything it reports is ground truth, and any
//! divergence from it is a bug in the grid backend (or, more rarely, a
//! bug in the shared `Shape::intersects` code that escaped the unit
//! tests in `shape.rs`).
//!
//! These tests are intentionally not random across CI runs — we use a
//! fixed-seed LCG so failures are reproducible without `proptest`.

use std::collections::HashSet;

use glam::Vec2;
use vibe_aoi::{AoiWorld, EntityId, Shape};

/// Tiny deterministic PRNG so we can produce a stable workload without
/// pulling in `rand` as a dev-dep. Linear congruential generator with
/// the same constants as Numerical Recipes.
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.state >> 16) as u32
    }
    fn next_f32(&mut self, lo: f32, hi: f32) -> f32 {
        let t = (self.next_u32() as f32) / (u32::MAX as f32);
        lo + t * (hi - lo)
    }
    fn next_choice(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

const WORLD: f32 = 400.0;

fn random_shape(rng: &mut Lcg) -> Shape {
    match rng.next_choice(3) {
        0 => Shape::point(Vec2::new(
            rng.next_f32(0.0, WORLD),
            rng.next_f32(0.0, WORLD),
        )),
        1 => Shape::circle(
            Vec2::new(rng.next_f32(0.0, WORLD), rng.next_f32(0.0, WORLD)),
            rng.next_f32(2.0, 30.0),
        ),
        _ => Shape::aabb(
            Vec2::new(rng.next_f32(0.0, WORLD), rng.next_f32(0.0, WORLD)),
            Vec2::new(rng.next_f32(2.0, 25.0), rng.next_f32(2.0, 25.0)),
        ),
    }
}

fn populate(world: &mut AoiWorld, count: usize, seed: u64) -> Vec<EntityId> {
    let mut rng = Lcg::new(seed);
    (0..count)
        .map(|_| world.insert(random_shape(&mut rng)))
        .collect()
}

fn as_set(ids: Vec<EntityId>) -> HashSet<EntityId> {
    ids.into_iter().collect()
}

#[test]
fn query_aabb_matches_oracle() {
    let mut bf = AoiWorld::with_bruteforce();
    let mut grid = AoiWorld::new(Vec2::splat(WORLD));
    populate(&mut bf, 200, 0xC0FFEE);
    populate(&mut grid, 200, 0xC0FFEE);

    let mut rng = Lcg::new(0xDEAD);
    for _ in 0..50 {
        let cx = rng.next_f32(-50.0, WORLD + 50.0);
        let cy = rng.next_f32(-50.0, WORLD + 50.0);
        let hx = rng.next_f32(5.0, 80.0);
        let hy = rng.next_f32(5.0, 80.0);
        let min = Vec2::new(cx - hx, cy - hy);
        let max = Vec2::new(cx + hx, cy + hy);

        let oracle = as_set(bf.query_aabb(min, max));
        let grid_hits = as_set(grid.query_aabb(min, max));
        assert_eq!(
            oracle, grid_hits,
            "query_aabb mismatch at center=({cx},{cy}) half=({hx},{hy})"
        );
    }
}

#[test]
fn query_circle_matches_oracle() {
    let mut bf = AoiWorld::with_bruteforce();
    let mut grid = AoiWorld::new(Vec2::splat(WORLD));
    populate(&mut bf, 200, 0xCAFEBABE);
    populate(&mut grid, 200, 0xCAFEBABE);

    let mut rng = Lcg::new(0xBEEF);
    for _ in 0..50 {
        let center = Vec2::new(
            rng.next_f32(-20.0, WORLD + 20.0),
            rng.next_f32(-20.0, WORLD + 20.0),
        );
        let radius = rng.next_f32(5.0, 80.0);

        let oracle = as_set(bf.query_circle(center, radius));
        let grid_hits = as_set(grid.query_circle(center, radius));
        assert_eq!(
            oracle, grid_hits,
            "query_circle mismatch at center={center:?} r={radius}"
        );
    }
}

#[test]
fn query_point_matches_oracle() {
    let mut bf = AoiWorld::with_bruteforce();
    let mut grid = AoiWorld::new(Vec2::splat(WORLD));
    // Bias toward larger shapes so query_point has something to hit
    // beyond the vanishingly rare exact-point match.
    let mut rng_setup = Lcg::new(0x1234);
    for _ in 0..150 {
        let s = if rng_setup.next_choice(2) == 0 {
            Shape::circle(
                Vec2::new(
                    rng_setup.next_f32(0.0, WORLD),
                    rng_setup.next_f32(0.0, WORLD),
                ),
                rng_setup.next_f32(5.0, 30.0),
            )
        } else {
            Shape::aabb(
                Vec2::new(
                    rng_setup.next_f32(0.0, WORLD),
                    rng_setup.next_f32(0.0, WORLD),
                ),
                Vec2::new(rng_setup.next_f32(5.0, 25.0), rng_setup.next_f32(5.0, 25.0)),
            )
        };
        bf.insert(s);
        grid.insert(s);
    }

    let mut rng = Lcg::new(0x5678);
    for _ in 0..50 {
        let p = Vec2::new(rng.next_f32(0.0, WORLD), rng.next_f32(0.0, WORLD));
        let oracle = as_set(bf.query_point(p));
        let grid_hits = as_set(grid.query_point(p));
        assert_eq!(oracle, grid_hits, "query_point mismatch at {p:?}");
    }
}

#[test]
fn updates_keep_backends_in_sync() {
    // Insert, then move things around, then query. Any divergence here
    // means the grid backend's update path (un-register old cells,
    // re-register new ones) is broken.
    let mut bf = AoiWorld::with_bruteforce();
    let mut grid = AoiWorld::new(Vec2::splat(WORLD));
    let bf_ids = populate(&mut bf, 100, 0xAAAA);
    let grid_ids = populate(&mut grid, 100, 0xAAAA);
    assert_eq!(bf_ids.len(), grid_ids.len());

    let mut rng = Lcg::new(0xBBBB);
    for _ in 0..200 {
        let i = rng.next_choice(bf_ids.len() as u32) as usize;
        let new_shape = random_shape(&mut rng);
        bf.update(bf_ids[i], new_shape);
        grid.update(grid_ids[i], new_shape);
    }

    let center = Vec2::splat(WORLD * 0.5);
    let oracle = as_set(bf.query_circle(center, 100.0));
    let grid_hits = as_set(grid.query_circle(center, 100.0));
    assert_eq!(oracle, grid_hits);
}

#[test]
fn removes_keep_backends_in_sync() {
    let mut bf = AoiWorld::with_bruteforce();
    let mut grid = AoiWorld::new(Vec2::splat(WORLD));
    let bf_ids = populate(&mut bf, 100, 0xCCCC);
    let grid_ids = populate(&mut grid, 100, 0xCCCC);

    // Remove every other entity.
    for i in (0..bf_ids.len()).step_by(2) {
        bf.remove(bf_ids[i]);
        grid.remove(grid_ids[i]);
    }
    assert_eq!(bf.len(), grid.len());

    let oracle = as_set(bf.query_aabb(Vec2::ZERO, Vec2::splat(WORLD)));
    let grid_hits = as_set(grid.query_aabb(Vec2::ZERO, Vec2::splat(WORLD)));
    assert_eq!(oracle, grid_hits);
}
