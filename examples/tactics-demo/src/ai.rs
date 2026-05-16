//! Single-frame synchronous enemy phase.
//!
//! Per-enemy decision: if any player unit is in weapon range from where I
//! stand, attack the lowest-HP one. Otherwise pick the reachable tile that
//! minimizes Manhattan distance to the nearest player unit, walk to it,
//! and attack from the new position if able. Pure greedy — no global
//! coordination — which is sufficient to make the demo feel alive without
//! turning into a tactics puzzle.

use crate::combat::{CombatLogEntry, resolve};
use crate::map::{Map, attack_targets_from, occupied_by, reachable_tiles};
use crate::model::{Faction, GridPos, Unit, UnitId};

pub fn run_enemy_turn(map: &Map, units: &mut [Unit]) -> CombatLogEntry {
    let mut log = CombatLogEntry::default();
    // Snapshot the enemy id list up front: as enemies act, the world
    // mutates underneath us and we'd otherwise risk re-acting on units
    // killed in counter-attacks earlier in the same phase.
    let enemy_ids: Vec<UnitId> = units
        .iter()
        .filter(|u| u.alive && u.faction == Faction::Enemy)
        .map(|u| u.id)
        .collect();

    for id in enemy_ids {
        // Re-fetch each iteration — preceding attacks may have killed this
        // enemy via counter, or shifted positions that affect targeting.
        let still_alive = units.iter().any(|u| u.id == id && u.alive && !u.acted);
        if !still_alive {
            continue;
        }
        run_one_enemy(map, units, id, &mut log);
    }

    // Reset acted flags for the next round (both factions); the round
    // counter is bumped by the caller.
    for u in units.iter_mut() {
        u.acted = false;
    }
    log
}

fn run_one_enemy(map: &Map, units: &mut [Unit], id: UnitId, log: &mut CombatLogEntry) {
    // 1) Try to attack from current position.
    if let Some(target_id) = best_target_from(units, id, current_pos(units, id)) {
        let line = resolve(map, units, id, target_id);
        log.lines.extend(line.lines);
        return;
    }

    // 2) Pick the best reachable tile to approach the nearest player.
    let move_target = pick_approach_tile(map, units, id);
    if let Some(dest) = move_target
        && dest != current_pos(units, id)
    {
        let enemy = units.iter_mut().find(|u| u.id == id).unwrap();
        let from = enemy.pos;
        enemy.pos = dest;
        log.lines.push(format!(
            "{} moves ({},{}) -> ({},{})",
            enemy.name, from.x, from.y, dest.x, dest.y
        ));
    }

    // 3) Try again from the new position.
    if let Some(target_id) = best_target_from(units, id, current_pos(units, id)) {
        let line = resolve(map, units, id, target_id);
        log.lines.extend(line.lines);
    } else if let Some(enemy) = units.iter_mut().find(|u| u.id == id) {
        // Mark acted even when we didn't attack so we don't loop.
        enemy.acted = true;
    }
}

fn current_pos(units: &[Unit], id: UnitId) -> GridPos {
    units
        .iter()
        .find(|u| u.id == id)
        .map(|u| u.pos)
        .unwrap_or(GridPos::new(0, 0))
}

fn best_target_from(units: &[Unit], attacker_id: UnitId, from: GridPos) -> Option<UnitId> {
    let attacker = units.iter().find(|u| u.id == attacker_id)?;
    let mut candidates: Vec<&Unit> = Vec::new();
    for p in attack_targets_from(units, attacker, from) {
        if let Some(occ) = occupied_by(units, p)
            && let Some(target) = units.iter().find(|u| u.id == occ)
            && target.faction != attacker.faction
        {
            candidates.push(target);
        }
    }
    candidates.sort_by_key(|u| u.hp);
    candidates.first().map(|u| u.id)
}

fn pick_approach_tile(map: &Map, units: &[Unit], id: UnitId) -> Option<GridPos> {
    let me = units.iter().find(|u| u.id == id)?;
    let players: Vec<&Unit> = units
        .iter()
        .filter(|u| u.alive && u.faction == Faction::Player)
        .collect();
    if players.is_empty() {
        return None;
    }
    let reach = reachable_tiles(map, units, me);
    reach.into_iter().min_by_key(|p| {
        players
            .iter()
            .map(|q| p.manhattan(q.pos))
            .min()
            .unwrap_or(i32::MAX)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Faction, GridPos, Weapon};

    fn unit(id: UnitId, faction: Faction, pos: GridPos, hp: i32) -> Unit {
        Unit {
            id,
            name: "U",
            class_name: "T",
            faction,
            pos,
            hp,
            max_hp: hp,
            strength: 5,
            skill: 5,
            speed: 5,
            defense: 0,
            move_range: 4,
            weapon: Weapon {
                name: "Sword",
                might: 5,
                hit: 100,
                min_range: 1,
                max_range: 1,
            },
            acted: false,
            alive: true,
        }
    }

    #[test]
    fn enemy_attacks_in_range_player_with_lowest_hp() {
        let map = Map::fixed_level();
        // Two adjacent players, one nearly dead.
        let mut units = vec![
            unit(1, Faction::Enemy, GridPos::new(5, 4), 20),
            unit(2, Faction::Player, GridPos::new(4, 4), 20),
            unit(3, Faction::Player, GridPos::new(6, 4), 1),
        ];
        let log = run_enemy_turn(&map, &mut units);
        assert!(!log.lines.is_empty(), "enemy should have logged an attack");
        // The 1-HP target must have died.
        let target = units.iter().find(|u| u.id == 3).unwrap();
        assert!(!target.alive, "lowest HP target should be killed");
    }

    #[test]
    fn enemy_approaches_when_out_of_range() {
        let map = Map::fixed_level();
        let mut units = vec![
            // Place enemy on (5, 4) (a road tile) and player on (5, 5)
            // road tile too — clearly within move range, but >1 away.
            unit(1, Faction::Enemy, GridPos::new(5, 4), 20),
            unit(2, Faction::Player, GridPos::new(0, 4), 20),
        ];
        let start = units[0].pos;
        run_enemy_turn(&map, &mut units);
        let after = units[0].pos;
        assert_ne!(start, after, "enemy should have moved closer");
        let dx0 = start.manhattan(units[1].pos);
        let dx1 = after.manhattan(units[1].pos);
        assert!(dx1 < dx0, "enemy should be closer than before");
    }
}
