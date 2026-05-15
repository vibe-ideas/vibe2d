//! Fixed level data, terrain queries, and movement-range algorithm.

use crate::model::{Faction, GridPos, Tile, TileKind, Unit, UnitId};
use std::collections::{HashMap, VecDeque};

pub const MAP_W: i32 = 14;
pub const MAP_H: i32 = 10;

pub struct Map {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Tile>,
}

impl Map {
    pub fn idx(&self, pos: GridPos) -> usize {
        (pos.y * self.width + pos.x) as usize
    }
    pub fn in_bounds(&self, pos: GridPos) -> bool {
        pos.x >= 0 && pos.x < self.width && pos.y >= 0 && pos.y < self.height
    }
    pub fn tile(&self, pos: GridPos) -> Option<&Tile> {
        if self.in_bounds(pos) {
            Some(&self.tiles[self.idx(pos)])
        } else {
            None
        }
    }
    pub fn is_blocked(&self, pos: GridPos) -> bool {
        self.tile(pos).map(|t| t.blocks).unwrap_or(true)
    }
    pub fn tile_kind(&self, pos: GridPos) -> Option<TileKind> {
        self.tile(pos).map(|t| t.kind)
    }
}

/// Build the fixed 14×10 level map.
pub fn build_map() -> Map {
    // W = Wall, P = Plain, R = Road, F = Forest, T = Fort
    // Row-major, top to bottom
    // 14 columns × 10 rows
    #[rustfmt::skip]
    let layout: &[&str] = &[
        "WWWWWWWWWWWWWW",
        "WPPPFPPPPPPPFW",
        "WPRPPPPPPPPPRW",
        "WPPFPWWWPPFPPW",
        "WPPPPWPPPPPPFW",
        "WRPPPPPPPPPPFW",
        "WPPFPPPPWWWWWW",
        "WPPPPPPPPPPPTW",
        "WPPPFPPPPPPPFW",
        "WWWWWWWWWWWWWW",
    ];
    let mut tiles = Vec::with_capacity((MAP_W * MAP_H) as usize);
    for row in layout {
        for ch in row.chars() {
            tiles.push(match ch {
                'W' => Tile::wall(),
                'R' => Tile::road(),
                'F' => Tile::forest(),
                'T' => Tile::fort(),
                _ => Tile::plain(),
            });
        }
    }
    Map {
        width: MAP_W,
        height: MAP_H,
        tiles,
    }
}

/// BFS movement range.  Returns all GridPos reachable within `move_range` move points.
/// Blocks on wall tiles, enemy-occupied tiles (other faction), and out-of-bounds.
pub fn reachable_tiles(
    map: &Map,
    start: GridPos,
    move_range: u32,
    units: &[Unit],
    mover_id: UnitId,
    mover_faction: Faction,
) -> Vec<GridPos> {
    let mut cost_map: HashMap<GridPos, u32> = HashMap::new();
    let mut queue: VecDeque<(GridPos, u32)> = VecDeque::new();
    cost_map.insert(start, 0);
    queue.push_back((start, 0));

    while let Some((pos, cost)) = queue.pop_front() {
        for nb in pos.neighbors() {
            if !map.in_bounds(nb) {
                continue;
            }
            if map.is_blocked(nb) {
                continue;
            }
            let blocked_by_unit = units
                .iter()
                .any(|u| u.alive && u.id != mover_id && u.pos == nb && u.faction != mover_faction);
            if blocked_by_unit {
                continue;
            }
            let tile_cost = map.tile(nb).map(|t| t.move_cost).unwrap_or(99);
            let new_cost = cost + tile_cost;
            if new_cost <= move_range {
                if !cost_map.contains_key(&nb) || cost_map[&nb] > new_cost {
                    cost_map.insert(nb, new_cost);
                    queue.push_back((nb, new_cost));
                }
            }
        }
    }
    cost_map.into_keys().collect()
}

/// Compute all enemy positions reachable-and-attackable from any reachable tile.
pub fn attackable_targets(
    reachable: &[GridPos],
    units: &[Unit],
    attacker_id: UnitId,
    attacker_faction: Faction,
    weapon_min: i32,
    weapon_max: i32,
) -> Vec<GridPos> {
    let mut result = Vec::new();
    for target_unit in units {
        if !target_unit.alive {
            continue;
        }
        if target_unit.faction == attacker_faction {
            continue;
        }
        if target_unit.id == attacker_id {
            continue;
        }
        let reachable_from_any = reachable.iter().any(|&from| {
            let dist = from.manhattan_dist(&target_unit.pos);
            dist >= weapon_min && dist <= weapon_max
        });
        if reachable_from_any {
            result.push(target_unit.pos);
        }
    }
    result
}

/// Attackable targets from a single position.
pub fn attackable_from(
    from: GridPos,
    units: &[Unit],
    attacker_id: UnitId,
    attacker_faction: Faction,
    weapon_min: i32,
    weapon_max: i32,
) -> Vec<UnitId> {
    units
        .iter()
        .filter(|u| u.alive && u.faction != attacker_faction && u.id != attacker_id)
        .filter(|u| {
            let dist = from.manhattan_dist(&u.pos);
            dist >= weapon_min && dist <= weapon_max
        })
        .map(|u| u.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Weapon;

    fn make_unit(id: UnitId, x: i32, y: i32, faction: Faction, alive: bool) -> Unit {
        Unit {
            id,
            name: "T",
            class_name: "C",
            faction,
            pos: GridPos::new(x, y),
            hp: 10,
            max_hp: 10,
            strength: 5,
            skill: 5,
            speed: 5,
            defense: 3,
            move_range: 4,
            weapon: Weapon {
                name: "S",
                might: 5,
                hit: 80,
                min_range: 1,
                max_range: 1,
            },
            acted: false,
            alive,
        }
    }

    #[test]
    fn map_in_bounds() {
        let m = build_map();
        assert!(m.in_bounds(GridPos::new(1, 1)));
        assert!(!m.in_bounds(GridPos::new(-1, 0)));
        assert!(!m.in_bounds(GridPos::new(14, 0)));
    }

    #[test]
    fn map_border_is_wall() {
        let m = build_map();
        for x in 0..MAP_W {
            assert!(
                m.is_blocked(GridPos::new(x, 0)),
                "top row should be wall at x={}",
                x
            );
            assert!(
                m.is_blocked(GridPos::new(x, MAP_H - 1)),
                "bottom row should be wall at x={}",
                x
            );
        }
        for y in 0..MAP_H {
            assert!(
                m.is_blocked(GridPos::new(0, y)),
                "left col should be wall at y={}",
                y
            );
            assert!(
                m.is_blocked(GridPos::new(MAP_W - 1, y)),
                "right col should be wall at y={}",
                y
            );
        }
    }

    #[test]
    fn reachable_basic() {
        let m = build_map();
        let units = vec![];
        let r = reachable_tiles(&m, GridPos::new(1, 1), 3, &units, 1, Faction::Player);
        assert!(!r.is_empty(), "should reach some tiles");
        assert!(!r.contains(&GridPos::new(0, 1)));
    }

    #[test]
    fn reachable_cannot_cross_wall() {
        let m = build_map();
        let units = vec![];
        let r = reachable_tiles(&m, GridPos::new(3, 3), 2, &units, 1, Faction::Player);
        assert!(!r.contains(&GridPos::new(6, 3)));
        assert!(!r.contains(&GridPos::new(7, 3)));
    }

    #[test]
    fn reachable_blocked_by_enemy() {
        let m = build_map();
        let enemy = make_unit(2, 2, 1, Faction::Enemy, true);
        let r = reachable_tiles(
            &m,
            GridPos::new(1, 1),
            4,
            &[enemy.clone()],
            1,
            Faction::Player,
        );
        assert!(!r.contains(&GridPos::new(2, 1)));
    }

    #[test]
    fn attackable_from_correct_range() {
        let m = build_map();
        let _ = m;
        let units = vec![
            make_unit(2, 4, 1, Faction::Enemy, true),
            make_unit(3, 10, 1, Faction::Enemy, true),
        ];
        let attackable = attackable_from(GridPos::new(3, 1), &units, 1, Faction::Player, 1, 1);
        assert!(
            attackable.contains(&2),
            "unit at (4,1) should be attackable from (3,1)"
        );
        assert!(
            !attackable.contains(&3),
            "unit at (10,1) should not be attackable from (3,1)"
        );
    }
}
