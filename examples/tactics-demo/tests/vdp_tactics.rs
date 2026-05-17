//! VDP integration tests for tactics-demo.
//!
//! These tests spawn a real `tactics-demo` window and drive it via VDP.
//! Heavy tests are marked `#[ignore]`. Run with:
//!
//!     cargo test -p tactics-demo -- --ignored --test-threads=1

use serde_json::json;
use vibe_test::GameHarness;

const GAME_PACKAGE: &str = "tactics-demo";
const VDP_PORT: u16 = 9233;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo window; run with --ignored"]
async fn initial_state_is_valid() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch tactics-demo");

    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    let state = h.call("game.inspect", json!({})).await.unwrap();
    let result = state.get("result").expect("inspect must return result");

    assert_eq!(result["phase"], "player_select");
    assert_eq!(result["turn"], 1);
    assert_eq!(result["map"]["width"], 14);
    assert_eq!(result["map"]["height"], 10);
    let units = result["units"].as_array().expect("units array");
    assert_eq!(units.len(), 10);
    let player_units: Vec<_> = units.iter().filter(|u| u["faction"] == "player").collect();
    let enemy_units: Vec<_> = units.iter().filter(|u| u["faction"] == "enemy").collect();
    assert_eq!(player_units.len(), 4);
    assert_eq!(enemy_units.len(), 6);
    for u in units {
        assert_eq!(u["alive"], true, "all units should start alive");
        assert_eq!(u["acted"], false, "no unit should have acted at start");
    }

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo window; run with --ignored"]
async fn select_unit_exposes_reachable_tiles() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch tactics-demo");

    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    let resp = h.call("game.selectUnit", json!({"id": 1})).await.unwrap();
    assert!(resp.get("error").is_none(), "selectUnit failed: {:?}", resp);
    h.step_and_wait(1).await.unwrap();

    let state = h.call("game.inspect", json!({})).await.unwrap();
    let result = state["result"].as_object().expect("result object");
    let reachable = result["reachable"].as_array().expect("reachable array");
    assert!(
        !reachable.is_empty(),
        "reachable should be non-empty after selecting unit 1"
    );

    let map_tiles = result["map"]["tiles"].as_array().expect("tiles");
    for pos in reachable {
        let x = pos[0].as_i64().unwrap() as usize;
        let y = pos[1].as_i64().unwrap() as usize;
        let kind = map_tiles[y][x].as_str().unwrap();
        assert_ne!(
            kind, "wall",
            "reachable tile ({},{}) should not be a wall",
            x, y
        );
    }

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo window; run with --ignored"]
async fn move_selected_changes_position() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch tactics-demo");

    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    h.call("game.selectUnit", json!({"id": 1})).await.unwrap();
    h.step_and_wait(1).await.unwrap();

    let state = h.call("game.inspect", json!({})).await.unwrap();
    let result = &state["result"];
    let reachable = result["reachable"].as_array().unwrap();
    let initial_x = result["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == 1)
        .unwrap()["x"]
        .as_i64()
        .unwrap();
    let initial_y = result["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == 1)
        .unwrap()["y"]
        .as_i64()
        .unwrap();

    let dest = reachable
        .iter()
        .find(|pos| pos[0].as_i64().unwrap() != initial_x || pos[1].as_i64().unwrap() != initial_y);
    if dest.is_none() {
        h.resume().await.unwrap();
        return;
    }
    let dest = dest.unwrap();
    let dx = dest[0].as_i64().unwrap();
    let dy = dest[1].as_i64().unwrap();

    let move_resp = h
        .call("game.moveSelected", json!({"x": dx, "y": dy}))
        .await
        .unwrap();
    assert!(
        move_resp.get("error").is_none(),
        "moveSelected failed: {:?}",
        move_resp
    );
    h.step_and_wait(1).await.unwrap();

    let state2 = h.call("game.inspect", json!({})).await.unwrap();
    let result2 = &state2["result"];
    let unit1 = result2["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == 1)
        .unwrap();
    assert_eq!(unit1["x"].as_i64().unwrap(), dx, "unit x should be {}", dx);
    assert_eq!(unit1["y"].as_i64().unwrap(), dy, "unit y should be {}", dy);
    assert_eq!(result2["phase"], "player_action");

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo window; run with --ignored"]
async fn attack_reduces_hp_or_kills() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch tactics-demo");

    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    h.call("game.setUnitPos", json!({"id": 1, "x": 2, "y": 2}))
        .await
        .unwrap();
    h.call("game.setUnitPos", json!({"id": 5, "x": 3, "y": 2}))
        .await
        .unwrap();
    h.step_and_wait(1).await.unwrap();

    let state_before = h.call("game.inspect", json!({})).await.unwrap();
    let hp_before = state_before["result"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == 5)
        .unwrap()["hp"]
        .as_i64()
        .unwrap();

    let atk_resp = h
        .call("game.attack", json!({"attacker": 1, "target": 5}))
        .await
        .unwrap();
    assert!(
        atk_resp.get("error").is_none(),
        "attack failed: {:?}",
        atk_resp
    );
    h.step_and_wait(1).await.unwrap();

    let state_after = h.call("game.inspect", json!({})).await.unwrap();
    let unit5 = state_after["result"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == 5)
        .unwrap();
    let hp_after = unit5["hp"].as_i64().unwrap();
    assert!(
        hp_after < hp_before || unit5["alive"] == false,
        "target HP should decrease or unit should die; before={} after={}",
        hp_before,
        hp_after
    );

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo window; run with --ignored"]
async fn end_turn_runs_enemy_phase() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch tactics-demo");

    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    h.call("game.endTurn", json!({})).await.unwrap();
    h.step_and_wait(2).await.unwrap();

    let state = h.call("game.inspect", json!({})).await.unwrap();
    let result = &state["result"];
    assert_eq!(result["phase"], "player_select");
    assert_eq!(
        result["turn"].as_i64().unwrap(),
        2,
        "turn should be 2 after enemy phase"
    );

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo window; run with --ignored"]
async fn all_enemies_dead_wins() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch tactics-demo");

    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    for enemy_id in 5..=10 {
        h.call("game.setUnitHp", json!({"id": enemy_id, "hp": 0}))
            .await
            .unwrap();
    }
    h.call("game.endTurn", json!({})).await.unwrap();
    h.step_and_wait(2).await.unwrap();

    let state = h.call("game.inspect", json!({})).await.unwrap();
    let result = &state["result"];
    assert_eq!(
        result["phase"], "victory",
        "phase should be victory when all enemies are dead, got: {}",
        result["phase"]
    );
    assert_eq!(result["winner"], "player");

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real tactics-demo window; run with --ignored"]
async fn unknown_method_returns_error() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch tactics-demo");

    let resp = h.call("game.doesNotExist", json!({})).await.unwrap();
    let err = resp.get("error").expect("unknown method must return error");
    let code = err["code"].as_i64().unwrap();
    assert!(
        code == -32601 || code == -32000,
        "unexpected error code: {}",
        code
    );

    h.resume().await.unwrap();
}
