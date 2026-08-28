pub mod grid;

use crate::input::key_event_to_nvim;
use crate::nvim::process::NvimSession;
use gpui::*;
use std::sync::Arc;

pub struct ZenviView {
    pub session: Arc<NvimSession>,
    pub focus_handle: FocusHandle,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub char_width: f32,
    pub last_cols: usize,
    pub last_rows: usize,
}

impl ZenviView {
    pub fn new(session: Arc<NvimSession>, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Initial attach with 100x35
        session.attach_ui(100, 35);

        Self {
            session,
            focus_handle,
            font_size: px(14.0),
            line_height: px(20.0),
            char_width: 8.42,
            last_cols: 100,
            last_rows: 35,
        }
    }
}

impl Render for ZenviView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.session.state.read();
        let default_bg = state.default_bg;
        let current_mode = state.current_mode.to_uppercase();
        let title = if state.title.is_empty() {
            "Zenvi".to_string()
        } else {
            state.title.clone()
        };

        let grid = state
            .grids
            .get(&state.active_grid)
            .cloned()
            .unwrap_or_else(|| crate::nvim::state::Grid::new(1, 80, 24));

        let bounds = window.bounds();
        let window_w: f32 = bounds.size.width.into();
        let window_h: f32 = bounds.size.height.into();

        // Titlebar height: 32px, Bottom status bar height: 24px, Padding: 8px
        let top_offset = 32.0;
        let bottom_offset = 24.0;
        let padding_y = 8.0;

        let cols = ((window_w - 16.0) / self.char_width).floor().max(20.0) as usize;
        let rows = ((window_h - top_offset - bottom_offset - padding_y) / 20.0).floor().max(5.0) as usize;

        if cols != self.last_cols || rows != self.last_rows {
            self.last_cols = cols;
            self.last_rows = rows;
            self.session.try_resize(cols, rows);
        }

        let grid_element = grid::render_grid(&state, &grid, self.font_size, self.line_height);

        // Status bar colors based on mode
        let mode_bg = match current_mode.as_str() {
            "INSERT" => rgb(0x2e7d32),
            "VISUAL" | "V-LINE" | "V-BLOCK" => rgb(0x6a1b9a),
            "REPLACE" => rgb(0xc62828),
            "COMMAND" => rgb(0xef6c00),
            _ => rgb(0x37474f),
        };

        div()
            .id("zenvi-root")
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(default_bg))
            .track_focus(&self.focus_handle)
            .key_context("zenvi")
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, _cx| {
                if let Some(nvim_key) = key_event_to_nvim(event) {
                    this.session.send_input(&nvim_key);
                }
            }))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _window, _cx| {
                for path in paths.paths() {
                    if path.is_dir() {
                        this.session.send_command(&format!("cd {}", path.display()));
                        this.session.send_command(&format!("edit {}", path.display()));
                    } else {
                        if let Some(parent) = path.parent() {
                            this.session.send_command(&format!("cd {}", parent.display()));
                        }
                        this.session.send_command(&format!("edit {}", path.display()));
                    }
                }
            }))
            .child(
                // Top Custom Titlebar: Leaves room (pl 78px) for macOS traffic light buttons
                div()
                    .h(px(32.0))
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .pl(px(78.0))
                    .pr(px(12.0))
                    .bg(rgb(0x161616))
                    .border_b_1()
                    .border_color(rgb(0x222222))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(0xaaaaaa))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(0x666666))
                            .child("⚡️ GPUI"),
                    ),
            )
            .child(
                // Editor Main Grid Area
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p(px(4.0))
                    .child(grid_element),
            )
            .child(
                // Bottom native status bar
                div()
                    .h(px(24.0))
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px(px(8.0))
                    .bg(rgb(0x181818))
                    .border_t_1()
                    .border_color(rgb(0x2a2a2a))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .px(px(6.0))
                                    .py(px(1.0))
                                    .rounded(px(3.0))
                                    .bg(mode_bg)
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xffffff))
                                    .child(current_mode),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(0x888888))
                                    .child(format!("{}:{}", grid.cursor_row + 1, grid.cursor_col + 1)),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(0x888888))
                            .child("Zenvi (GPUI + Neovim)"),
                    ),
            )
    }
}
