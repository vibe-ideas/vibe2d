//! Core data types for tactics-demo.

pub type UnitId = u32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Title,
    PlayerSelect,
    PlayerMove,
    PlayerAction,
    PlayerAttackTarget,
    EnemyTurn,
    Victory,
    Defeat,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Faction {
    Player,
    Enemy,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TileKind {
    Plain,
    Road,
    Forest,
    Fort,
    Wall,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

impl GridPos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub fn manhattan_dist(&self, other: &GridPos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
    pub fn neighbors(&self) -> [GridPos; 4] {
        [
            GridPos::new(self.x - 1, self.y),
            GridPos::new(self.x + 1, self.y),
            GridPos::new(self.x, self.y - 1),
            GridPos::new(self.x, self.y + 1),
        ]
    }
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub kind: TileKind,
    pub move_cost: u32,
    pub defense_bonus: i32,
    pub avoid_bonus: i32,
    pub blocks: bool,
}

impl Tile {
    pub fn plain() -> Self {
        Self {
            kind: TileKind::Plain,
            move_cost: 1,
            defense_bonus: 0,
            avoid_bonus: 0,
            blocks: false,
        }
    }
    pub fn road() -> Self {
        Self {
            kind: TileKind::Road,
            move_cost: 1,
            defense_bonus: 0,
            avoid_bonus: 10,
            blocks: false,
        }
    }
    pub fn forest() -> Self {
        Self {
            kind: TileKind::Forest,
            move_cost: 2,
            defense_bonus: 1,
            avoid_bonus: 20,
            blocks: false,
        }
    }
    pub fn fort() -> Self {
        Self {
            kind: TileKind::Fort,
            move_cost: 1,
            defense_bonus: 2,
            avoid_bonus: 30,
            blocks: false,
        }
    }
    pub fn wall() -> Self {
        Self {
            kind: TileKind::Wall,
            move_cost: 99,
            defense_bonus: 0,
            avoid_bonus: 0,
            blocks: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Weapon {
    pub name: &'static str,
    pub might: i32,
    pub hit: i32,
    pub min_range: i32,
    pub max_range: i32,
}

#[derive(Clone, Debug)]
pub struct Unit {
    pub id: UnitId,
    pub name: &'static str,
    pub class_name: &'static str,
    pub faction: Faction,
    pub pos: GridPos,
    pub hp: i32,
    pub max_hp: i32,
    pub strength: i32,
    pub skill: i32,
    pub speed: i32,
    pub defense: i32,
    pub move_range: u32,
    pub weapon: Weapon,
    pub acted: bool,
    pub alive: bool,
}

impl Unit {
    pub fn can_attack_from(&self, from: GridPos, target: GridPos) -> bool {
        let dist = from.manhattan_dist(&target);
        dist >= self.weapon.min_range && dist <= self.weapon.max_range
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PendingAction {
    None,
    Selected {
        unit_id: UnitId,
    },
    Moved {
        unit_id: UnitId,
        from: GridPos,
        to: GridPos,
    },
    ChoosingAttack {
        unit_id: UnitId,
        from: GridPos,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_pos_manhattan_dist() {
        let a = GridPos::new(0, 0);
        let b = GridPos::new(3, 4);
        assert_eq!(a.manhattan_dist(&b), 7);
    }

    #[test]
    fn unit_can_attack_range() {
        let weapon = Weapon {
            name: "Sword",
            might: 5,
            hit: 80,
            min_range: 1,
            max_range: 1,
        };
        let u = Unit {
            id: 1,
            name: "A",
            class_name: "Fighter",
            faction: Faction::Player,
            pos: GridPos::new(0, 0),
            hp: 20,
            max_hp: 20,
            strength: 8,
            skill: 5,
            speed: 7,
            defense: 5,
            move_range: 5,
            weapon,
            acted: false,
            alive: true,
        };
        assert!(u.can_attack_from(GridPos::new(2, 3), GridPos::new(2, 4)));
        assert!(!u.can_attack_from(GridPos::new(2, 3), GridPos::new(2, 5)));
    }

    #[test]
    fn tile_kinds_correct_costs() {
        assert_eq!(Tile::plain().move_cost, 1);
        assert_eq!(Tile::forest().move_cost, 2);
        assert!(Tile::wall().blocks);
        assert!(!Tile::plain().blocks);
    }
}
