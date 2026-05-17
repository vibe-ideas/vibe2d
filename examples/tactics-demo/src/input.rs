//! Map mouse/keyboard input to game commands.

use crate::model::GridPos;
use vibe2d::prelude::InputState;

pub const TILE_SIZE: f32 = 40.0;
pub const MAP_OFFSET_X: f32 = 20.0;
pub const MAP_OFFSET_Y: f32 = 80.0;

/// Convert screen coordinates to grid position.
pub fn screen_to_grid(sx: f32, sy: f32) -> GridPos {
    let gx = ((sx - MAP_OFFSET_X) / TILE_SIZE).floor() as i32;
    let gy = ((sy - MAP_OFFSET_Y) / TILE_SIZE).floor() as i32;
    GridPos::new(gx, gy)
}

/// Convert grid position to screen coordinates (top-left of tile).
pub fn grid_to_screen(pos: GridPos) -> (f32, f32) {
    let sx = MAP_OFFSET_X + pos.x as f32 * TILE_SIZE;
    let sy = MAP_OFFSET_Y + pos.y as f32 * TILE_SIZE;
    (sx, sy)
}

/// Returns the grid tile the mouse is currently hovering over.
pub fn mouse_hover_grid(input: &InputState) -> GridPos {
    let (mx, my) = input.mouse_position();
    screen_to_grid(mx, my)
}

/// Returns true if a left click occurred this frame.
pub fn left_clicked(input: &InputState) -> bool {
    input.is_action_just_pressed("confirm")
}

/// Returns true if a right click / cancel occurred this frame.
pub fn cancel_pressed(input: &InputState) -> bool {
    input.is_action_just_pressed("cancel")
}

/// Returns true if end_turn was pressed.
pub fn end_turn_pressed(input: &InputState) -> bool {
    input.is_action_just_pressed("end_turn")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_to_grid_origin() {
        let g = screen_to_grid(MAP_OFFSET_X + 1.0, MAP_OFFSET_Y + 1.0);
        assert_eq!(g, GridPos::new(0, 0));
    }

    #[test]
    fn screen_to_grid_second_tile() {
        let g = screen_to_grid(MAP_OFFSET_X + TILE_SIZE + 1.0, MAP_OFFSET_Y + 1.0);
        assert_eq!(g, GridPos::new(1, 0));
    }

    #[test]
    fn grid_to_screen_origin() {
        let (sx, sy) = grid_to_screen(GridPos::new(0, 0));
        assert!((sx - MAP_OFFSET_X).abs() < 0.001);
        assert!((sy - MAP_OFFSET_Y).abs() < 0.001);
    }

    #[test]
    fn roundtrip_grid_screen() {
        let pos = GridPos::new(5, 3);
        let (sx, sy) = grid_to_screen(pos);
        let back = screen_to_grid(sx + 1.0, sy + 1.0);
        assert_eq!(back, pos);
    }
}
