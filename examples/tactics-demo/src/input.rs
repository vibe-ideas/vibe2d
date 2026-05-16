//! Mouse + keyboard → high-level game commands.
//!
//! Lives apart from the game struct so the action selection rules can be
//! unit-tested without spinning up a Context. The actual application of
//! a `Cmd` to game state happens in `main.rs`.

use vibe2d::prelude::InputState;

use crate::map::{MAP_H, MAP_W};
use crate::model::GridPos;

pub const TILE_PX: f32 = 40.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// User confirmed (Enter / Space / Left-click) at `cursor` — meaning
    /// depends on the current phase (select unit, pick destination, etc.).
    Confirm { cursor: GridPos },
    /// User cancelled (Esc / Right-click). Phase determines what unwinds.
    Cancel,
    /// User asked to end the player phase.
    EndTurn,
    /// User moved cursor (keyboard or mouse). UI uses this for hover only.
    CursorMoved { cursor: GridPos },
}

/// Convert a virtual-pixel mouse coordinate to a grid cell, clamping to
/// the map rect and returning `None` if the cursor is fully outside the
/// map area (right HUD, etc.).
pub fn mouse_to_grid(mx: f32, my: f32) -> Option<GridPos> {
    if mx < 0.0 || my < 0.0 {
        return None;
    }
    let gx = (mx / TILE_PX).floor() as i32;
    let gy = (my / TILE_PX).floor() as i32;
    if !(0..MAP_W).contains(&gx) || !(0..MAP_H).contains(&gy) {
        return None;
    }
    Some(GridPos::new(gx, gy))
}

/// Inspect this frame's input and produce at most one game command, plus
/// the latest cursor grid and the latest mouse pixel position. Keyboard
/// navigation moves the cursor; the mouse only snaps the cursor when it
/// has actually moved this frame — otherwise the engine's default-zero
/// mouse position would clobber any cursor set by VDP / select actions.
pub fn collect_command(
    input: &InputState,
    prev_cursor: GridPos,
    prev_mouse_px: Option<(f32, f32)>,
) -> (GridPos, Option<(f32, f32)>, Option<Cmd>) {
    let mut cursor = prev_cursor;

    // Snap to grid only when the pointer pixel actually moved since last
    // frame. First frame (`prev_mouse_px == None`) just records position
    // without moving the cursor.
    let cur_mouse = input.mouse_position();
    let mouse_changed = prev_mouse_px.is_some_and(|(px, py)| (px, py) != cur_mouse);
    if mouse_changed && let Some(g) = mouse_to_grid(cur_mouse.0, cur_mouse.1) {
        cursor = g;
    }
    let new_mouse_px = Some(cur_mouse);

    // Keyboard cursor movement.
    if input.is_action_just_pressed("cursor_left") {
        cursor.x = (cursor.x - 1).max(0);
    }
    if input.is_action_just_pressed("cursor_right") {
        cursor.x = (cursor.x + 1).min(MAP_W - 1);
    }
    if input.is_action_just_pressed("cursor_up") {
        cursor.y = (cursor.y - 1).max(0);
    }
    if input.is_action_just_pressed("cursor_down") {
        cursor.y = (cursor.y + 1).min(MAP_H - 1);
    }

    // Order: cancel beats confirm beats end-turn so right-click never
    // accidentally confirms when both fire on the same frame.
    if input.is_action_just_pressed("cancel") {
        return (cursor, new_mouse_px, Some(Cmd::Cancel));
    }
    if input.is_action_just_pressed("confirm") {
        return (cursor, new_mouse_px, Some(Cmd::Confirm { cursor }));
    }
    if input.is_action_just_pressed("end_turn") {
        return (cursor, new_mouse_px, Some(Cmd::EndTurn));
    }
    if cursor != prev_cursor {
        return (cursor, new_mouse_px, Some(Cmd::CursorMoved { cursor }));
    }
    (cursor, new_mouse_px, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_inside_map_maps_to_cell() {
        // (0, 0) -> (0, 0); inside cell of (3, 4).
        assert_eq!(mouse_to_grid(0.0, 0.0), Some(GridPos::new(0, 0)));
        let g = mouse_to_grid(3.0 * TILE_PX + 5.0, 4.0 * TILE_PX + 1.0).unwrap();
        assert_eq!(g, GridPos::new(3, 4));
    }

    #[test]
    fn mouse_outside_map_returns_none() {
        assert!(mouse_to_grid(-1.0, 0.0).is_none());
        assert!(mouse_to_grid(0.0, -1.0).is_none());
        assert!(mouse_to_grid(MAP_W as f32 * TILE_PX + 1.0, 0.0).is_none());
        assert!(mouse_to_grid(0.0, MAP_H as f32 * TILE_PX + 1.0).is_none());
    }
}
