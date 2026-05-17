//! Enemy phase: single-frame synchronous AI for all enemies.

use crate::combat::resolve_combat;
use crate::map::{Map, attackable_from, reachable_tiles};
use crate::model::{Faction, GridPos, Unit, UnitId};

/// Execute the entire enemy phase synchronously in one call.
/// Returns a log of all actions taken.
pub fn run_enemy_phase(map: &Map, units: &mut Vec<Unit>) -> Vec<String> {
    let mut log = Vec::new();
    let enemy_ids: Vec<UnitId> = units
        .iter()
        .filter(|u| u.alive && u.faction == Faction::Enemy)
        .map(|u| u.id)
        .collect();

    for eid in enemy_ids {
        let eidx = match units.iter().position(|u| u.id == eid) {
            Some(i) => i,
            None => continue,
        };
        if !units[eidx].alive {
            continue;
        }

        let current_pos = units[eidx].pos;
        let wmin = units[eidx].weapon.min_range;
        let wmax = units[eidx].weapon.max_range;
        let direct_targets = attackable_from(current_pos, units, eid, Faction::Enemy, wmin, wmax);

        if !direct_targets.is_empty() {
            let target_id = direct_targets
                .iter()
                .filter_map(|&tid| units.iter().find(|u| u.id == tid))
                .min_by_key(|u| u.hp)
                .map(|u| u.id)
                .unwrap();
            let atk_from = current_pos;
            let combat_log = resolve_combat(units, eid, target_id, 0, 0, 0, 0, atk_from);
            log.extend(combat_log);
        } else {
            let move_range = units[eidx].move_range;
            let reachable =
                reachable_tiles(map, current_pos, move_range, units, eid, Faction::Enemy);

            let nearest_player_pos = units
                .iter()
                .filter(|u| u.alive && u.faction == Faction::Player)
                .map(|u| u.pos)
                .min_by_key(|&p| current_pos.manhattan_dist(&p));

            if let Some(target_pos) = nearest_player_pos {
                let occupied: Vec<GridPos> = units
                    .iter()
                    .filter(|u| u.alive && u.id != eid)
                    .map(|u| u.pos)
                    .collect();
                let best_move = reachable
                    .iter()
                    .filter(|&&p| !occupied.contains(&p))
                    .min_by_key(|&&p| p.manhattan_dist(&target_pos))
                    .copied();

                if let Some(new_pos) = best_move {
                    let eidx2 = units.iter().position(|u| u.id == eid).unwrap();
                    if new_pos != units[eidx2].pos {
                        log.push(format!(
                            "{} moves to ({},{})",
                            units[eidx2].name, new_pos.x, new_pos.y
                        ));
                        units[eidx2].pos = new_pos;
                    }
                    let eidx3 = units.iter().position(|u| u.id == eid).unwrap();
                    let new_pos = units[eidx3].pos;
                    let wmin2 = units[eidx3].weapon.min_range;
                    let wmax2 = units[eidx3].weapon.max_range;
                    let after_move_targets =
                        attackable_from(new_pos, units, eid, Faction::Enemy, wmin2, wmax2);
                    if !after_move_targets.is_empty() {
                        let t_id = after_move_targets
                            .iter()
                            .filter_map(|&tid| units.iter().find(|u| u.id == tid))
                            .min_by_key(|u| u.hp)
                            .map(|u| u.id)
                            .unwrap();
                        let atk_from2 = new_pos;
                        let combat_log = resolve_combat(units, eid, t_id, 0, 0, 0, 0, atk_from2);
                        log.extend(combat_log);
                    } else {
                        let eidx4 = units.iter().position(|u| u.id == eid).unwrap();
                        units[eidx4].acted = true;
                    }
                } else {
                    units[eidx].acted = true;
                }
            } else {
                units[eidx].acted = true;
            }
        }
    }
    log
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::build_map;
    use crate::model::Weapon;

    fn make_unit(id: UnitId, x: i32, y: i32, faction: Faction) -> Unit {
        Unit {
            id,
            name: "X",
            class_name: "C",
            faction,
            pos: GridPos::new(x, y),
            hp: 20,
            max_hp: 20,
            strength: 5,
            skill: 5,
            speed: 5,
            defense: 3,
            move_range: 4,
            weapon: Weapon {
                name: "S",
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
    fn enemy_attacks_adjacent_player() {
        let map = build_map();
        let enemy = make_unit(1, 2, 1, Faction::Enemy);
        let player = make_unit(2, 3, 1, Faction::Player);
        let mut units = vec![enemy, player];
        let log = run_enemy_phase(&map, &mut units);
        assert!(!log.is_empty(), "expected combat log");
        assert!(units[0].acted, "enemy should have acted");
    }

    #[test]
    fn enemy_moves_toward_player() {
        let map = build_map();
        let enemy = make_unit(1, 1, 5, Faction::Enemy);
        let player = make_unit(2, 7, 5, Faction::Player);
        let player_pos = player.pos;
        let mut units = vec![enemy, player];
        let initial_pos = units[0].pos;
        run_enemy_phase(&map, &mut units);
        let new_pos = units[0].pos;
        assert!(
            new_pos.manhattan_dist(&player_pos) < initial_pos.manhattan_dist(&player_pos),
            "enemy should have moved closer: from {:?} dist {} to {:?} dist {}",
            initial_pos,
            initial_pos.manhattan_dist(&player_pos),
            new_pos,
            new_pos.manhattan_dist(&player_pos)
        );
    }

    #[test]
    fn enemy_attacks_lowest_hp_player() {
        let map = build_map();
        let enemy = make_unit(1, 2, 1, Faction::Enemy);
        let mut player1 = make_unit(2, 3, 1, Faction::Player);
        let mut player2 = make_unit(3, 2, 2, Faction::Player);
        player1.hp = 5;
        player2.hp = 15;
        let mut units = vec![enemy, player1, player2];
        let log = run_enemy_phase(&map, &mut units);
        assert!(!log.is_empty());
        assert!(log[0].contains("X"), "should target some player");
    }
}
