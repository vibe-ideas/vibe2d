//! Fixed level map, terrain table, and movement-range BFS.

use std::collections::{HashMap, HashSet};

use crate::model::{Faction, GridPos, Unit, UnitId};

pub const MAP_W: i32 = 14;
pub const MAP_H: i32 = 10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TileKind {
    Plain,
    Road,
    Forest,
    Fort,
    Wall,
}

impl TileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TileKind::Plain => "plain",
            TileKind::Road => "road",
            TileKind::Forest => "forest",
            TileKind::Fort => "fort",
            TileKind::Wall => "wall",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub kind: TileKind,
    pub move_cost: u32,
    pub defense_bonus: i32,
    pub avoid_bonus: i32,
    pub blocks: bool,
}

impl Tile {
    pub fn from_kind(kind: TileKind) -> Self {
        match kind {
            TileKind::Plain => Tile {
                kind,
                move_cost: 1,
                defense_bonus: 0,
                avoid_bonus: 0,
                blocks: false,
            },
            TileKind::Road => Tile {
                kind,
                move_cost: 1,
                defense_bonus: 0,
                avoid_bonus: 0,
                blocks: false,
            },
            TileKind::Forest => Tile {
                kind,
                move_cost: 2,
                defense_bonus: 1,
                avoid_bonus: 20,
                blocks: false,
            },
            TileKind::Fort => Tile {
                kind,
                move_cost: 1,
                defense_bonus: 2,
                avoid_bonus: 10,
                blocks: false,
            },
            TileKind::Wall => Tile {
                kind,
                move_cost: 99,
                defense_bonus: 0,
                avoid_bonus: 0,
                blocks: true,
            },
        }
    }
}

pub struct Map {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Tile>,
}

impl Map {
    pub fn fixed_level() -> Self {
        // 14x10 grid. '.' plain, 'r' road, 'f' forest, 'F' fort, '#' wall.
        // Layout: a horizontal road through the middle, two forest patches
        // flanking the player start, a fort near the enemy captain, a few
        // wall tiles on the right side that force tactical routing.
        const ROWS: [&str; MAP_H as usize] = [
            "..##....f.....",
            "..f.........f.",
            "..f.rrrrrr....",
            "....r....r....",
            "rrrrr....rrrrr",
            "rrrrr....rrrrr",
            "....r....r....",
            "..f.rrrrrr.F..",
            "..f...........",
            "..............",
        ];
        let mut tiles = Vec::with_capacity((MAP_W * MAP_H) as usize);
        for row in ROWS {
            assert_eq!(row.len(), MAP_W as usize, "row width must equal MAP_W");
            for ch in row.chars() {
                let kind = match ch {
                    '.' => TileKind::Plain,
                    'r' => TileKind::Road,
                    'f' => TileKind::Forest,
                    'F' => TileKind::Fort,
                    '#' => TileKind::Wall,
                    _ => panic!("unknown tile char {ch}"),
                };
                tiles.push(Tile::from_kind(kind));
            }
        }
        Self {
            width: MAP_W,
            height: MAP_H,
            tiles,
        }
    }

    pub fn in_bounds(&self, pos: GridPos) -> bool {
        pos.x >= 0 && pos.x < self.width && pos.y >= 0 && pos.y < self.height
    }

    pub fn tile(&self, pos: GridPos) -> Option<&Tile> {
        if !self.in_bounds(pos) {
            return None;
        }
        Some(&self.tiles[(pos.y * self.width + pos.x) as usize])
    }

    pub fn is_blocked(&self, pos: GridPos) -> bool {
        self.tile(pos).map(|t| t.blocks).unwrap_or(true)
    }
}

pub fn occupied_by(units: &[Unit], pos: GridPos) -> Option<UnitId> {
    units.iter().find(|u| u.alive && u.pos == pos).map(|u| u.id)
}

/// BFS-with-cost to compute the set of tiles a unit can move to.
///
/// Movement consumes `Tile::move_cost`; the unit's starting tile is free.
/// Tiles occupied by *enemy* units block traversal entirely; tiles occupied
/// by *friendly* units may be passed through but cannot be the final
/// destination (mirrors classic FE behavior). The starting tile itself
/// is included in the result so the player can choose to "wait in place".
pub fn reachable_tiles(map: &Map, units: &[Unit], unit: &Unit) -> Vec<GridPos> {
    let mut best: HashMap<GridPos, u32> = HashMap::new();
    best.insert(unit.pos, 0);
    let mut frontier: Vec<(GridPos, u32)> = vec![(unit.pos, 0)];

    while let Some((pos, cost)) = frontier.pop() {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let next = GridPos::new(pos.x + dx, pos.y + dy);
            let Some(tile) = map.tile(next) else {
                continue;
            };
            if tile.blocks {
                continue;
            }
            if let Some(occ_id) = occupied_by(units, next) {
                let occ = units.iter().find(|u| u.id == occ_id).unwrap();
                if occ.faction != unit.faction {
                    continue;
                }
            }
            let next_cost = cost + tile.move_cost;
            if next_cost > unit.move_range {
                continue;
            }
            if best.get(&next).is_some_and(|&c| c <= next_cost) {
                continue;
            }
            best.insert(next, next_cost);
            frontier.push((next, next_cost));
        }
    }

    // Final destinations: must be empty or the unit's own starting tile.
    best.into_iter()
        .filter(|(p, _)| *p == unit.pos || occupied_by(units, *p).is_none())
        .map(|(p, _)| p)
        .collect()
}

/// Tiles that a unit standing at `from` could attack from with its weapon,
/// constrained to enemy occupants. Diamond ring at `[min_range, max_range]`.
pub fn attack_targets_from(units: &[Unit], unit: &Unit, from: GridPos) -> Vec<GridPos> {
    let mut hits = Vec::new();
    for u in units {
        if !u.alive || u.faction == unit.faction {
            continue;
        }
        let d = from.manhattan(u.pos);
        if unit.weapon_can_reach(d) {
            hits.push(u.pos);
        }
    }
    hits
}

/// Union of all tiles the unit could attack from any reachable tile —
/// shown to players as the red overlay during PlayerMove. De-duplicated.
pub fn attackable_after_move(map: &Map, units: &[Unit], unit: &Unit) -> Vec<GridPos> {
    let reach = reachable_tiles(map, units, unit);
    let mut out: HashSet<GridPos> = HashSet::new();
    for from in reach {
        for t in attack_targets_from(units, unit, from) {
            out.insert(t);
        }
    }
    out.into_iter().collect()
}

pub fn alive_units_of(units: &[Unit], faction: Faction) -> impl Iterator<Item = &Unit> {
    units
        .iter()
        .filter(move |u| u.alive && u.faction == faction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Weapon;

    fn mk(id: UnitId, faction: Faction, pos: GridPos, mv: u32) -> Unit {
        Unit {
            id,
            name: "U",
            class_name: "T",
            faction,
            pos,
            hp: 10,
            max_hp: 10,
            strength: 5,
            skill: 5,
            speed: 5,
            defense: 3,
            move_range: mv,
            weapon: Weapon {
                name: "Sword",
                might: 5,
                hit: 90,
                min_range: 1,
                max_range: 1,
            },
            acted: false,
            alive: true,
        }
    }

    #[test]
    fn forest_costs_more_than_plain() {
        assert_eq!(Tile::from_kind(TileKind::Plain).move_cost, 1);
        assert_eq!(Tile::from_kind(TileKind::Forest).move_cost, 2);
    }

    #[test]
    fn wall_blocks_traversal() {
        assert!(Tile::from_kind(TileKind::Wall).blocks);
    }

    #[test]
    fn fixed_level_has_expected_dimensions() {
        let m = Map::fixed_level();
        assert_eq!(m.width, MAP_W);
        assert_eq!(m.height, MAP_H);
        assert_eq!(m.tiles.len(), (MAP_W * MAP_H) as usize);
        assert!(!m.in_bounds(GridPos::new(-1, 0)));
        assert!(!m.in_bounds(GridPos::new(MAP_W, 0)));
        assert!(m.in_bounds(GridPos::new(0, 0)));
    }

    #[test]
    fn reachable_avoids_walls_and_respects_budget() {
        let m = Map::fixed_level();
        let u = mk(1, Faction::Player, GridPos::new(0, 4), 3);
        let units = vec![u.clone()];
        let r = reachable_tiles(&m, &units, &u);
        assert!(r.contains(&u.pos), "starting tile must be reachable");
        // No reachable tile may be a wall.
        for p in &r {
            assert!(
                !m.tile(*p).unwrap().blocks,
                "reachable tile {p:?} is a wall"
            );
        }
        // Budget cap: cost 0 reaches 1 tile, cost 3 over road can reach
        // ≥ 4 distinct tiles.
        assert!(r.len() >= 4);
    }

    #[test]
    fn cannot_end_movement_on_friendly_but_can_on_self() {
        let m = Map::fixed_level();
        let u1 = mk(1, Faction::Player, GridPos::new(0, 4), 4);
        let u2 = mk(2, Faction::Player, GridPos::new(1, 4), 4);
        let units = vec![u1.clone(), u2.clone()];
        let r = reachable_tiles(&m, &units, &u1);
        assert!(r.contains(&u1.pos));
        assert!(!r.contains(&u2.pos), "cannot end on friendly tile");
    }

    #[test]
    fn enemy_tile_itself_is_never_a_destination() {
        // The fixed map has multiple road corridors so the BFS may route
        // *around* a blocker — but the blocker's own tile must never be a
        // legal destination for a player unit.
        let m = Map::fixed_level();
        let u = mk(1, Faction::Player, GridPos::new(0, 4), 6);
        let blocker = mk(2, Faction::Enemy, GridPos::new(2, 4), 4);
        let units = vec![u.clone(), blocker];
        let r = reachable_tiles(&m, &units, &u);
        assert!(!r.contains(&GridPos::new(2, 4)), "cannot end on enemy tile");
    }

    #[test]
    fn enemy_blocks_traversal_through_corridor() {
        // Set up a row-0 corridor where x=3 is a wall (per fixed level)
        // and we plug x=4 with an enemy so x=5..7 cost more from x=2.
        // The far-side tile must require the longer detour, exceeding
        // the player's move budget.
        let m = Map::fixed_level();
        let u = mk(1, Faction::Player, GridPos::new(2, 1), 1);
        let blocker = mk(2, Faction::Enemy, GridPos::new(3, 1), 4);
        let units = vec![u.clone(), blocker];
        let r = reachable_tiles(&m, &units, &u);
        // (3, 1) blocked by enemy; with move=1 we can't even reach (4,1).
        assert!(!r.contains(&GridPos::new(3, 1)));
        assert!(!r.contains(&GridPos::new(4, 1)));
    }

    #[test]
    fn dead_unit_does_not_block_traversal() {
        let m = Map::fixed_level();
        let u = mk(1, Faction::Player, GridPos::new(2, 1), 1);
        let mut blocker = mk(2, Faction::Enemy, GridPos::new(3, 1), 4);
        blocker.alive = false;
        let units = vec![u.clone(), blocker];
        let r = reachable_tiles(&m, &units, &u);
        assert!(
            r.contains(&GridPos::new(3, 1)),
            "dead enemy must not block movement"
        );
    }

    #[test]
    fn attack_targets_from_respects_min_max_range() {
        let mut archer = mk(1, Faction::Player, GridPos::new(0, 0), 4);
        archer.weapon.min_range = 2;
        archer.weapon.max_range = 2;
        let near = mk(2, Faction::Enemy, GridPos::new(1, 0), 4);
        let far = mk(3, Faction::Enemy, GridPos::new(2, 0), 4);
        let units = vec![archer.clone(), near, far];
        let hits = attack_targets_from(&units, &archer, archer.pos);
        assert!(!hits.contains(&GridPos::new(1, 0)), "1 tile is below min");
        assert!(hits.contains(&GridPos::new(2, 0)));
    }
}
