//! VDP integration tests for the tactics demo.
//!
//! Each test cold-starts `tactics-demo` via `cargo run -p tactics-demo`,
//! drives it through the WebSocket VDP server on port 9233, and asserts
//! on `game.inspect` / custom `game.*` methods. They're `#[ignore]`d
//! because they need a real window/GPU and a free port.
//!
//! Run with:
//!
//!     cargo test -p tactics-demo -- --ignored --test-threads=1

use std::time::Duration;

use serde_json::json;
use vibe_test::GameHarness;

const GAME_PACKAGE: &str = "tactics-demo";
const VDP_PORT: u16 = 9233;
// Mirrors `MAP_W` / `MAP_H` in `examples/tactics-demo/src/map.rs`.
const MAP_W: u64 = 14;
const MAP_H: u64 = 10;

fn unit_by_id(state: &serde_json::Value, id: u64) -> serde_json::Value {
    let units = state["units"].as_array().expect("units array");
    units
        .iter()
        .find(|u| u["id"].as_u64() == Some(id))
        .cloned()
        .unwrap_or_else(|| panic!("no unit with id {id}"))
}

fn faction_ids(state: &serde_json::Value, faction: &str) -> Vec<u64> {
    state["units"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|u| u["faction"].as_str() == Some(faction) && u["alive"].as_bool() == Some(true))
        .map(|u| u["id"].as_u64().unwrap())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo game window; run with --ignored"]
async fn initial_state_is_valid() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch tactics-demo");
    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    let s = h.inspect().await.unwrap();
    assert_eq!(s["phase"].as_str(), Some("player_select"));
    assert_eq!(s["turn"].as_u64(), Some(1));
    assert_eq!(s["map"]["width"].as_u64(), Some(MAP_W));
    assert_eq!(s["map"]["height"].as_u64(), Some(MAP_H));
    assert_eq!(faction_ids(&s, "player").len(), 4);
    assert_eq!(faction_ids(&s, "enemy").len(), 6);
    // First player unit (id=1) starts with positive HP and matches max_hp.
    let p1 = unit_by_id(&s, 1);
    let hp = p1["hp"].as_i64().unwrap();
    assert!(hp > 0);
    assert_eq!(hp, p1["max_hp"].as_i64().unwrap());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo game window; run with --ignored"]
async fn select_unit_exposes_reachable_tiles() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT).await.unwrap();
    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    let s = h.inspect().await.unwrap();
    let id = faction_ids(&s, "player")[0];

    h.call("game.selectUnit", json!({ "id": id }))
        .await
        .unwrap();
    h.step_and_wait(1).await.unwrap();

    let after = h.inspect().await.unwrap();
    assert_eq!(after["phase"].as_str(), Some("player_move"));
    assert_eq!(after["selected"].as_u64(), Some(id));
    let reach = after["reachable"].as_array().unwrap();
    assert!(!reach.is_empty(), "selected unit must have reachable tiles");
    // No reachable tile may sit on a wall — sanity-check via the map.
    let tiles = after["map"]["tiles"].as_array().unwrap();
    for r in reach {
        let x = r[0].as_i64().unwrap() as usize;
        let y = r[1].as_i64().unwrap() as usize;
        let kind = tiles[y].as_array().unwrap()[x].as_str().unwrap();
        assert_ne!(kind, "wall", "reachable tile {x},{y} is a wall");
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo game window; run with --ignored"]
async fn move_selected_changes_position() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT).await.unwrap();
    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    let s = h.inspect().await.unwrap();
    let id = faction_ids(&s, "player")[0];
    let before = unit_by_id(&s, id);
    let bx = before["x"].as_i64().unwrap();
    let by = before["y"].as_i64().unwrap();

    h.call("game.selectUnit", json!({ "id": id }))
        .await
        .unwrap();
    h.step_and_wait(1).await.unwrap();
    // Pick any reachable tile that isn't the unit's current cell.
    let reach = h.inspect().await.unwrap()["reachable"]
        .as_array()
        .unwrap()
        .clone();
    let dest = reach
        .iter()
        .find(|r| !(r[0].as_i64() == Some(bx) && r[1].as_i64() == Some(by)))
        .expect("expected at least one move-away destination");
    let dx = dest[0].as_i64().unwrap();
    let dy = dest[1].as_i64().unwrap();

    h.call("game.moveSelected", json!({ "x": dx, "y": dy }))
        .await
        .unwrap();
    h.step_and_wait(1).await.unwrap();

    let after = h.inspect().await.unwrap();
    assert_eq!(after["phase"].as_str(), Some("player_action"));
    let moved = unit_by_id(&after, id);
    assert_eq!(moved["x"].as_i64(), Some(dx));
    assert_eq!(moved["y"].as_i64(), Some(dy));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo game window; run with --ignored"]
async fn attack_reduces_hp_or_kills() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT).await.unwrap();
    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    let s = h.inspect().await.unwrap();
    let attacker = faction_ids(&s, "player")[0];
    let target = faction_ids(&s, "enemy")[0];

    // Stage attacker adjacent to target so the weapon range check passes.
    let t = unit_by_id(&s, target);
    let tx = t["x"].as_i64().unwrap();
    let ty = t["y"].as_i64().unwrap();
    // Place attacker one tile to the left of target — but if target is
    // on the left edge, place to the right instead.
    let (ax, ay) = if tx > 0 { (tx - 1, ty) } else { (tx + 1, ty) };
    h.call(
        "game.setUnitPos",
        json!({ "id": attacker, "x": ax, "y": ay }),
    )
    .await
    .unwrap();

    let target_hp_before = unit_by_id(&s, target)["hp"].as_i64().unwrap();
    h.call(
        "game.attack",
        json!({ "attacker": attacker, "target": target }),
    )
    .await
    .unwrap();
    h.step_and_wait(1).await.unwrap();

    let after = h.inspect().await.unwrap();
    let t_after = unit_by_id(&after, target);
    let hp_after = t_after["hp"].as_i64().unwrap();
    assert!(
        hp_after < target_hp_before || t_after["alive"].as_bool() == Some(false),
        "attack should reduce HP or kill (was {target_hp_before} now {hp_after})"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo game window; run with --ignored"]
async fn end_turn_runs_enemy_phase() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT).await.unwrap();
    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    let before_log_len = h.inspect().await.unwrap()["combat_log"]
        .as_array()
        .unwrap()
        .len();

    h.call("game.endTurn", json!({})).await.unwrap();
    // The endTurn flips phase to enemy_turn; the next frame's `update`
    // runs the AI synchronously and lands back at player_select with
    // turn += 1. Two frames is plenty of cushion.
    h.step_and_wait(2).await.unwrap();

    let after = h.inspect().await.unwrap();
    assert_eq!(after["phase"].as_str(), Some("player_select"));
    assert_eq!(after["turn"].as_u64(), Some(2));
    let after_log = after["combat_log"].as_array().unwrap();
    assert!(
        after_log.len() > before_log_len,
        "enemy phase should have appended at least one log line"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo game window; run with --ignored"]
async fn all_enemies_dead_wins() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT).await.unwrap();
    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    let s = h.inspect().await.unwrap();
    for id in faction_ids(&s, "enemy") {
        h.call("game.setUnitHp", json!({ "id": id, "hp": 0 }))
            .await
            .unwrap();
    }
    // Step so `update` runs `check_winner` and flips phase to victory.
    h.step_and_wait(1).await.unwrap();

    let after = h.inspect().await.unwrap();
    assert_eq!(after["phase"].as_str(), Some("victory"));
    assert_eq!(after["winner"].as_str(), Some("player"));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo game window; run with --ignored"]
async fn screenshot_writes_png() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT).await.unwrap();
    // Pause + step to make sure the renderer has produced at least one
    // frame; `engine.step` is the only stepping path that works once the
    // engine is paused, and `engine.step` itself rejects requests when
    // the engine is running.
    h.pause().await.unwrap();
    h.step_and_wait(2).await.unwrap();

    let path = std::env::temp_dir().join(format!(
        "tactics_demo_screenshot_{}.png",
        std::process::id()
    ));
    let path_str = path.to_string_lossy().to_string();
    let _ = std::fs::remove_file(&path);

    h.call("game.screenshot", json!({ "path": path_str }))
        .await
        .unwrap();
    // The renderer queues the read-back at the start of the next frame.
    // A few stepped frames + a short wall-clock grace period covers both
    // the GPU map and the OS-level fsync.
    h.step_and_wait(3).await.unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(meta) = std::fs::metadata(&path)
            && meta.len() > 0
        {
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!(
                "screenshot file {} never appeared / is empty",
                path.display()
            );
        }
        h.step_and_wait(1).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let _ = std::fs::remove_file(&path);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo game window; run with --ignored"]
async fn preview_combat_returns_expected_fields() {
    // game.previewCombat must work without a phase precondition (it's
    // read-only) and surface damage/hit + counter pair + double flags.
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT).await.unwrap();
    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    let s = h.inspect().await.unwrap();
    let attacker = faction_ids(&s, "player")[0];
    let target = faction_ids(&s, "enemy")[0];

    // Stage adjacency so a melee preview is meaningful (counter reachable).
    let t = unit_by_id(&s, target);
    let tx = t["x"].as_i64().unwrap();
    let ty = t["y"].as_i64().unwrap();
    let (ax, ay) = if tx > 0 { (tx - 1, ty) } else { (tx + 1, ty) };
    h.call(
        "game.setUnitPos",
        json!({ "id": attacker, "x": ax, "y": ay }),
    )
    .await
    .unwrap();

    let resp = h
        .call(
            "game.previewCombat",
            json!({ "attacker": attacker, "target": target }),
        )
        .await
        .unwrap();
    let result = resp.get("result").expect("vdp envelope must have result");
    assert!(
        result["damage"].as_i64().unwrap() >= 1,
        "damage must hit floor of 1"
    );
    let hit = result["hit"].as_i64().unwrap();
    assert!((0..=100).contains(&hit), "hit% must clamp to [0,100]");
    // Both fields exist; counter pair is symmetrical (both Some or both None).
    let cd = &result["counter_damage"];
    let ch = &result["counter_hit"];
    assert_eq!(
        cd.is_null(),
        ch.is_null(),
        "counter_damage and counter_hit must agree on presence"
    );
    assert!(result["double_attack"].is_boolean());
    assert!(result["counter_double"].is_boolean());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo game window; run with --ignored"]
async fn preview_combat_rejects_out_of_range_target() {
    // The preview itself doesn't enforce range (it just reports counter
    // = None), but it does require both unit ids to exist. Bad ids must
    // surface as an error envelope.
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT).await.unwrap();
    let resp = h
        .call(
            "game.previewCombat",
            json!({ "attacker": 9999, "target": 1 }),
        )
        .await
        .unwrap();
    assert!(
        resp.get("error").is_some(),
        "missing attacker must yield error envelope"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo game window; run with --ignored"]
async fn combat_preview_panel_renders_when_targeting() {
    // End-to-end UI flow: select → move-in-place → enterAttackTarget →
    // setCursor onto enemy → assert the `cp_*` widgets show up in
    // ui.listWidgets so we know the preview panel actually rendered.
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT).await.unwrap();
    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    let s = h.inspect().await.unwrap();
    let attacker = faction_ids(&s, "player")[0];
    let target = faction_ids(&s, "enemy")[0];
    let t = unit_by_id(&s, target);
    let tx = t["x"].as_i64().unwrap();
    let ty = t["y"].as_i64().unwrap();
    let (ax, ay) = if tx > 0 { (tx - 1, ty) } else { (tx + 1, ty) };

    h.call(
        "game.setUnitPos",
        json!({ "id": attacker, "x": ax, "y": ay }),
    )
    .await
    .unwrap();
    h.call("game.selectUnit", json!({ "id": attacker }))
        .await
        .unwrap();
    h.step_and_wait(1).await.unwrap();
    // move-in-place: dest == current pos. Allowed because reachable
    // always contains the unit's starting tile.
    h.call("game.moveSelected", json!({ "x": ax, "y": ay }))
        .await
        .unwrap();
    h.step_and_wait(1).await.unwrap();
    h.call("game.enterAttackTarget", json!({})).await.unwrap();
    h.call("game.setCursor", json!({ "x": tx, "y": ty }))
        .await
        .unwrap();
    h.step_and_wait(1).await.unwrap();

    let mid = h.inspect().await.unwrap();
    assert_eq!(mid["phase"].as_str(), Some("player_attack_target"));
    assert_eq!(
        mid["cursor"][0].as_i64(),
        Some(tx),
        "cursor x must be on target"
    );
    assert_eq!(mid["cursor"][1].as_i64(), Some(ty));

    // Preview widgets must be present.
    for id in ["cp_header", "cp_attack", "cp_counter"] {
        let w = h.find_widget(id).await.unwrap();
        assert!(
            w.is_some(),
            "preview widget `{id}` must render in PlayerAttackTarget"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo game window; run with --ignored"]
async fn enter_attack_target_rejects_outside_player_action() {
    // Brand-new game: phase is player_select, so calling enterAttackTarget
    // must error rather than silently shuffling state.
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT).await.unwrap();
    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();
    let resp = h.call("game.enterAttackTarget", json!({})).await.unwrap();
    assert!(
        resp.get("error").is_some(),
        "enterAttackTarget outside PlayerAction must error"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo game window; run with --ignored"]
async fn unknown_method_returns_error() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT).await.unwrap();
    let resp = h.call("game.somethingMadeUp", json!({})).await.unwrap();
    let err = resp
        .get("error")
        .expect("unknown game.* method must produce an error envelope");
    let code = err["code"].as_i64().unwrap();
    assert!(
        code == -32601 || code == -32000,
        "unexpected error code: {code}"
    );
}
