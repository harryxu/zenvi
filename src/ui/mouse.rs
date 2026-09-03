use super::{ZenviView, GRID_PADDING_LEFT, TOP_OFFSET};
use gpui::*;
use std::sync::Arc;

/// Converts GPUI modifier flags to Neovim modifier string (e.g. "CS" for Ctrl+Shift)
/// without any heap allocations.
#[inline]
pub fn mods_to_nvim(mods: &Modifiers) -> &'static str {
    match (mods.control, mods.shift, mods.alt, mods.platform) {
        (false, false, false, false) => "",
        (true, false, false, false) => "C",
        (false, true, false, false) => "S",
        (false, false, true, false) => "M",
        (false, false, false, true) => "D",
        (true, true, false, false) => "CS",
        (true, false, true, false) => "CM",
        (true, false, false, true) => "CD",
        (false, true, true, false) => "SM",
        (false, true, false, true) => "SD",
        (false, false, true, true) => "MD",
        (true, true, true, false) => "CSM",
        (true, true, false, true) => "CSD",
        (true, false, true, true) => "CMD",
        (false, true, true, true) => "SMD",
        (true, true, true, true) => "CSMD",
    }
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
        cx: &mut Context<Self>,
    ) {
        #[cfg(not(target_os = "macos"))]
        {
            self.is_menu_open = false;
            self.active_submenu = None;
        }
        window.focus(&self.focus_handle);
        self.trigger_interaction();
        let (col, row) = self.pos_to_grid(position);
        if button == "left" {
            self.is_mouse_down = true;
            self.last_mouse_pos = Some((col, row));
            self.is_drag_in_flight = false;
            self.pending_mouse_drag = None;
            self._drag_task = None;
            // When clicking the right border scrollbar, lock the drag column to this position.
            // This grants mouse capture so moving horizontally into the editor buffer won't abort the scrollbar drag.
            if col >= self.last_cols.saturating_sub(3) {
                self.scrollbar_drag_col = Some(col);
            } else {
                self.scrollbar_drag_col = None;
            }
            cx.notify(); // Re-render to dynamically attach on_mouse_move listener
        }
        let mods = mods_to_nvim(modifiers);
        self.session
            .send_mouse(button, "press", mods, 0, row, col);
    }

    /// Handles mouse button release events for any button.
    /// For left button, stops drag tracking.
    pub fn handle_mouse_up(
        &mut self,
        button: &str,
        position: Point<Pixels>,
        modifiers: &Modifiers,
        cx: &mut Context<Self>,
    ) {
        let (col, row) = self.pos_to_grid(position);
        let effective_col = if button == "left" {
            self.scrollbar_drag_col.take().unwrap_or(col)
        } else {
            col
        };

        if button == "left" {
            self.is_mouse_down = false;
            self.last_mouse_pos = None;
            self.pending_mouse_drag = None;
            self.is_drag_in_flight = false;
            self._drag_task = None;
            cx.notify(); // Re-render to detach on_mouse_move listener
        }
        let mods = mods_to_nvim(modifiers);
        self.session
            .send_mouse(button, "release", mods, 0, row, effective_col);
    }

    /// Handles mouse drag when left button is held down.
    /// Implements closed-loop in-flight backpressure and latest-coordinate coalescing:
    /// Never floods Neovim with more than 1 pending drag request at a time.
    pub fn handle_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if !self.is_mouse_down {
            return;
        }
        self.trigger_interaction();
        cx.notify();
        let (col, row) = self.pos_to_grid(event.position);
        let effective_col = self.scrollbar_drag_col.unwrap_or(col);

        if self.last_mouse_pos == Some((effective_col, row)) {
            return;
        }
        self.last_mouse_pos = Some((effective_col, row));

        if self.is_drag_in_flight {
            // Neovim is still processing previous drag: buffer the freshest coordinate
            self.pending_mouse_drag = Some((effective_col, row, event.modifiers.clone()));
        } else {
            // Dispatch immediately with backpressure tracking
            self.dispatch_mouse_drag(effective_col, row, &event.modifiers, cx);
        }
    }

    /// Dispatches a mouse drag request to Neovim and awaits its response before sending the next buffered drag.
    pub fn dispatch_mouse_drag(
        &mut self,
        col: usize,
        row: usize,
        modifiers: &Modifiers,
        cx: &mut Context<Self>,
    ) {
        self.is_drag_in_flight = true;
        self.pending_mouse_drag = None;
        let mods = mods_to_nvim(modifiers);
        let session = Arc::clone(&self.session);

        self._drag_task = Some(cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cx = cx.clone();
            async move {
                let _ = session
                    .request_mouse("left", "drag", mods, 0, row, col)
                    .await;

                let _ = cx.update(|cx| {
                    if let Some(entity) = this.upgrade() {
                        entity.update(cx, |this, cx| {
                            this.is_drag_in_flight = false;
                            this._drag_task = None;
                            if this.is_mouse_down {
                                if let Some((c, r, m)) = this.pending_mouse_drag.take() {
                                    this.dispatch_mouse_drag(c, r, &m, cx);
                                }
                            }
                        });
                    }
                });
            }
        }));
    }

    /// Handles scroll wheel events, converting pixel or line deltas
    /// into discrete Neovim scroll commands with calibrated distance.
    pub fn handle_scroll_wheel(&mut self, event: &ScrollWheelEvent, _cx: &mut Context<Self>) {
        self.trigger_interaction();
        let (col, row) = self.pos_to_grid(event.position);
        let mods = mods_to_nvim(&event.modifiers);

        let lh_f32: f32 = self.line_height.into();
        // Calibrated step: 1 wheel notch (~100-120px) or swipe produces 1-2 ticks (~3-6 lines in Neovim),
        let step = (lh_f32 * 2.4).max(45.0);

        let mut ticks = 0i32;
        match event.delta {
            ScrollDelta::Pixels(p) => {
                let dy: f32 = p.y.into();
                self.scroll_accum_y += dy;

                while self.scroll_accum_y >= step {
                    self.scroll_accum_y -= step;
                    ticks += 1;
                }
                while self.scroll_accum_y <= -step {
                    self.scroll_accum_y += step;
                    ticks -= 1;
                }
                self.scroll_accum_y = self.scroll_accum_y.clamp(-step * 2.0, step * 2.0);
            }
            ScrollDelta::Lines(l) => {
                let lines = l.y;
                ticks = lines.round() as i32;
            }
        }

        if ticks != 0 {
            let dir = if ticks > 0 { "up" } else { "down" };
            self.session.send_mouse("wheel", dir, mods, 0, row, col);
        }
    }
}
