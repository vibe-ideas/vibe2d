//! Damage / hit / counter / double resolution. Hits are deterministic in
//! MVP — no RNG — so VDP integration tests can pin exact HP outcomes.

use crate::map::Map;
use crate::model::{Unit, UnitId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatPreview {
    pub damage: i32,
    pub hit: i32,
    pub counter_damage: Option<i32>,
    pub counter_hit: Option<i32>,
    pub double_attack: bool,
    pub counter_double: bool,
}

const DOUBLE_THRESHOLD: i32 = 4;

fn tile_def_avoid(map: &Map, unit: &Unit) -> (i32, i32) {
    map.tile(unit.pos)
        .map(|t| (t.defense_bonus, t.avoid_bonus))
        .unwrap_or((0, 0))
}

fn one_attack_stats(map: &Map, attacker: &Unit, defender: &Unit) -> (i32, i32) {
    let (def_bonus, avo_bonus) = tile_def_avoid(map, defender);
    let dmg = (attacker.strength + attacker.weapon.might - defender.defense - def_bonus).max(1);
    let hit =
        (attacker.weapon.hit + attacker.skill * 2 - defender.speed * 2 - avo_bonus).clamp(0, 100);
    (dmg, hit)
}

pub fn preview(map: &Map, attacker: &Unit, defender: &Unit) -> CombatPreview {
    let (damage, hit) = one_attack_stats(map, attacker, defender);
    let distance = attacker.pos.manhattan(defender.pos);
    let can_counter = defender.weapon_can_reach(distance);
    let (counter_damage, counter_hit) = if can_counter {
        let (d, h) = one_attack_stats(map, defender, attacker);
        (Some(d), Some(h))
    } else {
        (None, None)
    };
    let double_attack = attacker.speed - defender.speed >= DOUBLE_THRESHOLD;
    let counter_double = can_counter && (defender.speed - attacker.speed >= DOUBLE_THRESHOLD);
    CombatPreview {
        damage,
        hit,
        counter_damage,
        counter_hit,
        double_attack,
        counter_double,
    }
}

#[derive(Debug, Default, Clone)]
pub struct CombatLogEntry {
    pub lines: Vec<String>,
}

/// Resolve a full attack: attacker → counter → attacker double → defender
/// double. MVP treats every blow as a hit (`hit >= 0` always lands); HP
/// can never go negative. Returns the human-readable log lines plus the
/// post-combat alive flags so the caller can reconcile victory/defeat.
pub fn resolve(
    map: &Map,
    units: &mut [Unit],
    attacker_id: UnitId,
    defender_id: UnitId,
) -> CombatLogEntry {
    let mut log = CombatLogEntry::default();

    let prev = {
        let a = units
            .iter()
            .find(|u| u.id == attacker_id)
            .expect("attacker");
        let d = units
            .iter()
            .find(|u| u.id == defender_id)
            .expect("defender");
        preview(map, a, d)
    };

    // Attack 1: attacker swings.
    apply_blow(units, attacker_id, defender_id, prev.damage, &mut log);

    // Counter, if defender survived and can reach.
    let defender_alive = units.iter().any(|u| u.id == defender_id && u.alive);
    if defender_alive && let Some(cd) = prev.counter_damage {
        apply_blow(units, defender_id, attacker_id, cd, &mut log);
    }

    // Attacker double, if still alive and qualifies.
    let attacker_alive = units.iter().any(|u| u.id == attacker_id && u.alive);
    let defender_alive = units.iter().any(|u| u.id == defender_id && u.alive);
    if prev.double_attack && attacker_alive && defender_alive {
        apply_blow(units, attacker_id, defender_id, prev.damage, &mut log);
    }

    // Defender double counter (rare).
    let attacker_alive = units.iter().any(|u| u.id == attacker_id && u.alive);
    let defender_alive = units.iter().any(|u| u.id == defender_id && u.alive);
    if prev.counter_double
        && let Some(cd) = prev.counter_damage
        && defender_alive
        && attacker_alive
    {
        apply_blow(units, defender_id, attacker_id, cd, &mut log);
    }

    // Mark attacker as having acted regardless of survival outcome.
    if let Some(a) = units.iter_mut().find(|u| u.id == attacker_id) {
        a.acted = true;
    }
    log
}

fn apply_blow(
    units: &mut [Unit],
    attacker_id: UnitId,
    defender_id: UnitId,
    raw_damage: i32,
    log: &mut CombatLogEntry,
) {
    let attacker_name = units
        .iter()
        .find(|u| u.id == attacker_id)
        .map(|u| u.name)
        .unwrap_or("?");
    let defender = units
        .iter_mut()
        .find(|u| u.id == defender_id)
        .expect("defender exists");
    let dealt = raw_damage.min(defender.hp).max(0);
    defender.hp = (defender.hp - raw_damage).max(0);
    let killed = if defender.hp == 0 {
        defender.alive = false;
        true
    } else {
        false
    };
    let defender_name = defender.name;
    let defender_hp = defender.hp;
    if killed {
        log.lines.push(format!(
            "{attacker_name} hits {defender_name} for {dealt} (KO)"
        ));
    } else {
        log.lines.push(format!(
            "{attacker_name} hits {defender_name} for {dealt} (HP {defender_hp})"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::Map;
    use crate::model::{Faction, GridPos, Weapon};

    fn unit(id: UnitId, faction: Faction, pos: GridPos, hp: i32, str_: i32, def: i32) -> Unit {
        Unit {
            id,
            name: "U",
            class_name: "T",
            faction,
            pos,
            hp,
            max_hp: hp,
            strength: str_,
            skill: 5,
            speed: 5,
            defense: def,
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

    #[test]
    fn preview_damage_floor_is_one() {
        let map = Map::fixed_level();
        let a = unit(1, Faction::Player, GridPos::new(0, 4), 10, 1, 0);
        let d = unit(2, Faction::Enemy, GridPos::new(1, 4), 10, 1, 99);
        let p = preview(&map, &a, &d);
        assert_eq!(p.damage, 1, "damage cannot drop below 1");
    }

    #[test]
    fn preview_counter_blocked_when_out_of_range() {
        let map = Map::fixed_level();
        let mut archer = unit(1, Faction::Player, GridPos::new(0, 4), 10, 5, 3);
        archer.weapon.min_range = 2;
        archer.weapon.max_range = 2;
        let melee = unit(2, Faction::Enemy, GridPos::new(2, 4), 10, 5, 3);
        let p = preview(&map, &archer, &melee);
        assert!(p.counter_damage.is_none(), "melee can't reach archer");
    }

    #[test]
    fn resolve_kills_when_damage_exceeds_hp() {
        let map = Map::fixed_level();
        let attacker = unit(1, Faction::Player, GridPos::new(0, 4), 20, 99, 0);
        let defender = unit(2, Faction::Enemy, GridPos::new(1, 4), 5, 1, 0);
        let mut units = vec![attacker, defender];
        resolve(&map, &mut units, 1, 2);
        let d = units.iter().find(|u| u.id == 2).unwrap();
        assert!(!d.alive);
        assert_eq!(d.hp, 0);
        let a = units.iter().find(|u| u.id == 1).unwrap();
        assert!(a.acted);
    }

    #[test]
    fn resolve_counter_can_kill_attacker() {
        let map = Map::fixed_level();
        let mut attacker = unit(1, Faction::Player, GridPos::new(0, 4), 5, 1, 0);
        attacker.weapon.might = 1;
        let defender = unit(2, Faction::Enemy, GridPos::new(1, 4), 100, 99, 0);
        let mut units = vec![attacker, defender];
        resolve(&map, &mut units, 1, 2);
        let a = units.iter().find(|u| u.id == 1).unwrap();
        assert!(!a.alive, "attacker dies to counter");
    }
}
