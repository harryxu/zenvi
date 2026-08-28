pub mod font;
pub mod grid;

use crate::input::key_event_to_nvim;
use crate::nvim::process::NvimSession;
use font::parse_guifont;
use gpui::*;
use std::sync::Arc;

fn mods_to_nvim(mods: &Modifiers) -> String {
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

pub struct ZenviView {
    pub session: Arc<NvimSession>,
    pub focus_handle: FocusHandle,
    pub font_family: String,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub char_width: f32,
    pub last_cols: usize,
    pub last_rows: usize,
    pub last_guifont: String,
    pub last_linespace: i64,
    pub is_mouse_down: bool,
    pub scroll_accum_y: f32,
}

impl ZenviView {
    pub fn new(session: Arc<NvimSession>, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Initial attach with 100x35
        session.attach_ui(100, 35);
        session.send_command("set mouse=a");

        Self {
            session,
            focus_handle,
            font_family: "Menlo".to_string(),
            font_size: px(14.0),
            line_height: px(20.0),
            char_width: 8.42,
            last_cols: 100,
            last_rows: 35,
            last_guifont: String::new(),
            last_linespace: 0,
            is_mouse_down: false,
            scroll_accum_y: 0.0,
        }
    }

    fn update_font(&mut self, guifont: &str, linespace: i64) {
        let parsed = parse_guifont(guifont);

        if let Some(family) = parsed.family {
            self.font_family = family;
        } else if guifont.is_empty() {
            self.font_family = "Menlo".to_string();
        }

        let size: f32 = if let Some(s) = parsed.size {
            s
        } else if guifont.is_empty() {
            14.0
        } else {
            self.font_size.into()
        };

        self.font_size = px(size);
        self.char_width = size * 0.6015;

        // Line height calculation: base 1.428 multiplier + linespace pixels
        let base_lh = (size * 1.428).round();
        let final_lh = (base_lh + linespace as f32).max(8.0);
        self.line_height = px(final_lh);
    }

    fn pos_to_grid(&self, pos: Point<Pixels>) -> (usize, usize) {
        let x: f32 = pos.x.into();
        let y: f32 = pos.y.into();

        // Titlebar height: 32.0, Grid padding: 4.0
        let top_offset = 36.0;
        let left_offset = 4.0;
        let lh: f32 = self.line_height.into();

        let col = ((x - left_offset) / self.char_width).floor().max(0.0) as usize;
        let row = ((y - top_offset) / lh).floor().max(0.0) as usize;

        (
            col.min(self.last_cols.saturating_sub(1)),
            row.min(self.last_rows.saturating_sub(1)),
        )
    }
}

impl Render for ZenviView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (guifont_changed, new_guifont, new_linespace) = {
            let state = self.session.state.read();
            if state.guifont != self.last_guifont || state.linespace != self.last_linespace {
                (true, state.guifont.clone(), state.linespace)
            } else {
                (false, String::new(), 0)
            }
        };

        if guifont_changed {
            self.last_guifont = new_guifont.clone();
            self.last_linespace = new_linespace;
            self.update_font(&new_guifont, new_linespace);
        }

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
        let lh: f32 = self.line_height.into();

        let cols = ((window_w - 16.0) / self.char_width).floor().max(20.0) as usize;
        let rows = ((window_h - top_offset - bottom_offset - padding_y) / lh).floor().max(5.0) as usize;

        if cols != self.last_cols || rows != self.last_rows {
            self.last_cols = cols;
            self.last_rows = rows;
            self.session.try_resize(cols, rows);
        }

        let grid_element = grid::render_grid(
            &state,
            &grid,
            &self.font_family,
            self.font_size,
            self.line_height,
        );

        // Status bar colors based on mode
        let mode_bg = match current_mode.as_str() {
            "INSERT" => rgb(0x2e7d32),
            "VISUAL" | "V-LINE" | "V-BLOCK" => rgb(0x6a1b9a),
            "REPLACE" => rgb(0xc62828),
            "COMMAND" => rgb(0xef6c00),
            _ => rgb(0x37474f),
        };

        let focus_handle = self.focus_handle.clone();
        let entity = cx.entity().clone();

        div()
            .id("zenvi-root")
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(default_bg))
            .track_focus(&self.focus_handle)
            .key_context("zenvi")
            .child(
                canvas(
                    |_bounds, _window, _cx| {},
                    move |bounds, _, window, cx| {
                        window.handle_input(
                            &focus_handle,
                            ElementInputHandler::new(bounds, entity),
                            cx,
                        );
                    },
                )
                .size_0(),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, _cx| {
                if let Some(nvim_key) = key_event_to_nvim(event) {
                    this.session.send_input(&nvim_key);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, _cx| {
                    window.focus(&this.focus_handle);
                    this.is_mouse_down = true;
                    let (col, row) = this.pos_to_grid(event.position);
                    let mods = mods_to_nvim(&event.modifiers);
                    this.session.send_mouse("left", "press", &mods, 0, row, col);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &MouseDownEvent, window, _cx| {
                    window.focus(&this.focus_handle);
                    let (col, row) = this.pos_to_grid(event.position);
                    let mods = mods_to_nvim(&event.modifiers);
                    this.session.send_mouse("right", "press", &mods, 0, row, col);
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|this, event: &MouseDownEvent, window, _cx| {
                    window.focus(&this.focus_handle);
                    let (col, row) = this.pos_to_grid(event.position);
                    let mods = mods_to_nvim(&event.modifiers);
                    this.session.send_mouse("middle", "press", &mods, 0, row, col);
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _window, _cx| {
                    this.is_mouse_down = false;
                    let (col, row) = this.pos_to_grid(event.position);
                    let mods = mods_to_nvim(&event.modifiers);
                    this.session.send_mouse("left", "release", &mods, 0, row, col);
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, event: &MouseUpEvent, _window, _cx| {
                    let (col, row) = this.pos_to_grid(event.position);
                    let mods = mods_to_nvim(&event.modifiers);
                    this.session.send_mouse("right", "release", &mods, 0, row, col);
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, event: &MouseUpEvent, _window, _cx| {
                    let (col, row) = this.pos_to_grid(event.position);
                    let mods = mods_to_nvim(&event.modifiers);
                    this.session.send_mouse("middle", "release", &mods, 0, row, col);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, _cx| {
                if this.is_mouse_down {
                    let (col, row) = this.pos_to_grid(event.position);
                    let mods = mods_to_nvim(&event.modifiers);
                    this.session.send_mouse("left", "drag", &mods, 0, row, col);
                }
            }))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _window, _cx| {
                let (col, row) = this.pos_to_grid(event.position);
                let mods = mods_to_nvim(&event.modifiers);

                match event.delta {
                    ScrollDelta::Pixels(p) => {
                        let dy: f32 = p.y.into();
                        this.scroll_accum_y += dy;
                        let step = 15.0;
                        while this.scroll_accum_y >= step {
                            this.scroll_accum_y -= step;
                            this.session.send_mouse("wheel", "up", &mods, 0, row, col);
                        }
                        while this.scroll_accum_y <= -step {
                            this.scroll_accum_y += step;
                            this.session.send_mouse("wheel", "down", &mods, 0, row, col);
                        }
                    }
                    ScrollDelta::Lines(l) => {
                        let lines = l.y;
                        if lines > 0.0 {
                            for _ in 0..(lines.round().abs() as usize) {
                                this.session.send_mouse("wheel", "up", &mods, 0, row, col);
                            }
                        } else if lines < 0.0 {
                            for _ in 0..(lines.round().abs() as usize) {
                                this.session.send_mouse("wheel", "down", &mods, 0, row, col);
                            }
                        }
                    }
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

impl EntityInputHandler for ZenviView {
    fn text_for_range(
        &mut self,
        _range_utf16: std::ops::Range<usize>,
        _actual_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        _range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        if !new_text.is_empty() && (!new_text.is_ascii() || new_text.len() > 1) {
            self.session.send_input(new_text);
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range_utf16: Option<std::ops::Range<usize>>,
        _new_text: &str,
        _new_selected_range_utf16: Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: std::ops::Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(element_bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(0)
    }
}
