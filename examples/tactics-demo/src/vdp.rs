//! VDP inspect + custom method dispatch.

#[cfg(feature = "vdp")]
use serde_json::{Value, json};

#[cfg(feature = "vdp")]
use crate::model::{Faction, GridPos, Phase, TileKind};

#[cfg(feature = "vdp")]
use crate::TacticsGame;

#[cfg(feature = "vdp")]
pub fn inspect(game: &TacticsGame) -> Value {
    let phase_str = match game.phase {
        Phase::Title => "title",
        Phase::PlayerSelect => "player_select",
        Phase::PlayerMove => "player_move",
        Phase::PlayerAction => "player_action",
        Phase::PlayerAttackTarget => "player_attack_target",
        Phase::EnemyTurn => "enemy_turn",
        Phase::Victory => "victory",
        Phase::Defeat => "defeat",
    };
    let units: Vec<Value> = game
        .units
        .iter()
        .map(|u| {
            json!({
                "id": u.id,
                "name": u.name,
                "class": u.class_name,
                "faction": if u.faction == Faction::Player { "player" } else { "enemy" },
                "x": u.pos.x,
                "y": u.pos.y,
                "hp": u.hp,
                "max_hp": u.max_hp,
                "acted": u.acted,
                "alive": u.alive,
            })
        })
        .collect();

    let tiles: Vec<Vec<&str>> = (0..game.map.height)
        .map(|y| {
            (0..game.map.width)
                .map(|x| match game.map.tile_kind(GridPos::new(x, y)) {
                    Some(TileKind::Plain) => "plain",
                    Some(TileKind::Road) => "road",
                    Some(TileKind::Forest) => "forest",
                    Some(TileKind::Fort) => "fort",
                    Some(TileKind::Wall) => "wall",
                    None => "wall",
                })
                .collect()
        })
        .collect();

    let reachable: Vec<[i32; 2]> = game.reachable.iter().map(|p| [p.x, p.y]).collect();
    let attackable: Vec<[i32; 2]> = game.attackable.iter().map(|p| [p.x, p.y]).collect();

    let winner = match game.phase {
        Phase::Victory => json!("player"),
        Phase::Defeat => json!("enemy"),
        _ => json!(null),
    };

    json!({
        "phase": phase_str,
        "turn": game.turn,
        "selected": game.selected,
        "cursor": [game.cursor.x, game.cursor.y],
        "map": {
            "width": game.map.width,
            "height": game.map.height,
            "tiles": tiles,
        },
        "units": units,
        "reachable": reachable,
        "attackable": attackable,
        "winner": winner,
        "combat_log": game.combat_log,
    })
}

#[cfg(feature = "vdp")]
pub fn handle_vdp(game: &mut TacticsGame, method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "game.inspect" => Ok(inspect(game)),
        "game.reset" => {
            game.reset_state();
            Ok(json!({"ok": true}))
        }
        "game.selectUnit" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_u64())
                .ok_or("missing id")? as u32;
            game.select_unit(id)?;
            Ok(
                json!({"ok": true, "reachable": game.reachable.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>()}),
            )
        }
        "game.moveSelected" => {
            if game.phase != Phase::PlayerMove {
                return Err(format!(
                    "not in PlayerMove phase (current: {:?})",
                    game.phase
                ));
            }
            let x = params
                .get("x")
                .and_then(|v| v.as_i64())
                .ok_or("missing x")? as i32;
            let y = params
                .get("y")
                .and_then(|v| v.as_i64())
                .ok_or("missing y")? as i32;
            let dest = GridPos::new(x, y);
            game.move_selected(dest)?;
            Ok(json!({"ok": true}))
        }
        "game.waitSelected" => {
            game.wait_selected()?;
            Ok(json!({"ok": true}))
        }
        "game.attack" => {
            let attacker_id = params
                .get("attacker")
                .and_then(|v| v.as_u64())
                .ok_or("missing attacker")? as u32;
            let target_id = params
                .get("target")
                .and_then(|v| v.as_u64())
                .ok_or("missing target")? as u32;
            game.do_attack(attacker_id, target_id)?;
            Ok(json!({"ok": true, "log": game.combat_log.last()}))
        }
        "game.previewCombat" => {
            let attacker_id = params
                .get("attacker")
                .and_then(|v| v.as_u64())
                .ok_or("missing attacker")? as u32;
            let target_id = params
                .get("target")
                .and_then(|v| v.as_u64())
                .ok_or("missing target")? as u32;
            let preview = game.preview_combat(attacker_id, target_id)?;
            Ok(json!({
                "attacker_id": preview.attacker_id,
                "defender_id": preview.defender_id,
                "atk_damage": preview.atk_damage,
                "atk_hit": preview.atk_hit,
                "def_damage": preview.def_damage,
                "def_hit": preview.def_hit,
                "atk_double": preview.atk_double,
            }))
        }
        "game.endTurn" => {
            game.end_player_turn();
            Ok(json!({"ok": true}))
        }
        "game.setUnitPos" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_u64())
                .ok_or("missing id")? as u32;
            let x = params
                .get("x")
                .and_then(|v| v.as_i64())
                .ok_or("missing x")? as i32;
            let y = params
                .get("y")
                .and_then(|v| v.as_i64())
                .ok_or("missing y")? as i32;
            let u = game
                .units
                .iter_mut()
                .find(|u| u.id == id)
                .ok_or("unit not found")?;
            u.pos = GridPos::new(x, y);
            Ok(json!({"ok": true}))
        }
        "game.setUnitHp" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_u64())
                .ok_or("missing id")? as u32;
            let hp = params
                .get("hp")
                .and_then(|v| v.as_i64())
                .ok_or("missing hp")? as i32;
            let u = game
                .units
                .iter_mut()
                .find(|u| u.id == id)
                .ok_or("unit not found")?;
            u.hp = hp.max(0);
            if u.hp <= 0 {
                u.alive = false;
                u.hp = 0;
            }
            Ok(json!({"ok": true}))
        }
        "game.setAiEnabled" => {
            let enabled = params
                .get("enabled")
                .and_then(|v| v.as_bool())
                .ok_or("missing enabled")?;
            game.ai_enabled = enabled;
            Ok(json!({"ok": true}))
        }
        _ => Err(format!("Unknown method: {}", method)),
    }
}

// Suppress unused import warning when vdp feature is off
#[cfg(not(feature = "vdp"))]
pub fn _unused() {}
