pub mod font;
pub mod grid;

use crate::input::key_event_to_nvim;
use crate::nvim::process::{NvimEvent, NvimSession};
use crate::{Escape, OpenFile, OpenFolder, ReloadNvim};
use font::parse_guifont;
use gpui::*;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

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
    pub cwd: Option<PathBuf>,
    _event_task: Option<Task<()>>,
}

impl ZenviView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let (event_tx, event_rx) = mpsc::unbounded_channel::<NvimEvent>();
        let session = match NvimSession::spawn(event_tx, None) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to spawn Neovim process: {:?}", e);
                std::process::exit(1);
            }
        };

        // Initial attach with 100x35
        session.attach_ui(100, 35);
        session.send_command("set mouse=a");

        let font_family = "Menlo".to_string();
        let font_size = px(14.0);
        let line_height = px(20.0);

        let font_id = cx.text_system().resolve_font(&font(&font_family));
        let char_width: f32 = cx
            .text_system()
            .advance(font_id, font_size, '0')
            .or_else(|_| cx.text_system().advance(font_id, font_size, 'm'))
            .map(|s| s.width.into())
            .unwrap_or(14.0 * 0.6015);

        let event_task = Self::spawn_event_listener(event_rx, cx);

        Self {
            session,
            focus_handle,
            font_family,
            font_size,
            line_height,
            char_width,
            last_cols: 100,
            last_rows: 35,
            last_guifont: String::new(),
            last_linespace: 0,
            is_mouse_down: false,
            scroll_accum_y: 0.0,
            cwd: None,
            _event_task: Some(event_task),
        }
    }

    fn spawn_event_listener(
        mut event_rx: mpsc::UnboundedReceiver<NvimEvent>,
        cx: &mut Context<Self>,
    ) -> Task<()> {
        cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                while let Some(event) = event_rx.recv().await {
                    match event {
                        NvimEvent::Redraw => {
                            let _ = this.update(&mut cx, |_this, cx| {
                                cx.notify();
                            });
                        }
                        NvimEvent::Exit => {
                            let _ = cx.update(|cx| {
                                cx.quit();
                            });
                        }
                    }
                }
            }
        })
    }

    pub fn reload_nvim(&mut self, cx: &mut Context<Self>) {
        log::info!("Reloading Neovim session...");
        self.session.kill();
        self._event_task = None;

        let (event_tx, event_rx) = mpsc::unbounded_channel::<NvimEvent>();
        match NvimSession::spawn(event_tx, self.cwd.clone()) {
            Ok(new_session) => {
                new_session.attach_ui(self.last_cols, self.last_rows);
                new_session.send_command("set mouse=a");

                self.session = new_session;
                self.last_guifont = String::new();
                self.last_linespace = 0;
                self._event_task = Some(Self::spawn_event_listener(event_rx, cx));
                cx.notify();
            }
            Err(e) => {
                eprintln!("Failed to reload Neovim: {:?}", e);
            }
        }
    }

    pub fn open_file(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open File".into()),
        });
        let session = Arc::clone(&self.session);
        cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                if let Ok(Ok(Some(paths))) = receiver.await {
                    for path in paths {
                        if let Some(parent) = path.parent() {
                            let _ = this.update(&mut cx, |this, _cx| {
                                this.cwd = Some(parent.to_path_buf());
                            });
                            session.send_command(&format!("cd {}", parent.display()));
                        }
                        session.send_command(&format!("edit {}", path.display()));
                    }
                }
            }
        })
        .detach();
    }

    pub fn open_folder(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open Folder".into()),
        });
        let session = Arc::clone(&self.session);
        cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                if let Ok(Ok(Some(paths))) = receiver.await {
                    for path in paths {
                        let _ = this.update(&mut cx, |this, _cx| {
                            this.cwd = Some(path.clone());
                        });
                        session.send_command(&format!("cd {}", path.display()));
                        session.send_command(&format!("edit {}", path.display()));
                    }
                }
            }
        })
        .detach();
    }

    fn update_font(&mut self, guifont: &str, linespace: i64, cx: &App) {
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

        // Measure actual monospace advance width using GPUI text system
        let font_id = cx.text_system().resolve_font(&font(&self.font_family));
        let advance: f32 = cx
            .text_system()
            .advance(font_id, self.font_size, '0')
            .or_else(|_| cx.text_system().advance(font_id, self.font_size, 'm'))
            .map(|s| s.width.into())
            .unwrap_or(size * 0.6015);
        self.char_width = advance;

        // Line height calculation: base 1.428 multiplier + linespace pixels
        let base_lh = (size * 1.428).round();
        let final_lh = (base_lh + linespace as f32).max(8.0);
        self.line_height = px(final_lh);
    }

    fn pos_to_grid(&self, pos: Point<Pixels>) -> (usize, usize) {
        let x: f32 = pos.x.into();
        let y: f32 = pos.y.into();

        // Titlebar height: 32.0
        let top_offset = 32.0;
        let lh: f32 = self.line_height.into();

        let col = (x / self.char_width).floor().max(0.0) as usize;
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
            self.update_font(&new_guifont, new_linespace, cx);
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

        let viewport = window.viewport_size();
        let window_w: f32 = viewport.width.into();
        let window_h: f32 = viewport.height.into();

        // Titlebar height: 32px, Bottom status bar height: 24px
        let top_offset = 32.0;
        let bottom_offset = 24.0;
        let lh: f32 = self.line_height.into();

        let cols = (window_w / self.char_width).floor().max(20.0) as usize;
        let rows = ((window_h - top_offset - bottom_offset) / lh).floor().max(5.0) as usize;

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
            self.char_width,
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
            .on_action(cx.listener(|this, _: &ReloadNvim, _window, cx| {
                this.reload_nvim(cx);
            }))
            .on_action(cx.listener(|this, _: &OpenFile, _window, cx| {
                this.open_file(cx);
            }))
            .on_action(cx.listener(|this, _: &OpenFolder, _window, cx| {
                this.open_folder(cx);
            }))
            .on_action(cx.listener(|this, _: &Escape, _window, _cx| {
                this.session.send_input("<Esc>");
            }))
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
                        this.cwd = Some(path.clone());
                        this.session.send_command(&format!("cd {}", path.display()));
                        this.session.send_command(&format!("edit {}", path.display()));
                    } else {
                        if let Some(parent) = path.parent() {
                            this.cwd = Some(parent.to_path_buf());
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
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xdcdcdc))
                                    .child(title),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(0x777777))
                                    .child("⚡️ GPUI"),
                            ),
                    ),
            )
            .child(
                // Editor Main Grid Area
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
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
