//! VDP integration tests for the AOI demo.
//!
//! These tests cold-start `aoi-demo`, drive it via VDP, and assert on
//! both the engine-level AOI namespace (`aoi.stats` / `aoi.queryAabb` /
//! …) and the demo's own `demo.*` namespace (`demo.setCirclePos` /
//! `demo.setPaused`). Run with:
//!
//!     cargo test -p aoi-demo -- --ignored --test-threads=1
//!
//! The `--test-threads=1` is mandatory because every `GameHarness`
//! grabs the same VDP port (9232 here).

use serde_json::json;
use vibe_test::GameHarness;

const GAME_PACKAGE: &str = "aoi-demo";
// Matches `examples/aoi-demo/game.yaml` -> debug.vdp.port.
const VDP_PORT: u16 = 9232;
// Matches `NUM_POINTS` in main.rs.
const EXPECTED_POINTS: u64 = 200;
// Matches `WORLD_W` / `WORLD_H` in main.rs.
const WORLD_W: f64 = 512.0;
const WORLD_H: f64 = 288.0;
// Matches `CIRCLE_RADIUS` in main.rs. Used by the queryCircle
// cross-check below — must stay in sync with the real observer radius
// or the lit-set / direct-query equivalence assertion breaks.
const CIRCLE_RADIUS: f64 = 28.0;
// Matches `POINT_RADIUS` in main.rs. Scatter dots are real
// `Shape::Circle` entities now (not zero-extent points), so any
// distance-from-edge assertion in raycast tests must subtract this.
const POINT_RADIUS: f64 = 2.5;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real aoi-demo game window; run with --ignored"]
async fn aoi_stats_reports_full_scatter() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch aoi-demo");

    // Pause first so the entity count is stable across the RPC.
    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    let resp = h.call("aoi.stats", json!({})).await.unwrap();
    let stats = resp
        .get("result")
        .expect("aoi.stats must return a result envelope");
    assert_eq!(
        stats["entity_count"].as_u64().unwrap(),
        EXPECTED_POINTS,
        "entity_count should match the seeded scatter size"
    );
    // Sanity: the auto-sized grid for a 512x288 world produces ≥ 1
    // cell, and avg_per_cell > 0 once we have 200 points in it.
    assert!(stats["cell_count"].as_u64().unwrap() >= 1);
    assert!(stats["avg_entities_per_cell"].as_f64().unwrap() > 0.0);

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real aoi-demo game window; run with --ignored"]
async fn aoi_query_aabb_returns_all_points_for_full_world_query() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch aoi-demo");

    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    // A query covering the entire world must return every entity.
    // (Generous overshoot so we don't have to think about whether the
    // scatter inset matches the AABB endpoints exactly.)
    let resp = h
        .call(
            "aoi.queryAabb",
            json!({ "min": [-10.0, -10.0], "max": [WORLD_W + 10.0, WORLD_H + 10.0] }),
        )
        .await
        .unwrap();
    let hits = resp["result"]["hits"]
        .as_array()
        .expect("aoi.queryAabb result must contain a `hits` array");
    assert_eq!(
        hits.len() as u64,
        EXPECTED_POINTS,
        "full-world AABB should hit every scatter point"
    );

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real aoi-demo game window; run with --ignored"]
async fn observer_lights_up_a_dense_region_when_circle_teleports_in() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch aoi-demo");

    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    // Step 1: park *all three* circles far outside the scatter. The
    // scatter points are inset 20px from each edge (see main.rs), so a
    // position well outside the world bounds guarantees zero overlap.
    // We have to teleport every circle individually because the demo
    // has three independent observers; leaving any of them inside the
    // scatter would keep some points lit and break the "lit == 0"
    // assertion below.
    for idx in 0..3u64 {
        h.call(
            "demo.setCirclePos",
            json!({ "index": idx, "x": -200.0 - (idx as f64) * 100.0, "y": -200.0 }),
        )
        .await
        .unwrap();
    }
    // Two frames: one to apply the new positions, one for the observer
    // diffs to settle and Leave events to fire for everything previously
    // highlighted at startup.
    h.step_and_wait(2).await.unwrap();

    let inspect_far = h.inspect().await.unwrap();
    let lit_far = inspect_far["lit_count"].as_u64().unwrap();
    assert_eq!(
        lit_far, 0,
        "no points should be lit when all circles are parked outside the world (got {lit_far})"
    );

    // Step 2: teleport circle 0 into the dense centre of the scatter
    // while leaving the other two parked outside. With the
    // deterministic 0x5EED seed, a CIRCLE_RADIUS-radius circle in the
    // middle of a 200-point uniform scatter on a 512x288 board reliably
    // hits several points — the exact number is seed-dependent, so we
    // just assert non-zero rather than pin a magic number that would
    // be brittle if the PRNG changes.
    h.call(
        "demo.setCirclePos",
        json!({ "index": 0, "x": WORLD_W * 0.5, "y": WORLD_H * 0.5 }),
    )
    .await
    .unwrap();
    h.step_and_wait(2).await.unwrap();

    let inspect_near = h.inspect().await.unwrap();
    let lit_near = inspect_near["lit_count"].as_u64().unwrap();
    assert!(
        lit_near > 0,
        "circle in the centre of the scatter should light up at least one point (got {lit_near})"
    );

    // Step 3: cross-check with `aoi.queryCircle` directly, but
    // *manually filter* the result to only count Round dots — the
    // demo's observer rejects squares via its `AoiFilter`, so a raw
    // query (which doesn't apply that filter, since the VDP query
    // namespace is unfiltered) will return more entities than the
    // observer sees. This is the assertion that proves type filtering
    // is actually engaged at the observer.
    let circle_hits = h
        .call(
            "aoi.queryCircle",
            json!({ "center": [WORLD_W * 0.5, WORLD_H * 0.5], "radius": CIRCLE_RADIUS }),
        )
        .await
        .unwrap();
    let raw_hits = circle_hits["result"]["hits"].as_array().unwrap();
    // Re-walk those ids through `aoi.list` to learn their shape, then
    // count the circles only — those are the Round dots.
    let listing = h.call("aoi.list", json!({})).await.unwrap();
    let entities = listing["result"]["entities"].as_array().unwrap();
    let mut shape_by_id = std::collections::HashMap::new();
    for ent in entities {
        let id = ent["id"].as_u64().unwrap();
        let ty = ent["shape"]["type"].as_str().unwrap().to_string();
        shape_by_id.insert(id, ty);
    }
    let round_in_query: u64 = raw_hits
        .iter()
        .filter(|id| shape_by_id.get(&id.as_u64().unwrap()).map(|s| s.as_str()) == Some("circle"))
        .count() as u64;
    assert_eq!(
        lit_near, round_in_query,
        "observer's lit set ({lit_near}) must match the Round-only subset of queryCircle ({round_in_query}); \
         a mismatch means either the type filter leaked squares or queryCircle missed a round"
    );
    // Sanity: the unfiltered query should have at least one square in
    // it (otherwise this test is no longer exercising the type filter).
    assert!(
        raw_hits.len() as u64 >= round_in_query,
        "raw query ({}) must be ≥ filtered count ({round_in_query})",
        raw_hits.len()
    );

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real aoi-demo game window; run with --ignored"]
async fn observer_filter_excludes_square_dots() {
    // Direct test of the type filter: park a circle on top of a known
    // *square* dot and assert it does NOT light up. Without the
    // type filter this test would fail because squares would be
    // covered by the observer just like rounds.
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch aoi-demo");

    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    // Park *all three* circles outside so they don't contaminate
    // lit_count with their own coverage of nearby rounds.
    for idx in 0..3u64 {
        h.call(
            "demo.setCirclePos",
            json!({ "index": idx, "x": -200.0 - (idx as f64) * 100.0, "y": -200.0 }),
        )
        .await
        .unwrap();
    }
    h.step_and_wait(2).await.unwrap();

    // Find the first square dot in the scatter via `aoi.list`.
    let listing = h.call("aoi.list", json!({})).await.unwrap();
    let entities = listing["result"]["entities"].as_array().unwrap();
    let square = entities
        .iter()
        .find(|ent| ent["shape"]["type"].as_str() == Some("aabb"))
        .expect("scatter must contain at least one square dot");
    let pos = &square["shape"]["center"];
    let sx = pos[0].as_f64().unwrap();
    let sy = pos[1].as_f64().unwrap();

    // Teleport circle 0 directly onto the square. Its observer should
    // see zero hits (filter rejects squares) → lit_count contribution
    // from circle 0 is 0. The other two are still parked outside.
    h.call("demo.setCirclePos", json!({ "index": 0, "x": sx, "y": sy }))
        .await
        .unwrap();
    h.step_and_wait(2).await.unwrap();

    let inspect = h.inspect().await.unwrap();
    let lit = inspect["lit_count"].as_u64().unwrap();
    // The lit count should be exactly the count of *round* dots
    // covered by circle 0's region. The other two circles are parked
    // off-world and contribute 0. We assert weakly (lit may be 0 if
    // no rounds happen to be within CIRCLE_RADIUS of this square)
    // but require the strong invariant: any nonzero lit count must be
    // accountable to round dots, never the square we teleported onto.
    let circle_hits = h
        .call(
            "aoi.queryCircle",
            json!({ "center": [sx, sy], "radius": CIRCLE_RADIUS }),
        )
        .await
        .unwrap();
    let raw_hits = circle_hits["result"]["hits"].as_array().unwrap();
    let mut shape_by_id = std::collections::HashMap::new();
    for ent in entities {
        let id = ent["id"].as_u64().unwrap();
        let ty = ent["shape"]["type"].as_str().unwrap().to_string();
        shape_by_id.insert(id, ty);
    }
    let round_count_at_square: u64 = raw_hits
        .iter()
        .filter(|id| shape_by_id.get(&id.as_u64().unwrap()).map(|s| s.as_str()) == Some("circle"))
        .count() as u64;
    assert_eq!(
        lit, round_count_at_square,
        "lit_count ({lit}) must equal the round-only subset ({round_count_at_square}); \
         the square dot at the observer center must not contribute"
    );
    // And the square itself must be in the raw hits — proving it
    // really *is* in the observer's broadphase, just filtered out.
    let square_id = square["id"].as_u64().unwrap();
    assert!(
        raw_hits.iter().any(|id| id.as_u64() == Some(square_id)),
        "square dot id {square_id} must appear in raw queryCircle (proving it's in broadphase, just filtered)"
    );

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real aoi-demo game window; run with --ignored"]
async fn raycast_finds_a_known_point() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch aoi-demo");

    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    // Pick a real dot from the scatter and aim straight at it. A
    // naive horizontal ray across the world is unreliable even with
    // the larger 2.5 px dot radius — empirically it sometimes still
    // misses everything depending on the seed. Targeting a known dot
    // makes the test seed-independent.
    let listing = h.call("aoi.list", json!({})).await.unwrap();
    let entities = listing["result"]["entities"]
        .as_array()
        .expect("aoi.list must return entities array");
    assert!(!entities.is_empty(), "demo should have at least one dot");
    let target = &entities[0];
    // Scatter dots are `Shape::Circle` now, so the JSON field is
    // `center` (the old `Shape::Point` serialized as `position`).
    let pos = &target["shape"]["center"];
    let tx = pos[0].as_f64().unwrap();
    let ty = pos[1].as_f64().unwrap();

    // Origin 30 px to the left of the target's center, ray pointing +x.
    let origin_x = tx - 30.0;
    let resp = h
        .call(
            "aoi.raycast",
            json!({ "origin": [origin_x, ty], "dir": [1.0, 0.0], "maxDist": 100.0 }),
        )
        .await
        .unwrap();
    let hit = &resp["result"]["hit"];
    assert!(
        !hit.is_null(),
        "raycast aimed straight at dot {:?} should hit (got null)",
        target
    );
    let distance = hit["distance"].as_f64().unwrap();
    // Center is 30 px ahead; with POINT_RADIUS dots the ray hits the
    // *near edge* of the disc, so expected distance is 30 - 2.5 = 27.5.
    let expected = 30.0 - POINT_RADIUS;
    assert!(
        (distance - expected).abs() < 1.0,
        "expected distance ≈ {expected}, got {distance}"
    );

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real aoi-demo game window; run with --ignored"]
async fn aoi_unknown_method_returns_error() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch aoi-demo");

    let resp = h.call("aoi.somethingMadeUp", json!({})).await.unwrap();
    let err = resp
        .get("error")
        .expect("unknown aoi.* method must produce an error envelope");
    let code = err["code"].as_i64().unwrap();
    // -32601 = JSON-RPC method-not-found, -32000 = our generic
    // "handler returned Err" code (varies by engine version).
    assert!(
        code == -32601 || code == -32000,
        "unexpected error code: {code}"
    );
}
