//! Vibe2D tactics demo. See `docs/game_demo/tactics-demo/plan.md` for the
//! design intent. Module layout follows the plan: data in `model`, level +
//! pathing in `map`, combat math in `combat`, enemy phase in `ai`, input
//! mapping in `input`, VDP wiring in `vdp`. This file owns the
//! `TacticsDemo` game state, the `Game` trait impl, and the
//! state-machine action methods that both UI input and VDP route into.

use vibe2d::prelude::*;

mod ai;
mod combat;
mod input;
mod map;
mod model;
#[cfg(feature = "vdp")]
mod vdp;

use crate::ai::run_enemy_turn;
use crate::combat::{CombatLogEntry, CombatPreview, preview, resolve};
use crate::input::{Cmd, TILE_PX, collect_command};
use crate::map::{
    MAP_H, MAP_W, Map, alive_units_of, attack_targets_from, attackable_after_move, reachable_tiles,
};
use crate::model::{Faction, GridPos, PendingAction, Phase, Unit, UnitId, Weapon};

const HUD_X: f32 = 600.0;
const MAP_PX_W: f32 = MAP_W as f32 * TILE_PX;
const MAP_PX_H: f32 = MAP_H as f32 * TILE_PX;
const MAX_LOG_LINES: usize = 6;

pub struct TacticsDemo {
    pub phase: Phase,
    pub turn: u32,
    pub map: Map,
    pub units: Vec<Unit>,
    pub cursor: GridPos,
    pub reachable: Vec<GridPos>,
    pub attackable: Vec<GridPos>,
    pub pending_action: PendingAction,
    pub combat_log: Vec<String>,
    pub ai_enabled: bool,
    pub white_tex: TextureId,
    pub disc_tex: TextureId,
    pub ring_tex: TextureId,
    /// Latched UI-button intents for this frame, since `update_ui` runs
    /// after `update`. We sample them on the *next* frame in `update`.
    pub ui_intent: UiIntent,
    /// Last observed mouse pixel position. We only let the mouse override
    /// the grid cursor when this changes — otherwise a still mouse at
    /// (0,0) would clobber cursors set by `select_unit_action` or VDP.
    pub last_mouse_px: Option<(f32, f32)>,
}

#[derive(Default, Clone, Copy)]
pub struct UiIntent {
    pub attack: bool,
    pub wait: bool,
    pub cancel: bool,
    pub end_turn: bool,
}

impl TacticsDemo {
    fn build_units() -> Vec<Unit> {
        let sword = Weapon {
            name: "Sword",
            might: 6,
            hit: 90,
            min_range: 1,
            max_range: 1,
        };
        let lance = Weapon {
            name: "Lance",
            might: 7,
            hit: 80,
            min_range: 1,
            max_range: 1,
        };
        let fire = Weapon {
            name: "Fire",
            might: 5,
            hit: 90,
            min_range: 1,
            max_range: 2,
        };
        let bow = Weapon {
            name: "Bow",
            might: 6,
            hit: 85,
            min_range: 2,
            max_range: 2,
        };
        let axe = Weapon {
            name: "Axe",
            might: 6,
            hit: 70,
            min_range: 1,
            max_range: 1,
        };
        let boss_axe = Weapon {
            name: "Boss Axe",
            might: 8,
            hit: 70,
            min_range: 1,
            max_range: 1,
        };
        let mut next = 0u32;
        let mut mk = |name, class, faction, pos, hp, str_, skl, spd, def, mv, w| {
            next += 1;
            Unit {
                id: next,
                name,
                class_name: class,
                faction,
                pos,
                hp,
                max_hp: hp,
                strength: str_,
                skill: skl,
                speed: spd,
                defense: def,
                move_range: mv,
                weapon: w,
                acted: false,
                alive: true,
            }
        };
        vec![
            mk(
                "Alen",
                "Fighter",
                Faction::Player,
                GridPos::new(1, 5),
                22,
                7,
                6,
                6,
                5,
                4,
                sword,
            ),
            mk(
                "Cain",
                "Cavalier",
                Faction::Player,
                GridPos::new(1, 4),
                20,
                7,
                5,
                7,
                6,
                6,
                lance,
            ),
            mk(
                "Lena",
                "Mage",
                Faction::Player,
                GridPos::new(0, 4),
                16,
                5,
                8,
                9,
                3,
                4,
                fire,
            ),
            mk(
                "Maric",
                "Archer",
                Faction::Player,
                GridPos::new(0, 5),
                18,
                6,
                7,
                6,
                4,
                4,
                bow,
            ),
            mk(
                "Orc-A",
                "Fighter",
                Faction::Enemy,
                GridPos::new(12, 4),
                18,
                5,
                3,
                4,
                3,
                4,
                axe,
            ),
            mk(
                "Orc-B",
                "Fighter",
                Faction::Enemy,
                GridPos::new(12, 5),
                18,
                5,
                3,
                4,
                3,
                4,
                axe,
            ),
            mk(
                "Goblin-A",
                "Soldier",
                Faction::Enemy,
                GridPos::new(10, 3),
                16,
                4,
                4,
                4,
                4,
                4,
                lance,
            ),
            mk(
                "Goblin-B",
                "Soldier",
                Faction::Enemy,
                GridPos::new(10, 6),
                16,
                4,
                4,
                4,
                4,
                4,
                lance,
            ),
            mk(
                "Merc",
                "Fighter",
                Faction::Enemy,
                GridPos::new(11, 4),
                20,
                6,
                5,
                6,
                4,
                4,
                sword,
            ),
            mk(
                "Captain",
                "Boss",
                Faction::Enemy,
                GridPos::new(11, 7),
                28,
                8,
                6,
                5,
                7,
                4,
                boss_axe,
            ),
        ]
    }

    pub fn reset_state(&mut self) {
        self.phase = Phase::PlayerSelect;
        self.turn = 1;
        self.units = Self::build_units();
        self.reachable.clear();
        self.attackable.clear();
        self.pending_action = PendingAction::None;
        self.combat_log.clear();
        self.combat_log.push("Player phase".into());
        self.cursor = self.units[0].pos;
    }

    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id && u.alive)
    }
    pub fn unit_at(&self, pos: GridPos) -> Option<&Unit> {
        self.units.iter().find(|u| u.alive && u.pos == pos)
    }

    pub fn check_winner(&mut self) {
        if matches!(self.phase, Phase::Victory | Phase::Defeat) {
            return;
        }
        let players_alive = alive_units_of(&self.units, Faction::Player).count();
        let enemies_alive = alive_units_of(&self.units, Faction::Enemy).count();
        if enemies_alive == 0 {
            self.phase = Phase::Victory;
            self.push_log("Victory!");
        } else if players_alive == 0 {
            self.phase = Phase::Defeat;
            self.push_log("Defeat...");
        }
    }

    pub fn push_log(&mut self, line: impl Into<String>) {
        self.combat_log.push(line.into());
        if self.combat_log.len() > 64 {
            let drop_count = self.combat_log.len() - 64;
            self.combat_log.drain(0..drop_count);
        }
    }

    fn append_combat_log(&mut self, log: CombatLogEntry) {
        for line in log.lines {
            self.push_log(line);
        }
    }

    /// Phase-checked: select a player unit and enter PlayerMove. Used by
    /// both UI input and the `game.selectUnit` VDP method.
    pub fn select_unit_action(&mut self, id: UnitId) -> Result<(), String> {
        if self.phase != Phase::PlayerSelect {
            return Err(format!("cannot select in phase {}", self.phase.as_str()));
        }
        let u = self
            .units
            .iter()
            .find(|u| u.id == id)
            .ok_or_else(|| format!("no unit with id {id}"))?;
        if !u.alive {
            return Err("unit is dead".into());
        }
        if u.faction != Faction::Player {
            return Err("only player units selectable".into());
        }
        if u.acted {
            return Err("unit already acted this turn".into());
        }
        self.reachable = reachable_tiles(&self.map, &self.units, u);
        self.attackable = attackable_after_move(&self.map, &self.units, u);
        self.pending_action = PendingAction::Selected { unit_id: id };
        self.cursor = u.pos;
        self.phase = Phase::PlayerMove;
        Ok(())
    }

    pub fn move_selected_action(&mut self, dest: GridPos) -> Result<(), String> {
        if self.phase != Phase::PlayerMove {
            return Err(format!("cannot move in phase {}", self.phase.as_str()));
        }
        let PendingAction::Selected { unit_id } = self.pending_action else {
            return Err("no unit selected".into());
        };
        if !self.reachable.contains(&dest) {
            return Err(format!("({},{}) not reachable", dest.x, dest.y));
        }
        let from = self
            .units
            .iter_mut()
            .find(|u| u.id == unit_id)
            .map(|u| {
                let f = u.pos;
                u.pos = dest;
                f
            })
            .ok_or("unit gone")?;
        self.pending_action = PendingAction::Moved {
            unit_id,
            from,
            to: dest,
        };
        let u = self.units.iter().find(|u| u.id == unit_id).unwrap().clone();
        self.attackable = attack_targets_from(&self.units, &u, dest);
        self.reachable.clear();
        self.cursor = dest;
        self.phase = Phase::PlayerAction;
        Ok(())
    }

    pub fn wait_selected_action(&mut self) -> Result<(), String> {
        if self.phase != Phase::PlayerAction {
            return Err(format!("cannot wait in phase {}", self.phase.as_str()));
        }
        let PendingAction::Moved { unit_id, .. } = self.pending_action else {
            return Err("no moved unit".into());
        };
        let name = if let Some(u) = self.units.iter_mut().find(|u| u.id == unit_id) {
            u.acted = true;
            u.name
        } else {
            "?"
        };
        self.push_log(format!("{name} waits"));
        self.end_player_subturn();
        Ok(())
    }

    /// Cancel: PlayerMove -> PlayerSelect (clears selection); PlayerAction
    /// -> PlayerMove (restores moved unit to `from`); PlayerAttackTarget
    /// -> PlayerAction.
    pub fn cancel_action(&mut self) {
        match self.phase {
            Phase::PlayerMove => {
                self.pending_action = PendingAction::None;
                self.reachable.clear();
                self.attackable.clear();
                self.phase = Phase::PlayerSelect;
            }
            Phase::PlayerAction => {
                if let PendingAction::Moved { unit_id, from, .. } = self.pending_action {
                    if let Some(u) = self.units.iter_mut().find(|u| u.id == unit_id) {
                        u.pos = from;
                    }
                    self.pending_action = PendingAction::Selected { unit_id };
                    let u = self.units.iter().find(|u| u.id == unit_id).unwrap().clone();
                    self.reachable = reachable_tiles(&self.map, &self.units, &u);
                    self.attackable = attackable_after_move(&self.map, &self.units, &u);
                    self.cursor = from;
                    self.phase = Phase::PlayerMove;
                }
            }
            Phase::PlayerAttackTarget => {
                if let PendingAction::ChoosingAttack { unit_id, from } = self.pending_action {
                    self.pending_action = PendingAction::Moved {
                        unit_id,
                        from,
                        to: from,
                    };
                    let u = self.units.iter().find(|u| u.id == unit_id).unwrap().clone();
                    self.attackable = attack_targets_from(&self.units, &u, from);
                    self.phase = Phase::PlayerAction;
                }
            }
            _ => {}
        }
    }

    pub fn enter_attack_target_action(&mut self) -> Result<(), String> {
        if self.phase != Phase::PlayerAction {
            return Err(format!("cannot attack in phase {}", self.phase.as_str()));
        }
        let PendingAction::Moved { unit_id, to, .. } = self.pending_action else {
            return Err("no moved unit".into());
        };
        let u = self.units.iter().find(|u| u.id == unit_id).unwrap().clone();
        let targets = attack_targets_from(&self.units, &u, to);
        if targets.is_empty() {
            return Err("no targets in range".into());
        }
        self.attackable = targets;
        self.pending_action = PendingAction::ChoosingAttack { unit_id, from: to };
        self.phase = Phase::PlayerAttackTarget;
        Ok(())
    }

    /// Direct attack — used by both UI flow (after target picked) and the
    /// `game.attack` VDP method. Validates faction/range/alive but not
    /// phase, so tests can stage scenarios without walking the menu.
    pub fn do_attack(&mut self, attacker_id: UnitId, target_id: UnitId) -> Result<(), String> {
        let attacker = self
            .units
            .iter()
            .find(|u| u.id == attacker_id)
            .ok_or_else(|| format!("no attacker {attacker_id}"))?
            .clone();
        let defender = self
            .units
            .iter()
            .find(|u| u.id == target_id)
            .ok_or_else(|| format!("no target {target_id}"))?
            .clone();
        if !attacker.alive || !defender.alive {
            return Err("attacker or defender dead".into());
        }
        if attacker.faction == defender.faction {
            return Err("same faction".into());
        }
        let d = attacker.pos.manhattan(defender.pos);
        if !attacker.weapon_can_reach(d) {
            return Err(format!("target out of range (d={d})"));
        }
        let log = resolve(&self.map, &mut self.units, attacker_id, target_id);
        self.append_combat_log(log);
        // If this attack came from the UI flow, return to PlayerSelect.
        if matches!(
            self.phase,
            Phase::PlayerAttackTarget | Phase::PlayerAction | Phase::PlayerMove
        ) {
            self.end_player_subturn();
        }
        self.check_winner();
        Ok(())
    }

    pub fn preview_combat(
        &self,
        attacker_id: UnitId,
        target_id: UnitId,
    ) -> Result<CombatPreview, String> {
        let a = self
            .units
            .iter()
            .find(|u| u.id == attacker_id)
            .ok_or("no attacker")?;
        let d = self
            .units
            .iter()
            .find(|u| u.id == target_id)
            .ok_or("no target")?;
        Ok(preview(&self.map, a, d))
    }

    fn end_player_subturn(&mut self) {
        self.pending_action = PendingAction::None;
        self.reachable.clear();
        self.attackable.clear();
        let any_left = self
            .units
            .iter()
            .any(|u| u.alive && u.faction == Faction::Player && !u.acted);
        if any_left {
            self.phase = Phase::PlayerSelect;
        } else {
            // Auto-roll into enemy turn so the player isn't stuck staring
            // at greyed-out units.
            self.phase = Phase::EnemyTurn;
        }
    }

    pub fn end_turn_action(&mut self) {
        self.pending_action = PendingAction::None;
        self.reachable.clear();
        self.attackable.clear();
        self.phase = Phase::EnemyTurn;
        self.push_log("Enemy phase");
    }

    fn process_enemy_turn(&mut self) {
        if self.ai_enabled {
            let log = run_enemy_turn(&self.map, &mut self.units);
            self.append_combat_log(log);
        } else {
            // AI off — just reset acted flags and skip the round.
            for u in self.units.iter_mut() {
                u.acted = false;
            }
        }
        self.turn += 1;
        self.phase = Phase::PlayerSelect;
        self.push_log(format!("Player phase (turn {})", self.turn));
        self.check_winner();
    }

    /// Apply one user-issued command. Branches per phase per the plan's
    /// state-machine: confirm in PlayerSelect picks a player unit at the
    /// cursor, in PlayerMove picks a destination, in PlayerAttackTarget
    /// picks an enemy under cursor.
    fn apply_command(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::CursorMoved { cursor } => self.cursor = cursor,
            Cmd::EndTurn => {
                if self.phase == Phase::PlayerSelect {
                    self.end_turn_action();
                }
            }
            Cmd::Cancel => self.cancel_action(),
            Cmd::Confirm { cursor } => {
                self.cursor = cursor;
                match self.phase {
                    Phase::PlayerSelect => {
                        if let Some(u) = self.unit_at(cursor)
                            && u.faction == Faction::Player
                            && !u.acted
                        {
                            let id = u.id;
                            let _ = self.select_unit_action(id);
                        }
                    }
                    Phase::PlayerMove => {
                        if self.reachable.contains(&cursor) {
                            let _ = self.move_selected_action(cursor);
                        }
                    }
                    Phase::PlayerAttackTarget => {
                        if let PendingAction::ChoosingAttack { unit_id, .. } = self.pending_action
                            && let Some(target) = self.unit_at(cursor)
                            && target.faction == Faction::Enemy
                            && self.attackable.contains(&cursor)
                        {
                            let target_id = target.id;
                            let _ = self.do_attack(unit_id, target_id);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

mod render;

impl Game for TacticsDemo {
    fn new(ctx: &mut Context, renderer: &Renderer) -> Self {
        let white_tex = ctx
            .assets
            .register_texture("tactics_white", renderer.create_white_pixel_texture());
        let disc_tex = ctx.assets.register_texture(
            "tactics_disc",
            renderer.create_filled_circle_texture("tactics_disc", 128),
        );
        let ring_tex = ctx.assets.register_texture(
            "tactics_ring",
            renderer.create_ring_texture("tactics_ring", 128, 0.10),
        );
        let map = Map::fixed_level();
        let mut demo = Self {
            phase: Phase::PlayerSelect,
            turn: 1,
            map,
            units: Self::build_units(),
            cursor: GridPos::new(1, 4),
            reachable: Vec::new(),
            attackable: Vec::new(),
            pending_action: PendingAction::None,
            combat_log: vec!["Player phase".into()],
            ai_enabled: true,
            white_tex,
            disc_tex,
            ring_tex,
            ui_intent: UiIntent::default(),
            last_mouse_px: None,
        };
        demo.cursor = demo.units[0].pos;
        demo
    }

    fn update(&mut self, _ctx: &mut Context, _dt: f32, input: &InputState) {
        // Drain UI button intents recorded during the previous frame's
        // `update_ui`. Done first so e.g. clicking "Cancel" right before
        // hitting Esc doesn't double-cancel.
        let intent = std::mem::take(&mut self.ui_intent);
        if intent.cancel {
            self.cancel_action();
        }
        if intent.attack {
            let _ = self.enter_attack_target_action();
        }
        if intent.wait {
            let _ = self.wait_selected_action();
        }
        if intent.end_turn && self.phase == Phase::PlayerSelect {
            self.end_turn_action();
        }

        // Run enemy turn synchronously when we land in EnemyTurn.
        if self.phase == Phase::EnemyTurn {
            self.process_enemy_turn();
        }

        // Process keyboard/mouse for the player phase only.
        if matches!(
            self.phase,
            Phase::PlayerSelect
                | Phase::PlayerMove
                | Phase::PlayerAction
                | Phase::PlayerAttackTarget
        ) {
            let (cursor, mouse_px, cmd) =
                collect_command(input, self.cursor, self.last_mouse_px);
            self.cursor = cursor;
            self.last_mouse_px = mouse_px;
            if let Some(cmd) = cmd {
                self.apply_command(cmd);
            }
        }

        self.check_winner();
    }

    fn update_ui(&mut self, ctx: &mut Context, input: &InputState) {
        render::build_ui(self, ctx, input);
    }

    fn draw(&self, ctx: &Context, screen: &mut Screen) {
        render::draw_world(self, ctx, screen);
    }

    fn clear_color(&self) -> Color {
        Color::from_hex(0x101418)
    }

    #[cfg(feature = "vdp")]
    fn inspect(&self) -> serde_json::Value {
        vdp::inspect(self)
    }

    #[cfg(feature = "vdp")]
    fn handle_vdp(
        &mut self,
        method: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        vdp::handle(self, method, params)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    vibe2d::run::<TacticsDemo>("game.yaml");
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn web_main() {
    wasm_bindgen_futures::spawn_local(async {
        vibe2d::run_web::<TacticsDemo>("game.yaml").await;
    });
}
