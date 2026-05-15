use vibe2d::prelude::*;
mod ai;
mod combat;
mod input;
mod map;
mod model;
#[cfg(feature = "vdp")]
mod vdp;

use combat::{CombatPreview, preview_combat, resolve_combat};
use input::{
    TILE_SIZE, cancel_pressed, end_turn_pressed, grid_to_screen, left_clicked, mouse_hover_grid,
};
use map::{MAP_H, MAP_W, Map, attackable_from, attackable_targets, build_map, reachable_tiles};
use model::*;

const HUD_X: f32 = 600.0;
const LOG_MAX: usize = 8;

pub struct TacticsGame {
    pub phase: Phase,
    pub turn: u32,
    pub map: Map,
    pub units: Vec<Unit>,
    pub selected: Option<UnitId>,
    pub cursor: GridPos,
    pub reachable: Vec<GridPos>,
    pub attackable: Vec<GridPos>,
    pub pending_action: PendingAction,
    pub combat_log: Vec<String>,
    pub ai_enabled: bool,
    white_tex: TextureId,
    unit_disc_tex: TextureId,
    unit_ring_tex: TextureId,
}

impl TacticsGame {
    fn new_with_textures(
        white_tex: TextureId,
        unit_disc_tex: TextureId,
        unit_ring_tex: TextureId,
    ) -> Self {
        let map = build_map();
        let units = Self::build_units();
        Self {
            phase: Phase::PlayerSelect,
            turn: 1,
            map,
            units,
            selected: None,
            cursor: GridPos::new(1, 1),
            reachable: Vec::new(),
            attackable: Vec::new(),
            pending_action: PendingAction::None,
            combat_log: vec!["Turn 1 - Player Phase".to_string()],
            ai_enabled: true,
            white_tex,
            unit_disc_tex,
            unit_ring_tex,
        }
    }

    pub fn reset_state(&mut self) {
        let w = self.white_tex;
        let d = self.unit_disc_tex;
        let r = self.unit_ring_tex;
        *self = Self::new_with_textures(w, d, r);
    }

    fn build_units() -> Vec<Unit> {
        vec![
            Unit {
                id: 1,
                name: "Alen",
                class_name: "Fighter",
                faction: Faction::Player,
                pos: GridPos::new(1, 6),
                hp: 22,
                max_hp: 22,
                strength: 8,
                skill: 6,
                speed: 7,
                defense: 5,
                move_range: 5,
                weapon: Weapon {
                    name: "Sword",
                    might: 5,
                    hit: 80,
                    min_range: 1,
                    max_range: 1,
                },
                acted: false,
                alive: true,
            },
            Unit {
                id: 2,
                name: "Bram",
                class_name: "Archer",
                faction: Faction::Player,
                pos: GridPos::new(1, 4),
                hp: 18,
                max_hp: 18,
                strength: 7,
                skill: 8,
                speed: 8,
                defense: 3,
                move_range: 5,
                weapon: Weapon {
                    name: "Bow",
                    might: 5,
                    hit: 75,
                    min_range: 2,
                    max_range: 2,
                },
                acted: false,
                alive: true,
            },
            Unit {
                id: 3,
                name: "Clara",
                class_name: "Knight",
                faction: Faction::Player,
                pos: GridPos::new(2, 7),
                hp: 28,
                max_hp: 28,
                strength: 9,
                skill: 5,
                speed: 5,
                defense: 8,
                move_range: 4,
                weapon: Weapon {
                    name: "Lance",
                    might: 6,
                    hit: 70,
                    min_range: 1,
                    max_range: 1,
                },
                acted: false,
                alive: true,
            },
            Unit {
                id: 4,
                name: "Dania",
                class_name: "Mage",
                faction: Faction::Player,
                pos: GridPos::new(1, 8),
                hp: 15,
                max_hp: 15,
                strength: 10,
                skill: 9,
                speed: 9,
                defense: 2,
                move_range: 5,
                weapon: Weapon {
                    name: "Thunder",
                    might: 7,
                    hit: 85,
                    min_range: 1,
                    max_range: 2,
                },
                acted: false,
                alive: true,
            },
            Unit {
                id: 5,
                name: "Brigand1",
                class_name: "Brigand",
                faction: Faction::Enemy,
                pos: GridPos::new(11, 2),
                hp: 18,
                max_hp: 18,
                strength: 7,
                skill: 4,
                speed: 5,
                defense: 4,
                move_range: 4,
                weapon: Weapon {
                    name: "Axe",
                    might: 6,
                    hit: 70,
                    min_range: 1,
                    max_range: 1,
                },
                acted: false,
                alive: true,
            },
            Unit {
                id: 6,
                name: "Brigand2",
                class_name: "Brigand",
                faction: Faction::Enemy,
                pos: GridPos::new(10, 4),
                hp: 18,
                max_hp: 18,
                strength: 7,
                skill: 4,
                speed: 5,
                defense: 4,
                move_range: 4,
                weapon: Weapon {
                    name: "Axe",
                    might: 6,
                    hit: 70,
                    min_range: 1,
                    max_range: 1,
                },
                acted: false,
                alive: true,
            },
            Unit {
                id: 7,
                name: "Brigand3",
                class_name: "Brigand",
                faction: Faction::Enemy,
                pos: GridPos::new(11, 5),
                hp: 18,
                max_hp: 18,
                strength: 7,
                skill: 4,
                speed: 5,
                defense: 4,
                move_range: 4,
                weapon: Weapon {
                    name: "Axe",
                    might: 6,
                    hit: 70,
                    min_range: 1,
                    max_range: 1,
                },
                acted: false,
                alive: true,
            },
            Unit {
                id: 8,
                name: "Brigand4",
                class_name: "Brigand",
                faction: Faction::Enemy,
                pos: GridPos::new(12, 7),
                hp: 18,
                max_hp: 18,
                strength: 7,
                skill: 4,
                speed: 5,
                defense: 4,
                move_range: 4,
                weapon: Weapon {
                    name: "Axe",
                    might: 6,
                    hit: 70,
                    min_range: 1,
                    max_range: 1,
                },
                acted: false,
                alive: true,
            },
            Unit {
                id: 9,
                name: "Brigand5",
                class_name: "Brigand",
                faction: Faction::Enemy,
                pos: GridPos::new(10, 8),
                hp: 18,
                max_hp: 18,
                strength: 7,
                skill: 4,
                speed: 5,
                defense: 4,
                move_range: 4,
                weapon: Weapon {
                    name: "Axe",
                    might: 6,
                    hit: 70,
                    min_range: 1,
                    max_range: 1,
                },
                acted: false,
                alive: true,
            },
            Unit {
                id: 10,
                name: "Captain",
                class_name: "Bandit Chief",
                faction: Faction::Enemy,
                pos: GridPos::new(12, 5),
                hp: 28,
                max_hp: 28,
                strength: 10,
                skill: 7,
                speed: 6,
                defense: 6,
                move_range: 4,
                weapon: Weapon {
                    name: "GreatAxe",
                    might: 9,
                    hit: 65,
                    min_range: 1,
                    max_range: 1,
                },
                acted: false,
                alive: true,
            },
        ]
    }

    /// Select a unit and compute reachable/attackable tiles.
    pub fn select_unit(&mut self, id: UnitId) -> Result<(), String> {
        let u = self
            .units
            .iter()
            .find(|u| u.id == id)
            .ok_or_else(|| format!("Unit {} not found", id))?;
        if !u.alive {
            return Err(format!("Unit {} is dead", id));
        }
        if u.faction != Faction::Player {
            return Err(format!("Unit {} is not a player unit", id));
        }
        if u.acted {
            return Err(format!("Unit {} has already acted", id));
        }

        self.selected = Some(id);
        let pos = u.pos;
        let mr = u.move_range;
        let wmin = u.weapon.min_range;
        let wmax = u.weapon.max_range;
        self.reachable = reachable_tiles(&self.map, pos, mr, &self.units, id, Faction::Player);
        self.attackable = attackable_targets(
            &self.reachable,
            &self.units,
            id,
            Faction::Player,
            wmin,
            wmax,
        );
        self.pending_action = PendingAction::Selected { unit_id: id };
        self.phase = Phase::PlayerMove;
        Ok(())
    }

    /// Move the selected unit to dest.
    pub fn move_selected(&mut self, dest: GridPos) -> Result<(), String> {
        let id = self.selected.ok_or("No unit selected")?;
        if !self.reachable.contains(&dest) {
            return Err(format!("({},{}) not reachable", dest.x, dest.y));
        }
        if self
            .units
            .iter()
            .any(|u| u.alive && u.id != id && u.pos == dest)
        {
            return Err(format!("({},{}) occupied", dest.x, dest.y));
        }
        let idx = self.units.iter().position(|u| u.id == id).unwrap();
        let from = self.units[idx].pos;
        self.units[idx].pos = dest;
        self.pending_action = PendingAction::Moved {
            unit_id: id,
            from,
            to: dest,
        };
        self.phase = Phase::PlayerAction;
        Ok(())
    }

    /// Wait/skip action for selected unit.
    pub fn wait_selected(&mut self) -> Result<(), String> {
        let id = self.selected.ok_or("No unit selected")?;
        let idx = self.units.iter().position(|u| u.id == id).unwrap();
        self.units[idx].acted = true;
        self.selected = None;
        self.reachable.clear();
        self.attackable.clear();
        self.pending_action = PendingAction::None;
        self.phase = Phase::PlayerSelect;
        self.check_turn_end();
        Ok(())
    }

    /// Execute an attack from the current phase context.
    pub fn do_attack(&mut self, attacker_id: UnitId, target_id: UnitId) -> Result<(), String> {
        let atk = self
            .units
            .iter()
            .find(|u| u.id == attacker_id)
            .ok_or("attacker not found")?;
        if !atk.alive {
            return Err("attacker dead".to_string());
        }
        let def = self
            .units
            .iter()
            .find(|u| u.id == target_id)
            .ok_or("target not found")?;
        if !def.alive {
            return Err("target dead".to_string());
        }
        if atk.faction == def.faction {
            return Err("cannot attack same faction".to_string());
        }
        let atk_pos = atk.pos;
        if !atk.can_attack_from(atk_pos, def.pos) {
            return Err("target out of range".to_string());
        }
        let def_pos = def.pos;
        let def_tile_def = self.map.tile(def_pos).map(|t| t.defense_bonus).unwrap_or(0);
        let def_tile_avoid = self.map.tile(def_pos).map(|t| t.avoid_bonus).unwrap_or(0);
        let atk_tile_def = self.map.tile(atk_pos).map(|t| t.defense_bonus).unwrap_or(0);
        let atk_tile_avoid = self.map.tile(atk_pos).map(|t| t.avoid_bonus).unwrap_or(0);
        let log = resolve_combat(
            &mut self.units,
            attacker_id,
            target_id,
            def_tile_def,
            def_tile_avoid,
            atk_tile_def,
            atk_tile_avoid,
            atk_pos,
        );
        for entry in log {
            self.push_log(entry);
        }
        self.check_win_loss();
        if self.phase == Phase::PlayerAttackTarget {
            self.selected = None;
            self.reachable.clear();
            self.attackable.clear();
            self.pending_action = PendingAction::None;
            self.phase = Phase::PlayerSelect;
            self.check_turn_end();
        }
        Ok(())
    }

    /// Preview combat without modifying state.
    pub fn preview_combat(
        &self,
        attacker_id: UnitId,
        target_id: UnitId,
    ) -> Result<CombatPreview, String> {
        let atk = self
            .units
            .iter()
            .find(|u| u.id == attacker_id)
            .ok_or("attacker not found")?;
        let def = self
            .units
            .iter()
            .find(|u| u.id == target_id)
            .ok_or("target not found")?;
        if !atk.alive {
            return Err("attacker dead".to_string());
        }
        if !def.alive {
            return Err("target dead".to_string());
        }
        let atk_pos = atk.pos;
        let def_pos = def.pos;
        let def_tile_def = self.map.tile(def_pos).map(|t| t.defense_bonus).unwrap_or(0);
        let def_tile_avoid = self.map.tile(def_pos).map(|t| t.avoid_bonus).unwrap_or(0);
        let atk_tile_def = self.map.tile(atk_pos).map(|t| t.defense_bonus).unwrap_or(0);
        let atk_tile_avoid = self.map.tile(atk_pos).map(|t| t.avoid_bonus).unwrap_or(0);
        Ok(preview_combat(
            atk,
            def,
            atk_tile_def,
            atk_tile_avoid,
            def_tile_def,
            def_tile_avoid,
            atk_pos,
        ))
    }

    /// End player turn and run enemy phase.
    pub fn end_player_turn(&mut self) {
        self.selected = None;
        self.reachable.clear();
        self.attackable.clear();
        self.pending_action = PendingAction::None;
        self.phase = Phase::EnemyTurn;
        self.push_log("Enemy Phase".to_string());
        if self.ai_enabled {
            self.run_enemy_phase();
        } else {
            // Still need to reset and advance turn even without AI
            self.finish_enemy_phase();
        }
    }

    fn run_enemy_phase(&mut self) {
        let log = ai::run_enemy_phase(&self.map, &mut self.units);
        for entry in log {
            self.push_log(entry);
        }
        self.check_win_loss();
        self.finish_enemy_phase();
    }

    fn finish_enemy_phase(&mut self) {
        if self.phase != Phase::Victory && self.phase != Phase::Defeat {
            for u in &mut self.units {
                u.acted = false;
            }
            self.turn += 1;
            self.phase = Phase::PlayerSelect;
            self.push_log(format!("Turn {} - Player Phase", self.turn));
        }
    }

    fn check_turn_end(&mut self) {
        let all_acted = self
            .units
            .iter()
            .filter(|u| u.alive && u.faction == Faction::Player)
            .all(|u| u.acted);
        if all_acted {
            self.end_player_turn();
        }
    }

    fn check_win_loss(&mut self) {
        let all_enemies_dead = self
            .units
            .iter()
            .filter(|u| u.faction == Faction::Enemy)
            .all(|u| !u.alive);
        let all_players_dead = self
            .units
            .iter()
            .filter(|u| u.faction == Faction::Player)
            .all(|u| !u.alive);
        if all_enemies_dead {
            self.phase = Phase::Victory;
            self.push_log("Victory! All enemies defeated!".to_string());
        } else if all_players_dead {
            self.phase = Phase::Defeat;
            self.push_log("Defeat! All player units lost!".to_string());
        }
    }

    fn push_log(&mut self, entry: String) {
        self.combat_log.push(entry);
        if self.combat_log.len() > LOG_MAX {
            let excess = self.combat_log.len() - LOG_MAX;
            self.combat_log.drain(0..excess);
        }
    }

    fn tile_color(&self, pos: GridPos) -> Color {
        match self.map.tile_kind(pos) {
            Some(TileKind::Plain) => Color {
                r: 0.35,
                g: 0.55,
                b: 0.25,
                a: 1.0,
            },
            Some(TileKind::Road) => Color {
                r: 0.65,
                g: 0.58,
                b: 0.45,
                a: 1.0,
            },
            Some(TileKind::Forest) => Color {
                r: 0.18,
                g: 0.38,
                b: 0.18,
                a: 1.0,
            },
            Some(TileKind::Fort) => Color {
                r: 0.45,
                g: 0.50,
                b: 0.60,
                a: 1.0,
            },
            Some(TileKind::Wall) => Color {
                r: 0.28,
                g: 0.28,
                b: 0.28,
                a: 1.0,
            },
            None => Color {
                r: 0.1,
                g: 0.1,
                b: 0.1,
                a: 1.0,
            },
        }
    }

    fn handle_cancel(&mut self) {
        match self.phase {
            Phase::PlayerMove => {
                self.selected = None;
                self.reachable.clear();
                self.attackable.clear();
                self.pending_action = PendingAction::None;
                self.phase = Phase::PlayerSelect;
            }
            Phase::PlayerAction => {
                if let PendingAction::Moved { unit_id, from, .. } = self.pending_action {
                    if let Some(idx) = self.units.iter().position(|u| u.id == unit_id) {
                        self.units[idx].pos = from;
                        let mr = self.units[idx].move_range;
                        let wmin = self.units[idx].weapon.min_range;
                        let wmax = self.units[idx].weapon.max_range;
                        self.reachable = reachable_tiles(
                            &self.map,
                            from,
                            mr,
                            &self.units,
                            unit_id,
                            Faction::Player,
                        );
                        self.attackable = attackable_targets(
                            &self.reachable,
                            &self.units,
                            unit_id,
                            Faction::Player,
                            wmin,
                            wmax,
                        );
                        self.pending_action = PendingAction::Selected { unit_id };
                    }
                }
                self.phase = Phase::PlayerMove;
            }
            Phase::PlayerAttackTarget => {
                self.phase = Phase::PlayerAction;
                self.attackable.clear();
            }
            _ => {}
        }
    }

    fn handle_click(&mut self, pos: GridPos) {
        match self.phase {
            Phase::PlayerSelect => {
                let unit_at = self
                    .units
                    .iter()
                    .find(|u| u.alive && u.pos == pos && u.faction == Faction::Player && !u.acted)
                    .map(|u| u.id);
                if let Some(id) = unit_at {
                    let _ = self.select_unit(id);
                }
            }
            Phase::PlayerMove => {
                if self.reachable.contains(&pos) {
                    if !self
                        .units
                        .iter()
                        .any(|u| u.alive && u.pos == pos && self.selected != Some(u.id))
                    {
                        let _ = self.move_selected(pos);
                    }
                } else {
                    self.handle_cancel();
                }
            }
            Phase::PlayerAttackTarget => {
                let target_id = self
                    .units
                    .iter()
                    .find(|u| u.alive && u.pos == pos && u.faction == Faction::Enemy)
                    .map(|u| u.id);
                if let Some(tid) = target_id {
                    if let Some(aid) = self.selected {
                        let _ = self.do_attack(aid, tid);
                    }
                }
            }
            _ => {}
        }
    }
}

impl Game for TacticsGame {
    fn new(ctx: &mut Context, renderer: &Renderer) -> Self {
        let white_tex = ctx
            .assets
            .register_texture("__tactics_white", renderer.create_white_pixel_texture());
        let unit_disc_tex = ctx.assets.register_texture(
            "__tactics_disc",
            renderer.create_filled_circle_texture("unit_disc", 64),
        );
        let unit_ring_tex = ctx.assets.register_texture(
            "__tactics_ring",
            renderer.create_ring_texture("unit_ring", 64, 0.12),
        );
        TacticsGame::new_with_textures(white_tex, unit_disc_tex, unit_ring_tex)
    }

    fn update(&mut self, _ctx: &mut Context, _dt: f32, input: &InputState) {
        if self.phase == Phase::Victory || self.phase == Phase::Defeat {
            return;
        }
        if self.phase == Phase::EnemyTurn {
            return;
        }

        let hover = mouse_hover_grid(input);
        if self.map.in_bounds(hover) {
            self.cursor = hover;
        }

        if cancel_pressed(input) {
            self.handle_cancel();
        }

        if end_turn_pressed(input) && self.phase == Phase::PlayerSelect {
            self.end_player_turn();
            return;
        }

        if left_clicked(input) {
            self.handle_click(self.cursor);
        }
    }

    fn update_ui(&mut self, ctx: &mut Context, input: &InputState) {
        let mut ui_state = std::mem::take(&mut ctx.ui_state);
        let mut ui = UiContext::new(&mut ui_state, input, 960.0, 640.0);

        // Turn / Phase header
        ui.set_anchor(Anchor::TopLeft);
        ui.set_cursor(HUD_X, 10.0);
        if let Some(font) = ctx.assets.font("title") {
            let phase_str = match self.phase {
                Phase::PlayerSelect
                | Phase::PlayerMove
                | Phase::PlayerAction
                | Phase::PlayerAttackTarget => format!("Turn {} - Player", self.turn),
                Phase::EnemyTurn => format!("Turn {} - Enemy", self.turn),
                Phase::Victory => "VICTORY!".to_string(),
                Phase::Defeat => "DEFEAT!".to_string(),
                Phase::Title => "Tactics Demo".to_string(),
            };
            ui.label(font, &phase_str);
        }

        // Selected unit info
        ui.set_cursor(HUD_X, 50.0);
        if let Some(uid) = self.selected {
            if let Some(u) = self.units.iter().find(|u| u.id == uid) {
                if let Some(font) = ctx.assets.font("ui") {
                    ui.label(font, &format!("{} ({})", u.name, u.class_name));
                    ui.label(font, &format!("HP: {}/{}", u.hp, u.max_hp));
                    ui.label(
                        font,
                        &format!("Str:{} Spd:{} Def:{}", u.strength, u.speed, u.defense),
                    );
                    ui.label(
                        font,
                        &format!(
                            "Wpn: {} ({}~{})",
                            u.weapon.name, u.weapon.min_range, u.weapon.max_range
                        ),
                    );
                }
            }
        } else if let Some(tile) = self.map.tile(self.cursor) {
            if let Some(font) = ctx.assets.font("small") {
                let kind_str = match tile.kind {
                    TileKind::Plain => "Plain",
                    TileKind::Road => "Road",
                    TileKind::Forest => "Forest",
                    TileKind::Fort => "Fort",
                    TileKind::Wall => "Wall",
                };
                ui.label(
                    font,
                    &format!(
                        "Tile: {} Def+{} Avo+{}",
                        kind_str, tile.defense_bonus, tile.avoid_bonus
                    ),
                );
            }
        }

        // Action menu (PlayerAction phase)
        if self.phase == Phase::PlayerAction {
            ui.set_cursor(HUD_X, 200.0);
            if let Some(font) = ctx.assets.font("ui") {
                if let PendingAction::Moved { unit_id, to, .. } = self.pending_action {
                    let wmin = self
                        .units
                        .iter()
                        .find(|u| u.id == unit_id)
                        .map(|u| u.weapon.min_range)
                        .unwrap_or(1);
                    let wmax = self
                        .units
                        .iter()
                        .find(|u| u.id == unit_id)
                        .map(|u| u.weapon.max_range)
                        .unwrap_or(1);
                    let targets =
                        attackable_from(to, &self.units, unit_id, Faction::Player, wmin, wmax);
                    if !targets.is_empty() {
                        if ui.button_with_id("btn_attack", font, "Attack").clicked() {
                            self.phase = Phase::PlayerAttackTarget;
                            self.attackable = self
                                .units
                                .iter()
                                .filter(|u| targets.contains(&u.id))
                                .map(|u| u.pos)
                                .collect();
                        }
                    }
                }
                if ui.button_with_id("btn_wait", font, "Wait").clicked() {
                    let _ = self.wait_selected();
                }
                if ui.button_with_id("btn_cancel", font, "Cancel").clicked() {
                    self.handle_cancel();
                }
            }
        }

        // End Turn button
        if self.phase == Phase::PlayerSelect {
            ui.set_cursor(HUD_X, 300.0);
            if let Some(font) = ctx.assets.font("ui") {
                if ui
                    .button_with_id("btn_end_turn", font, "End Turn")
                    .clicked()
                {
                    self.end_player_turn();
                }
            }
        }

        // Combat Log
        ui.set_cursor(HUD_X, 400.0);
        if let Some(font) = ctx.assets.font("small") {
            ui.label(font, "--- Log ---");
            for entry in &self.combat_log {
                ui.label(font, entry);
            }
        }

        ui.finish();
        ctx.ui_state = ui_state;
    }

    fn draw(&self, _ctx: &Context, screen: &mut Screen) {
        // Draw map tiles
        for y in 0..MAP_H {
            for x in 0..MAP_W {
                let pos = GridPos::new(x, y);
                let (sx, sy) = grid_to_screen(pos);
                let color = self.tile_color(pos);
                screen.draw_sprite_tinted(
                    self.white_tex,
                    sx,
                    sy,
                    TILE_SIZE - 1.0,
                    TILE_SIZE - 1.0,
                    color,
                );
            }
        }

        // Reachable highlight (blue semi-transparent)
        let reach_color = Color {
            r: 0.2,
            g: 0.4,
            b: 1.0,
            a: 0.35,
        };
        for &pos in &self.reachable {
            let (sx, sy) = grid_to_screen(pos);
            screen.draw_sprite_tinted(
                self.white_tex,
                sx,
                sy,
                TILE_SIZE - 1.0,
                TILE_SIZE - 1.0,
                reach_color,
            );
        }

        // Attackable highlight (red semi-transparent)
        let atk_color = Color {
            r: 1.0,
            g: 0.2,
            b: 0.2,
            a: 0.35,
        };
        for &pos in &self.attackable {
            let (sx, sy) = grid_to_screen(pos);
            screen.draw_sprite_tinted(
                self.white_tex,
                sx,
                sy,
                TILE_SIZE - 1.0,
                TILE_SIZE - 1.0,
                atk_color,
            );
        }

        // Cursor highlight
        let cursor_color = Color {
            r: 1.0,
            g: 1.0,
            b: 0.3,
            a: 0.5,
        };
        let (cx, cy) = grid_to_screen(self.cursor);
        screen.draw_sprite_tinted(
            self.white_tex,
            cx,
            cy,
            TILE_SIZE - 1.0,
            TILE_SIZE - 1.0,
            cursor_color,
        );

        // Draw units
        for u in &self.units {
            if !u.alive {
                continue;
            }
            let (sx, sy) = grid_to_screen(u.pos);
            let cx = sx + TILE_SIZE / 2.0;
            let cy = sy + TILE_SIZE / 2.0;
            let radius = TILE_SIZE * 0.38;

            let base_color = if u.faction == Faction::Player {
                Color {
                    r: 0.2,
                    g: 0.4,
                    b: 1.0,
                    a: 1.0,
                }
            } else {
                Color {
                    r: 1.0,
                    g: 0.2,
                    b: 0.2,
                    a: 1.0,
                }
            };
            let color = if u.acted {
                Color {
                    r: base_color.r * 0.5,
                    g: base_color.g * 0.5,
                    b: base_color.b * 0.5,
                    a: 0.7,
                }
            } else {
                base_color
            };
            screen.draw_circle(self.unit_disc_tex, cx, cy, radius, color);

            if self.selected == Some(u.id) {
                screen.draw_circle_outline(self.unit_ring_tex, cx, cy, radius + 3.0, Color::WHITE);
            }

            // HP bar
            let hp_ratio = u.hp as f32 / u.max_hp as f32;
            let bar_w = TILE_SIZE - 4.0;
            let bar_h = 4.0;
            let bar_x = sx + 2.0;
            let bar_y = sy + TILE_SIZE - 6.0;
            screen.draw_sprite_tinted(
                self.white_tex,
                bar_x,
                bar_y,
                bar_w,
                bar_h,
                Color {
                    r: 0.2,
                    g: 0.0,
                    b: 0.0,
                    a: 0.8,
                },
            );
            screen.draw_sprite_tinted(
                self.white_tex,
                bar_x,
                bar_y,
                bar_w * hp_ratio,
                bar_h,
                Color {
                    r: 0.1,
                    g: 0.9,
                    b: 0.1,
                    a: 1.0,
                },
            );
        }
    }

    fn clear_color(&self) -> Color {
        Color {
            r: 0.08,
            g: 0.08,
            b: 0.12,
            a: 1.0,
        }
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
        vdp::handle_vdp(self, method, params)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    vibe2d::run::<TacticsGame>("game.yaml");
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn web_main() {
    wasm_bindgen_futures::spawn_local(async {
        vibe2d::run_web::<TacticsGame>("game.yaml").await;
    });
}
