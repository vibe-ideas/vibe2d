//! World rendering and UI builder for the tactics demo.
//!
//! Split out of `main.rs` because together they were too noisy to share
//! a file with the state-machine logic. World layer (map / units /
//! highlight tints) goes through `Screen::draw_sprite_tinted`; everything
//! textual lives on the UI layer because `Screen::draw_text` is white-only.

use vibe2d::prelude::*;

use crate::input::TILE_PX;
use crate::map::{MAP_H, MAP_W, TileKind};
use crate::model::{Faction, GridPos, PendingAction, Phase, Unit};
use crate::{HUD_X, MAP_PX_H, MAP_PX_W, MAX_LOG_LINES, TacticsDemo};

const COLOR_PLAIN: u32 = 0x4A6B3C;
const COLOR_ROAD: u32 = 0xA89070;
const COLOR_FOREST: u32 = 0x294A24;
const COLOR_FORT: u32 = 0x5A6E80;
const COLOR_WALL: u32 = 0x303236;
const COLOR_GRID: u32 = 0x222428;
const COLOR_REACH: u32 = 0x4488FF;
const COLOR_ATTACK: u32 = 0xCC4444;
const COLOR_PLAYER: u32 = 0x3E8FE3;
const COLOR_ENEMY: u32 = 0xD25555;
const COLOR_CURSOR: u32 = 0xFFEE55;

fn tile_color(kind: TileKind) -> Color {
    Color::from_hex(match kind {
        TileKind::Plain => COLOR_PLAIN,
        TileKind::Road => COLOR_ROAD,
        TileKind::Forest => COLOR_FOREST,
        TileKind::Fort => COLOR_FORT,
        TileKind::Wall => COLOR_WALL,
    })
}

fn faction_color(f: Faction) -> Color {
    Color::from_hex(match f {
        Faction::Player => COLOR_PLAYER,
        Faction::Enemy => COLOR_ENEMY,
    })
}

fn translucent(hex: u32, alpha: f32) -> Color {
    let mut c = Color::from_hex(hex);
    c.a = alpha;
    c
}

fn selected_unit(demo: &TacticsDemo) -> Option<&Unit> {
    let id = match demo.pending_action {
        PendingAction::Selected { unit_id }
        | PendingAction::Moved { unit_id, .. }
        | PendingAction::ChoosingAttack { unit_id, .. } => unit_id,
        PendingAction::None => return None,
    };
    demo.units.iter().find(|u| u.id == id && u.alive)
}

pub fn draw_world(demo: &TacticsDemo, _ctx: &Context, screen: &mut Screen) {
    let white = demo.white_tex;

    // ── Tiles ──
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            let pos = GridPos::new(x, y);
            let tile = demo.map.tile(pos).unwrap();
            let px = x as f32 * TILE_PX;
            let py = y as f32 * TILE_PX;
            screen.draw_sprite_tinted(white, px, py, TILE_PX, TILE_PX, tile_color(tile.kind));
            // 1 px grid line for legibility.
            screen.draw_sprite_tinted(
                white,
                px,
                py + TILE_PX - 1.0,
                TILE_PX,
                1.0,
                Color::from_hex(COLOR_GRID),
            );
            screen.draw_sprite_tinted(
                white,
                px + TILE_PX - 1.0,
                py,
                1.0,
                TILE_PX,
                Color::from_hex(COLOR_GRID),
            );
        }
    }

    // ── Highlights: reachable (blue) under attackable (red), so the red
    // ring around the threat zone reads on top of the blue movement halo.
    for p in &demo.reachable {
        let (px, py) = (p.x as f32 * TILE_PX, p.y as f32 * TILE_PX);
        screen.draw_sprite_tinted(
            white,
            px,
            py,
            TILE_PX,
            TILE_PX,
            translucent(COLOR_REACH, 0.35),
        );
    }
    for p in &demo.attackable {
        let (px, py) = (p.x as f32 * TILE_PX, p.y as f32 * TILE_PX);
        screen.draw_sprite_tinted(
            white,
            px,
            py,
            TILE_PX,
            TILE_PX,
            translucent(COLOR_ATTACK, 0.40),
        );
    }

    // ── Units (filled disc + faction tint, dim if acted) ──
    for u in &demo.units {
        if !u.alive {
            continue;
        }
        let cx = u.pos.x as f32 * TILE_PX + TILE_PX / 2.0;
        let cy = u.pos.y as f32 * TILE_PX + TILE_PX / 2.0;
        let mut color = faction_color(u.faction);
        if u.acted {
            // Dim tone — multiply RGB by 0.45, leave alpha alone.
            color.r *= 0.45;
            color.g *= 0.45;
            color.b *= 0.45;
        }
        screen.draw_circle(demo.disc_tex, cx, cy, TILE_PX * 0.38, color);

        // HP bar: small horizontal bar above each unit.
        let bar_w = TILE_PX * 0.7;
        let bar_x = cx - bar_w / 2.0;
        let bar_y = cy - TILE_PX * 0.45;
        screen.draw_sprite_tinted(white, bar_x, bar_y, bar_w, 3.0, Color::from_hex(0x202020));
        let frac = (u.hp as f32 / u.max_hp.max(1) as f32).clamp(0.0, 1.0);
        screen.draw_sprite_tinted(
            white,
            bar_x,
            bar_y,
            bar_w * frac,
            3.0,
            Color::from_hex(0x55DD55),
        );
    }

    // ── Selected unit ring + cursor ring ──
    if let Some(uid) = match demo.pending_action {
        PendingAction::Selected { unit_id } => Some(unit_id),
        PendingAction::Moved { unit_id, .. } => Some(unit_id),
        PendingAction::ChoosingAttack { unit_id, .. } => Some(unit_id),
        PendingAction::None => None,
    } && let Some(u) = demo.units.iter().find(|u| u.id == uid && u.alive)
    {
        let cx = u.pos.x as f32 * TILE_PX + TILE_PX / 2.0;
        let cy = u.pos.y as f32 * TILE_PX + TILE_PX / 2.0;
        screen.draw_circle_outline(
            demo.ring_tex,
            cx,
            cy,
            TILE_PX * 0.45,
            Color::from_hex(0xFFFFFF),
        );
    }
    let (cx, cy) = (
        demo.cursor.x as f32 * TILE_PX,
        demo.cursor.y as f32 * TILE_PX,
    );
    // Cursor: 4 small bars forming a square outline so it doesn't obscure
    // the cell contents.
    let c = Color::from_hex(COLOR_CURSOR);
    screen.draw_sprite_tinted(demo.white_tex, cx, cy, TILE_PX, 2.0, c);
    screen.draw_sprite_tinted(demo.white_tex, cx, cy + TILE_PX - 2.0, TILE_PX, 2.0, c);
    screen.draw_sprite_tinted(demo.white_tex, cx, cy, 2.0, TILE_PX, c);
    screen.draw_sprite_tinted(demo.white_tex, cx + TILE_PX - 2.0, cy, 2.0, TILE_PX, c);

    // Vertical divider between map and HUD.
    screen.draw_sprite_tinted(
        white,
        MAP_PX_W,
        0.0,
        2.0,
        MAP_PX_H,
        Color::from_hex(COLOR_GRID),
    );
}

pub fn build_ui(demo: &mut TacticsDemo, ctx: &mut Context, input: &InputState) {
    let vw = ctx.virtual_width;
    let vh = ctx.virtual_height;
    let mut ui_state = std::mem::take(&mut ctx.ui_state);
    let mut ui = UiContext::new(&mut ui_state, input, vw, vh);

    if let Some(font) = ctx.assets.font("title") {
        ui.set_anchor(Anchor::TopLeft);
        ui.set_padding(0.0);
        ui.set_cursor(HUD_X + 12.0, 8.0);
        let header = match demo.phase {
            Phase::Victory => format!("Turn {} - Victory!", demo.turn),
            Phase::Defeat => format!("Turn {} - Defeat", demo.turn),
            Phase::EnemyTurn => format!("Turn {} - Enemy Phase", demo.turn),
            _ => format!("Turn {} - Player Phase", demo.turn),
        };
        ui.label_with_id("turn_header", font, &header);
    }

    if let Some(font) = ctx.assets.font("ui") {
        // Unit info: prefer the actively selected unit (so the panel doesn't
        // go blank just because the cursor wandered onto an empty tile),
        // then fall back to whoever the cursor is hovering.
        ui.set_cursor(HUD_X + 12.0, 48.0);
        ui.set_spacing(2.0);
        let selected = selected_unit(demo);
        let hovered = demo.unit_at(demo.cursor);
        let panel_unit = selected.or(hovered);
        if let Some(u) = panel_unit {
            let header = if selected.map(|s| s.id) == Some(u.id) {
                format!("{} [{}] *", u.name, u.class_name)
            } else {
                format!("{} [{}]", u.name, u.class_name)
            };
            ui.label(font, &header);
            ui.label(font, &format!("HP: {}/{}", u.hp, u.max_hp));
            ui.label(
                font,
                &format!(
                    "Str {} Skl {} Spd {} Def {}",
                    u.strength, u.skill, u.speed, u.defense
                ),
            );
            ui.label(
                font,
                &format!(
                    "Wpn: {} Mt{} Hit{} Rng{}-{}",
                    u.weapon.name,
                    u.weapon.might,
                    u.weapon.hit,
                    u.weapon.min_range,
                    u.weapon.max_range
                ),
            );
        } else {
            ui.label(font, "(empty tile)");
        }

        // Cursor terrain info — useful for evaluating defense / avoid
        // bonuses before committing to a move.
        if let Some(t) = demo.map.tile(demo.cursor) {
            ui.set_cursor(HUD_X + 12.0, 140.0);
            ui.label(
                font,
                &format!(
                    "Terrain: {} (Def+{} Avo+{})",
                    t.kind.as_str(),
                    t.defense_bonus,
                    t.avoid_bonus
                ),
            );
        }

        // Action menu: only in PlayerAction. Stable button IDs for VDP.
        ui.set_cursor(HUD_X + 12.0, 200.0);
        ui.set_spacing(4.0);
        if demo.phase == Phase::PlayerAction
            && let PendingAction::Moved { unit_id, to, .. } = demo.pending_action
        {
            let u = demo.units.iter().find(|u| u.id == unit_id).unwrap().clone();
            let has_targets = !crate::map::attack_targets_from(&demo.units, &u, to).is_empty();
            if has_targets && ui.button_with_id("btn_attack", font, "Attack").clicked {
                demo.ui_intent.attack = true;
            }
            if ui.button_with_id("btn_wait", font, "Wait").clicked {
                demo.ui_intent.wait = true;
            }
            if ui.button_with_id("btn_cancel", font, "Cancel").clicked {
                demo.ui_intent.cancel = true;
            }
        }

        // End turn button visible only during PlayerSelect.
        if demo.phase == Phase::PlayerSelect {
            ui.set_cursor(HUD_X + 12.0, 320.0);
            if ui.button_with_id("btn_end_turn", font, "End Turn").clicked {
                demo.ui_intent.end_turn = true;
            }
        }

        // Combat preview: while in PlayerAttackTarget with the cursor over
        // an attackable enemy, show the projected fight outcome so the
        // player isn't committing blind. Stable IDs prefixed `cp_` so VDP
        // tests can assert the panel rendered.
        if demo.phase == Phase::PlayerAttackTarget
            && let PendingAction::ChoosingAttack { unit_id, .. } = demo.pending_action
            && demo.attackable.contains(&demo.cursor)
            && let Some(target) = demo.unit_at(demo.cursor)
            && target.faction == Faction::Enemy
            && let Ok(p) = demo.preview_combat(unit_id, target.id)
        {
            ui.set_cursor(HUD_X + 12.0, 250.0);
            ui.set_spacing(2.0);
            ui.label_with_id("cp_header", font, &format!("vs {}:", target.name));
            let dmg_line = if p.double_attack {
                format!("Dmg {}x2  Hit {}%", p.damage, p.hit)
            } else {
                format!("Dmg {}  Hit {}%", p.damage, p.hit)
            };
            ui.label_with_id("cp_attack", font, &dmg_line);
            let counter_line = match (p.counter_damage, p.counter_hit) {
                (Some(d), Some(h)) if p.counter_double => {
                    format!("Counter {}x2  Hit {}%", d, h)
                }
                (Some(d), Some(h)) => format!("Counter {}  Hit {}%", d, h),
                _ => "No counter".to_string(),
            };
            ui.label_with_id("cp_counter", font, &counter_line);
        }

        // Combat log.
        ui.set_cursor(HUD_X + 12.0, 380.0);
        ui.set_spacing(2.0);
        ui.label_with_id("combat_log", font, "Log:");
        let start = demo.combat_log.len().saturating_sub(MAX_LOG_LINES);
        for line in &demo.combat_log[start..] {
            ui.label(font, line);
        }
    }

    ui.finish();
    ctx.ui_state = ui_state;
}
