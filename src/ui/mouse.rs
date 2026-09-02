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
        let x: f32 = (pos.x - px(self.current_shadow_size)).into();
        let y: f32 = (pos.y - px(self.current_shadow_size)).into();
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
        #[cfg(not(target_os = "macos"))]
        {
            self.is_menu_open = false;
            self.active_submenu = None;
        }
        window.focus(&self.focus_handle);
        let (col, row) = self.pos_to_grid(position);
        if button == "left" {
            self.is_mouse_down = true;
            self.last_mouse_pos = Some((col, row));
        }
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
            self.last_mouse_pos = None;
            self._drag_task = None;
            self.pending_mouse_drag = None;
        }
        let (col, row) = self.pos_to_grid(position);
        let mods = mods_to_nvim(modifiers);
        self.session
            .send_mouse(button, "release", &mods, 0, row, col);
    }

    /// Handles mouse drag when left button is held down.
    pub fn handle_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.is_mouse_down {
            let (col, row) = self.pos_to_grid(event.position);
            // Deduplicate: only send RPC drag event to Neovim when the grid cell changes
            if self.last_mouse_pos != Some((col, row)) {
                self.last_mouse_pos = Some((col, row));
                let mods = mods_to_nvim(&event.modifiers);

                let now = std::time::Instant::now();
                let elapsed = now.duration_since(self.last_mouse_drag_instant);
                // 8ms throttling (~125 drag updates/sec) prevents flooding Neovim with intermediate scrollbar jumps
                if elapsed >= std::time::Duration::from_millis(8) {
                    self.last_mouse_drag_instant = now;
                    self.pending_mouse_drag = None;
                    self.session
                        .send_mouse("left", "drag", &mods, 0, row, col);
                } else {
                    self.pending_mouse_drag = Some((mods, row, col));
                    let remaining = std::time::Duration::from_millis(8).saturating_sub(elapsed);
                    self._drag_task = Some(cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                        let cx = cx.clone();
                        async move {
                            tokio::time::sleep(remaining).await;
                            let _ = cx.update(|cx| {
                                if let Some(entity) = this.upgrade() {
                                    entity.update(cx, |this, _cx| {
                                        if this.is_mouse_down {
                                            if let Some((mods, r, c)) = this.pending_mouse_drag.take() {
                                                this.last_mouse_drag_instant = std::time::Instant::now();
                                                this.session.send_mouse("left", "drag", &mods, 0, r, c);
                                            }
                                        }
                                    });
                                }
                            });
                        }
                    }));
                }
            }
        }
    }

    /// Handles scroll wheel events, converting pixel or line deltas
    /// into discrete Neovim scroll commands with fractional accumulation.
    pub fn handle_scroll_wheel(&mut self, event: &ScrollWheelEvent) {
        let (col, row) = self.pos_to_grid(event.position);
        let mods = mods_to_nvim(&event.modifiers);
        let lh_f32: f32 = self.line_height.into();
        let step = (lh_f32 * 0.8).max(12.0);

        match event.delta {
            ScrollDelta::Pixels(p) => {
                let dy: f32 = p.y.into();
                self.scroll_accum_y += dy;

                let mut ticks = 0i32;
                while self.scroll_accum_y >= step {
                    self.scroll_accum_y -= step;
                    ticks += 1;
                }
                while self.scroll_accum_y <= -step {
                    self.scroll_accum_y += step;
                    ticks -= 1;
                }

                // Prevent unbounded accumulation on ultra-fast trackpad swipes
                self.scroll_accum_y = self.scroll_accum_y.clamp(-step * 2.0, step * 2.0);

                if ticks > 0 {
                    let count = (ticks as usize).min(5);
                    for _ in 0..count {
                        self.session
                            .send_mouse("wheel", "up", &mods, 0, row, col);
                    }
                } else if ticks < 0 {
                    let count = ((-ticks) as usize).min(5);
                    for _ in 0..count {
                        self.session
                            .send_mouse("wheel", "down", &mods, 0, row, col);
                    }
                }
            }
            ScrollDelta::Lines(l) => {
                let lines = l.y;
                let count = (lines.round().abs() as usize).min(5);
                if lines > 0.0 {
                    for _ in 0..count {
                        self.session
                            .send_mouse("wheel", "up", &mods, 0, row, col);
                    }
                } else if lines < 0.0 {
                    for _ in 0..count {
                        self.session
                            .send_mouse("wheel", "down", &mods, 0, row, col);
                    }
                }
            }
        }
    }
}
