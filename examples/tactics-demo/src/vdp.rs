//! VDP serializer and dispatcher for the tactics demo. Gated by the
//! `vdp` feature so the release build (`--no-default-features`) doesn't
//! drag in `serde_json` here either.

use serde_json::{Value, json};

use crate::TacticsDemo;
use crate::model::{GridPos, PendingAction};

pub fn inspect(demo: &TacticsDemo) -> Value {
    let tiles: Vec<Vec<&'static str>> = (0..demo.map.height)
        .map(|y| {
            (0..demo.map.width)
                .map(|x| demo.map.tile(GridPos::new(x, y)).unwrap().kind.as_str())
                .collect()
        })
        .collect();

    let units: Vec<Value> = demo
        .units
        .iter()
        .map(|u| {
            json!({
                "id": u.id,
                "name": u.name,
                "class": u.class_name,
                "faction": u.faction.as_str(),
                "x": u.pos.x,
                "y": u.pos.y,
                "hp": u.hp,
                "max_hp": u.max_hp,
                "strength": u.strength,
                "skill": u.skill,
                "speed": u.speed,
                "defense": u.defense,
                "move_range": u.move_range,
                "weapon": {
                    "name": u.weapon.name,
                    "might": u.weapon.might,
                    "hit": u.weapon.hit,
                    "min_range": u.weapon.min_range,
                    "max_range": u.weapon.max_range,
                },
                "acted": u.acted,
                "alive": u.alive,
            })
        })
        .collect();

    let selected = match demo.pending_action {
        PendingAction::Selected { unit_id }
        | PendingAction::Moved { unit_id, .. }
        | PendingAction::ChoosingAttack { unit_id, .. } => Some(unit_id),
        PendingAction::None => None,
    };

    let winner = match demo.phase {
        crate::model::Phase::Victory => Some("player"),
        crate::model::Phase::Defeat => Some("enemy"),
        _ => None,
    };

    json!({
        "phase": demo.phase.as_str(),
        "turn": demo.turn,
        "selected": selected,
        "cursor": [demo.cursor.x, demo.cursor.y],
        "ai_enabled": demo.ai_enabled,
        "map": {
            "width": demo.map.width,
            "height": demo.map.height,
            "tiles": tiles,
        },
        "units": units,
        "reachable": demo.reachable.iter().map(|p| json!([p.x, p.y])).collect::<Vec<_>>(),
        "attackable": demo.attackable.iter().map(|p| json!([p.x, p.y])).collect::<Vec<_>>(),
        "winner": winner,
        "combat_log": demo.combat_log,
    })
}

fn get_u32(p: &Value, key: &str) -> Result<u32, String> {
    p.get(key)
        .and_then(|v| v.as_u64())
        .map(|n| n as u32)
        .ok_or_else(|| format!("missing u32 param `{key}`"))
}
fn get_i32(p: &Value, key: &str) -> Result<i32, String> {
    p.get(key)
        .and_then(|v| v.as_i64())
        .map(|n| n as i32)
        .ok_or_else(|| format!("missing i32 param `{key}`"))
}
fn get_pos(p: &Value) -> Result<GridPos, String> {
    Ok(GridPos::new(get_i32(p, "x")?, get_i32(p, "y")?))
}

pub fn handle(demo: &mut TacticsDemo, method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "game.reset" => {
            demo.reset_state();
            Ok(json!({"status": "ok"}))
        }
        "game.selectUnit" => {
            let id = get_u32(params, "id")?;
            demo.select_unit_action(id)?;
            Ok(json!({"status": "ok"}))
        }
        "game.moveSelected" => {
            let pos = get_pos(params)?;
            demo.move_selected_action(pos)?;
            Ok(json!({"status": "ok", "x": pos.x, "y": pos.y}))
        }
        "game.waitSelected" => {
            demo.wait_selected_action()?;
            Ok(json!({"status": "ok"}))
        }
        "game.attack" => {
            let attacker = get_u32(params, "attacker")?;
            let target = get_u32(params, "target")?;
            demo.do_attack(attacker, target)?;
            Ok(json!({"status": "ok"}))
        }
        "game.previewCombat" => {
            let attacker = get_u32(params, "attacker")?;
            let target = get_u32(params, "target")?;
            let p = demo.preview_combat(attacker, target)?;
            Ok(json!({
                "damage": p.damage,
                "hit": p.hit,
                "counter_damage": p.counter_damage,
                "counter_hit": p.counter_hit,
                "double_attack": p.double_attack,
                "counter_double": p.counter_double,
            }))
        }
        "game.endTurn" => {
            demo.end_turn_action();
            Ok(json!({"status": "ok"}))
        }
        "game.setUnitPos" => {
            let id = get_u32(params, "id")?;
            let pos = get_pos(params)?;
            let u = demo
                .units
                .iter_mut()
                .find(|u| u.id == id)
                .ok_or_else(|| format!("no unit {id}"))?;
            u.pos = pos;
            Ok(json!({"status": "ok"}))
        }
        "game.setUnitHp" => {
            let id = get_u32(params, "id")?;
            let hp = get_i32(params, "hp")?;
            let u = demo
                .units
                .iter_mut()
                .find(|u| u.id == id)
                .ok_or_else(|| format!("no unit {id}"))?;
            u.hp = hp.max(0);
            if u.hp == 0 {
                u.alive = false;
            } else if !u.alive {
                u.alive = true;
            }
            // Don't trigger victory check immediately — tests typically
            // batch HP changes and then `engine.step` to drive `update`
            // which calls `check_winner`. (Doing it here would force
            // every `setUnitHp` to also serialize the winner.)
            Ok(json!({"status": "ok", "hp": u.hp, "alive": u.alive}))
        }
        "game.setAiEnabled" => {
            let enabled = params
                .get("enabled")
                .and_then(|v| v.as_bool())
                .ok_or("missing bool param `enabled`")?;
            demo.ai_enabled = enabled;
            Ok(json!({"status": "ok", "enabled": enabled}))
        }
        _ => Err(format!("Unknown method: {method}")),
    }
}
