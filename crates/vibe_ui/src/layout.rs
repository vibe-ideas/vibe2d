/// Anchor position for UI content placement on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Direction in which widgets are stacked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutDirection {
    #[default]
    Vertical,
    Horizontal,
}

/// Compute the top-left origin for a block of given size, placed at the given anchor
/// within a virtual screen of (screen_w, screen_h) with padding.
pub fn anchor_origin(
    anchor: Anchor,
    screen_width: f32,
    screen_height: f32,
    content_width: f32,
    content_height: f32,
    padding: f32,
) -> (f32, f32) {
    let x = match anchor {
        Anchor::TopLeft | Anchor::CenterLeft | Anchor::BottomLeft => padding,
        Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter => {
            (screen_width - content_width) / 2.0
        }
        Anchor::TopRight | Anchor::CenterRight | Anchor::BottomRight => {
            screen_width - content_width - padding
        }
    };

    let y = match anchor {
        Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => padding,
        Anchor::CenterLeft | Anchor::Center | Anchor::CenterRight => {
            (screen_height - content_height) / 2.0
        }
        Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => {
            screen_height - content_height - padding
        }
    };

    (x, y)
}
