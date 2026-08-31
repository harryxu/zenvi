use super::{ZenviView, GRID_PADDING_LEFT, TOP_OFFSET};
use gpui::*;

/// Converts GPUI modifier flags to Neovim modifier string (e.g. "CS" for Ctrl+Shift).
pub fn mods_to_nvim(mods: &Modifiers) -> String {
    let mut s = String::new();
    if mods.control {
        s.push('C');
    }
    if mods.shift {
        s.push('S');
    }
    if mods.alt {
        s.push('A');
    }
    if mods.platform {
        s.push('D');
    }
    s
}

impl ZenviView {
    /// Converts a pixel position to a Neovim grid `(col, row)` coordinate.
    pub fn pos_to_grid(&self, pos: Point<Pixels>) -> (usize, usize) {
        let x: f32 = pos.x.into();
        let y: f32 = pos.y.into();
        let lh: f32 = self.line_height.into();

        let col = ((x - GRID_PADDING_LEFT) / self.char_width)
            .floor()
            .max(0.0) as usize;
        let row = ((y - TOP_OFFSET) / lh).floor().max(0.0) as usize;

        (
            col.min(self.last_cols.saturating_sub(1)),
            row.min(self.last_rows.saturating_sub(1)),
        )
    }

    /// Handles mouse button press events for any button (left, right, middle).
    /// Focuses the window and, for left button, begins drag tracking.
    pub fn handle_mouse_down(
        &mut self,
        button: &str,
        position: Point<Pixels>,
        modifiers: &Modifiers,
        window: &mut Window,
    ) {
        window.focus(&self.focus_handle);
        if button == "left" {
            self.is_mouse_down = true;
        }
        let (col, row) = self.pos_to_grid(position);
        let mods = mods_to_nvim(modifiers);
        self.session
            .send_mouse(button, "press", &mods, 0, row, col);
    }

    /// Handles mouse button release events for any button.
    /// For left button, stops drag tracking.
    pub fn handle_mouse_up(
        &mut self,
        button: &str,
        position: Point<Pixels>,
        modifiers: &Modifiers,
    ) {
        if button == "left" {
            self.is_mouse_down = false;
        }
        let (col, row) = self.pos_to_grid(position);
        let mods = mods_to_nvim(modifiers);
        self.session
            .send_mouse(button, "release", &mods, 0, row, col);
    }

    /// Handles mouse drag when left button is held down.
    pub fn handle_mouse_move(&mut self, event: &MouseMoveEvent) {
        if self.is_mouse_down {
            let (col, row) = self.pos_to_grid(event.position);
            let mods = mods_to_nvim(&event.modifiers);
            self.session
                .send_mouse("left", "drag", &mods, 0, row, col);
        }
    }

    /// Handles scroll wheel events, converting pixel or line deltas
    /// into discrete Neovim scroll commands with fractional accumulation.
    pub fn handle_scroll_wheel(&mut self, event: &ScrollWheelEvent) {
        let (col, row) = self.pos_to_grid(event.position);
        let mods = mods_to_nvim(&event.modifiers);

        match event.delta {
            ScrollDelta::Pixels(p) => {
                let dy: f32 = p.y.into();
                self.scroll_accum_y += dy;
                let step = 15.0;
                while self.scroll_accum_y >= step {
                    self.scroll_accum_y -= step;
                    self.session
                        .send_mouse("wheel", "up", &mods, 0, row, col);
                }
                while self.scroll_accum_y <= -step {
                    self.scroll_accum_y += step;
                    self.session
                        .send_mouse("wheel", "down", &mods, 0, row, col);
                }
            }
            ScrollDelta::Lines(l) => {
                let lines = l.y;
                if lines > 0.0 {
                    for _ in 0..(lines.round().abs() as usize) {
                        self.session
                            .send_mouse("wheel", "up", &mods, 0, row, col);
                    }
                } else if lines < 0.0 {
                    for _ in 0..(lines.round().abs() as usize) {
                        self.session
                            .send_mouse("wheel", "down", &mods, 0, row, col);
                    }
                }
            }
        }
    }
}
