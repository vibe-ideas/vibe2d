//! Core data types for the tactics demo.
//!
//! Map / movement-range algorithms live in [`crate::map`]; combat math in
//! [`crate::combat`]; everything here is plain data + tiny helpers.

pub type UnitId = u32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    PlayerSelect,
    PlayerMove,
    PlayerAction,
    PlayerAttackTarget,
    EnemyTurn,
    Victory,
    Defeat,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::PlayerSelect => "player_select",
            Phase::PlayerMove => "player_move",
            Phase::PlayerAction => "player_action",
            Phase::PlayerAttackTarget => "player_attack_target",
            Phase::EnemyTurn => "enemy_turn",
            Phase::Victory => "victory",
            Phase::Defeat => "defeat",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Faction {
    Player,
    Enemy,
}

impl Faction {
    pub fn as_str(self) -> &'static str {
        match self {
            Faction::Player => "player",
            Faction::Enemy => "enemy",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

impl GridPos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub fn manhattan(self, other: GridPos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

#[derive(Clone, Copy, Debug)]
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
    pub fn weapon_can_reach(&self, distance: i32) -> bool {
        distance >= self.weapon.min_range && distance <= self.weapon.max_range
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
    fn manhattan_distance_is_symmetric_and_zero_at_origin() {
        let a = GridPos::new(2, 3);
        let b = GridPos::new(5, 1);
        assert_eq!(a.manhattan(b), 5);
        assert_eq!(b.manhattan(a), 5);
        assert_eq!(a.manhattan(a), 0);
    }

    #[test]
    fn weapon_can_reach_uses_inclusive_bounds() {
        let mut u = make_test_unit();
        u.weapon.min_range = 1;
        u.weapon.max_range = 2;
        assert!(!u.weapon_can_reach(0));
        assert!(u.weapon_can_reach(1));
        assert!(u.weapon_can_reach(2));
        assert!(!u.weapon_can_reach(3));
    }

    fn make_test_unit() -> Unit {
        Unit {
            id: 1,
            name: "Test",
            class_name: "Tester",
            faction: Faction::Player,
            pos: GridPos::new(0, 0),
            hp: 10,
            max_hp: 10,
            strength: 5,
            skill: 5,
            speed: 5,
            defense: 3,
            move_range: 4,
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
}
