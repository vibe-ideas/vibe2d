//! Battle preview, damage, counter, double attack, and death resolution.

use crate::model::{GridPos, Unit, UnitId};

#[derive(Debug, Clone)]
pub struct CombatPreview {
    pub attacker_id: UnitId,
    pub defender_id: UnitId,
    pub atk_damage: i32,
    pub atk_hit: i32,
    pub def_damage: Option<i32>,
    pub def_hit: Option<i32>,
    pub atk_double: bool,
}

pub fn preview_combat(
    attacker: &Unit,
    defender: &Unit,
    atk_tile_def: i32,
    atk_tile_avoid: i32,
    def_tile_def: i32,
    def_tile_avoid: i32,
    attack_from: GridPos,
) -> CombatPreview {
    let atk_damage = calc_damage(attacker, defender, def_tile_def);
    let atk_hit = calc_hit(attacker, defender, def_tile_avoid);
    let atk_double = attacker.speed - defender.speed >= 4;

    let dist = attack_from.manhattan_dist(&defender.pos);
    let can_counter = defender.weapon.min_range <= dist && dist <= defender.weapon.max_range;
    let (def_damage, def_hit) = if can_counter {
        (
            Some(calc_damage(defender, attacker, atk_tile_def)),
            Some(calc_hit(defender, attacker, atk_tile_avoid)),
        )
    } else {
        (None, None)
    };

    CombatPreview {
        attacker_id: attacker.id,
        defender_id: defender.id,
        atk_damage,
        atk_hit,
        def_damage,
        def_hit,
        atk_double,
    }
}

fn calc_damage(attacker: &Unit, defender: &Unit, def_tile_def: i32) -> i32 {
    (attacker.strength + attacker.weapon.might - defender.defense - def_tile_def).max(1)
}

fn calc_hit(attacker: &Unit, defender: &Unit, def_tile_avoid: i32) -> i32 {
    (attacker.weapon.hit + attacker.skill * 2 - defender.speed * 2 - def_tile_avoid).clamp(0, 100)
}

/// Resolve combat between attacker and defender in-place.
/// Returns a log of what happened.
pub fn resolve_combat(
    units: &mut Vec<Unit>,
    attacker_id: UnitId,
    defender_id: UnitId,
    def_tile_def: i32,
    def_tile_avoid: i32,
    atk_tile_def: i32,
    atk_tile_avoid: i32,
    attack_from: GridPos,
) -> Vec<String> {
    let mut log = Vec::new();

    let atk_idx = units.iter().position(|u| u.id == attacker_id).unwrap();
    let def_idx = units.iter().position(|u| u.id == defender_id).unwrap();
    let preview = preview_combat(
        &units[atk_idx],
        &units[def_idx],
        atk_tile_def,
        atk_tile_avoid,
        def_tile_def,
        def_tile_avoid,
        attack_from,
    );

    // Step 1: Attacker attacks defender
    let dmg1 = preview.atk_damage;
    units[def_idx].hp -= dmg1;
    log.push(format!(
        "{} -> {}: {} dmg (hit {}%)",
        units[atk_idx].name, units[def_idx].name, dmg1, preview.atk_hit
    ));
    if units[def_idx].hp <= 0 {
        units[def_idx].hp = 0;
        units[def_idx].alive = false;
        log.push(format!("{} defeated!", units[def_idx].name));
        units[atk_idx].acted = true;
        return log;
    }

    // Step 2: Counter-attack
    if let Some(def_dmg) = preview.def_damage {
        let dmg2 = def_dmg;
        units[atk_idx].hp -= dmg2;
        log.push(format!(
            "{} <- {}: {} dmg (hit {}%)",
            units[atk_idx].name,
            units[def_idx].name,
            dmg2,
            preview.def_hit.unwrap_or(0)
        ));
        if units[atk_idx].hp <= 0 {
            units[atk_idx].hp = 0;
            units[atk_idx].alive = false;
            log.push(format!("{} defeated!", units[atk_idx].name));
            units[atk_idx].acted = true;
            return log;
        }
    }

    // Step 3: Double attack if attacker is still alive and fast enough
    if preview.atk_double && units[atk_idx].alive {
        let dmg3 = preview.atk_damage;
        units[def_idx].hp -= dmg3;
        log.push(format!(
            "{} -> {} (double): {} dmg",
            units[atk_idx].name, units[def_idx].name, dmg3
        ));
        if units[def_idx].hp <= 0 {
            units[def_idx].hp = 0;
            units[def_idx].alive = false;
            log.push(format!("{} defeated!", units[def_idx].name));
        }
    }

    units[atk_idx].acted = true;
    log
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Faction, Weapon};

    fn make_unit(
        id: UnitId,
        faction: Faction,
        pos: GridPos,
        hp: i32,
        max_hp: i32,
        str: i32,
        sk: i32,
        spd: i32,
        def: i32,
        wmight: i32,
        whit: i32,
        wmin: i32,
        wmax: i32,
    ) -> Unit {
        Unit {
            id,
            name: "X",
            class_name: "C",
            faction,
            pos,
            hp,
            max_hp,
            strength: str,
            skill: sk,
            speed: spd,
            defense: def,
            move_range: 5,
            weapon: Weapon {
                name: "S",
                might: wmight,
                hit: whit,
                min_range: wmin,
                max_range: wmax,
            },
            acted: false,
            alive: true,
        }
    }

    #[test]
    fn damage_formula() {
        let a = make_unit(
            1,
            Faction::Player,
            GridPos::new(1, 1),
            20,
            20,
            10,
            5,
            5,
            3,
            5,
            80,
            1,
            1,
        );
        let d = make_unit(
            2,
            Faction::Enemy,
            GridPos::new(2, 1),
            20,
            20,
            5,
            3,
            3,
            5,
            3,
            70,
            1,
            1,
        );
        let preview = preview_combat(&a, &d, 0, 0, 0, 0, GridPos::new(1, 1));
        // str(10) + might(5) - def(5) = 10
        assert_eq!(preview.atk_damage, 10);
    }

    #[test]
    fn hit_formula_clamped() {
        let a = make_unit(
            1,
            Faction::Player,
            GridPos::new(1, 1),
            20,
            20,
            5,
            0,
            0,
            3,
            5,
            0,
            1,
            1,
        );
        let d = make_unit(
            2,
            Faction::Enemy,
            GridPos::new(2, 1),
            20,
            20,
            5,
            0,
            100,
            3,
            3,
            70,
            1,
            1,
        );
        let preview = preview_combat(&a, &d, 0, 0, 0, 0, GridPos::new(1, 1));
        // hit + skill*2 - speed*2 = 0 + 0 - 200 = clamped to 0
        assert_eq!(preview.atk_hit, 0);
    }

    #[test]
    fn no_counter_out_of_range() {
        let a = make_unit(
            1,
            Faction::Player,
            GridPos::new(1, 1),
            20,
            20,
            10,
            5,
            5,
            3,
            5,
            80,
            1,
            1,
        );
        let d = make_unit(
            2,
            Faction::Enemy,
            GridPos::new(5, 1),
            20,
            20,
            5,
            3,
            3,
            5,
            3,
            70,
            1,
            1,
        );
        let preview = preview_combat(&a, &d, 0, 0, 0, 0, GridPos::new(1, 1));
        assert!(
            preview.def_damage.is_none(),
            "no counter from dist 4 with range 1"
        );
    }

    #[test]
    fn double_attack_when_speed_diff_4() {
        let a = make_unit(
            1,
            Faction::Player,
            GridPos::new(1, 1),
            20,
            20,
            5,
            5,
            10,
            3,
            5,
            80,
            1,
            1,
        );
        let d = make_unit(
            2,
            Faction::Enemy,
            GridPos::new(2, 1),
            20,
            20,
            5,
            3,
            6,
            5,
            3,
            70,
            1,
            1,
        );
        let preview = preview_combat(&a, &d, 0, 0, 0, 0, GridPos::new(1, 1));
        assert!(preview.atk_double, "speed 10 - 6 = 4 should trigger double");
    }

    #[test]
    fn resolve_combat_kills_defender() {
        let a = make_unit(
            1,
            Faction::Player,
            GridPos::new(1, 1),
            20,
            20,
            20,
            5,
            5,
            3,
            20,
            100,
            1,
            1,
        );
        let d = make_unit(
            2,
            Faction::Enemy,
            GridPos::new(2, 1),
            1,
            10,
            5,
            3,
            3,
            0,
            3,
            70,
            1,
            1,
        );
        let mut units = vec![a, d];
        let log = resolve_combat(&mut units, 1, 2, 0, 0, 0, 0, GridPos::new(1, 1));
        assert!(!units[1].alive, "defender should be dead");
        assert!(units[0].alive, "attacker should survive");
        assert!(log.iter().any(|l| l.contains("defeated")));
    }

    #[test]
    fn resolve_sets_acted() {
        let a = make_unit(
            1,
            Faction::Player,
            GridPos::new(1, 1),
            20,
            20,
            5,
            5,
            5,
            3,
            5,
            100,
            1,
            1,
        );
        let d = make_unit(
            2,
            Faction::Enemy,
            GridPos::new(2, 1),
            20,
            20,
            5,
            3,
            3,
            5,
            3,
            70,
            1,
            1,
        );
        let mut units = vec![a, d];
        resolve_combat(&mut units, 1, 2, 0, 0, 0, 0, GridPos::new(1, 1));
        assert!(units[0].acted, "attacker acted should be true after combat");
    }
}
