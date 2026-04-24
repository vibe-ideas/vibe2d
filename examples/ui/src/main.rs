use vibe2d::prelude::*;

// ── Demo state ─────────────────────────────────────────────────
struct UiDemo {
    vw: f32,
    vh: f32,

    // Interactive state
    click_count: u32,
    progress: f32,
    progress_direction: f32,
    messages: Vec<String>,
}

impl Game for UiDemo {
    fn new(ctx: &mut Context) -> Self {
        Self {
            vw: ctx.virtual_width,
            vh: ctx.virtual_height,
            click_count: 0,
            progress: 0.0,
            progress_direction: 1.0,
            messages: vec![
                "Welcome to the UI demo!".into(),
                "Try clicking buttons and typing.".into(),
                "Scroll with wheel, Shift+wheel for horizontal.".into(),
            ],
        }
    }

    fn update(&mut self, _ctx: &mut Context, dt: f32, _input: &InputState) {
        // Animate the progress bar back and forth
        self.progress += dt * 0.3 * self.progress_direction;
        if self.progress >= 1.0 {
            self.progress = 1.0;
            self.progress_direction = -1.0;
        } else if self.progress <= 0.0 {
            self.progress = 0.0;
            self.progress_direction = 1.0;
        }
    }

    fn update_ui(&mut self, ctx: &mut Context, input: &InputState) {
        let white_tex = ctx
            .assets
            .texture_id("__vibe_ui_white")
            .unwrap_or(TextureId(0));
        let vw = self.vw;
        let vh = self.vh;

        // Take ui_state out so we can borrow ctx.assets independently
        let mut ui_state = std::mem::take(&mut ctx.ui_state);
        let mut ui = UiContext::new(&mut ui_state, input, white_tex, vw, vh);

        // ── Title (top center) ──────────────────────────────────
        ui.set_anchor(Anchor::TopCenter);
        ui.set_cursor(0.0, 8.0);
        if let Some(font) = ctx.assets.font("title") {
            ui.label(font, "Vibe2D UI Demo");
        }

        // ── Layout: two rows, each with sections side by side ──
        // Virtual resolution is 512×320.
        // Zero out anchor padding so set_cursor controls exact positions.
        ui.set_padding(0.0);

        let col1 = 12.0;
        let col2 = 180.0;
        let col3 = 350.0;
        let row1 = 38.0;
        let row2 = 168.0;

        // ── Row 1, Col 1: Labels ────────────────────────────────
        ui.set_anchor(Anchor::TopLeft);
        ui.set_cursor(col1, row1);
        ui.set_spacing(4.0);

        if let Some(font) = ctx.assets.font("body") {
            ui.label_colored(font, "Labels", UiColor::from_hex(0x55BBFF));
            ui.label(font, "Plain label");
            ui.label_colored(font, "Colored label", UiColor::from_hex(0xFF8855));
            ui.label_colored(font, "Semi-transparent", UiColor::WHITE.with_alpha(0.4));
        }

        // ── Row 1, Col 2: Progress Bar ──────────────────────────
        ui.set_anchor(Anchor::TopLeft);
        ui.set_cursor(col2, row1);
        ui.set_spacing(4.0);

        if let Some(font) = ctx.assets.font("body") {
            ui.label_colored(font, "Progress Bar", UiColor::from_hex(0x55BBFF));
            let pct = format!("{:.0}%", self.progress * 100.0);
            ui.label(font, &pct);
            ui.progress_bar(self.progress, 150.0, 12.0);
        }

        // ── Row 1, Col 3: Text Input ────────────────────────────
        ui.set_anchor(Anchor::TopLeft);
        ui.set_cursor(col3, row1);
        ui.set_spacing(4.0);

        if let Some(font) = ctx.assets.font("body") {
            ui.label_colored(font, "Text Input", UiColor::from_hex(0x55BBFF));
            let input_resp =
                ui.text_input_with_placeholder("chat_input", font, 155.0, "Type here...");
            if input_resp.submitted {
                let text = ui.text_input_value("chat_input");
                if !text.is_empty() {
                    self.messages.push(format!("> {}", text));
                    ui.text_input_clear("chat_input");
                }
            }
        }

        // ── Row 2, Col 1: Buttons ───────────────────────────────
        ui.set_anchor(Anchor::TopLeft);
        ui.set_cursor(col1, row2);
        ui.set_spacing(4.0);

        if let Some(font) = ctx.assets.font("body") {
            ui.label_colored(font, "Buttons", UiColor::from_hex(0x55BBFF));

            if ui.button_with_id("btn_click", font, "Click me!").clicked() {
                self.click_count += 1;
                self.messages
                    .push(format!("Button clicked {} time(s)", self.click_count));
            }

            let counter_text = format!("Clicks: {}", self.click_count);
            ui.label(font, &counter_text);

            let green_style = ButtonStyle {
                bg_color: UiColor::new(0.2, 0.5, 0.2, 0.9),
                hover_color: UiColor::new(0.3, 0.7, 0.3, 0.9),
                pressed_color: UiColor::new(0.1, 0.4, 0.1, 0.9),
                text_color: UiColor::WHITE,
                padding: 6.0,
            };
            if ui.button_styled(font, "Reset", green_style).clicked() {
                self.click_count = 0;
                self.messages.push("Counter reset!".into());
            }
        }

        // ── Row 2, Col 2: Panel ─────────────────────────────────
        ui.set_anchor(Anchor::TopLeft);
        ui.set_cursor(col2, row2);
        ui.set_spacing(4.0);

        if let Some(font) = ctx.assets.font("body") {
            ui.label_colored(font, "Panel", UiColor::from_hex(0x55BBFF));

            let panel_style = PanelStyle {
                bg_color: UiColor::new(0.15, 0.15, 0.25, 0.85),
                padding: 8.0,
            };
            ui.panel(panel_style, |ui| {
                ui.label(font, "Inside a panel");
                ui.label_colored(font, "Nested content", UiColor::from_hex(0xAAFF88));
                ui.progress_bar(0.65, 120.0, 8.0);
            });
        }

        // ── Row 2, Col 3: Scroll List ───────────────────────────
        ui.set_anchor(Anchor::TopLeft);
        ui.set_cursor(col3, row2);
        ui.set_spacing(4.0);

        if let Some(font) = ctx.assets.font("body") {
            ui.label_colored(font, "Scroll List", UiColor::from_hex(0x55BBFF));

            let messages = &self.messages;
            ui.scroll_list("msg_list", 155.0, 110.0, |ui| {
                for msg in messages {
                    ui.label(font, msg);
                }
            });
        }

        // ── Bottom bar: Layout direction demo ───────────────────
        ui.set_anchor(Anchor::BottomCenter);
        ui.set_cursor(0.0, -8.0);
        ui.set_layout(LayoutDirection::Horizontal);
        ui.set_spacing(8.0);

        if let Some(font) = ctx.assets.font("small") {
            ui.label_colored(font, "Horizontal layout:", UiColor::new(0.6, 0.6, 0.6, 1.0));
            ui.label(font, "A");
            ui.label(font, "B");
            ui.label(font, "C");
        }

        ui.finish();
        ctx.ui_state = ui_state;
    }

    fn draw(&self, _ctx: &Context, _screen: &mut Screen) {
        // All rendering is done via the UI system — nothing to draw here.
    }

    fn clear_color(&self) -> Color {
        Color::from_hex(0x1A1A2E)
    }
}

fn main() {
    vibe2d::run::<UiDemo>("game.yaml");
}
