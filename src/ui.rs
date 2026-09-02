pub mod commands;
pub mod components;
pub mod font;
pub mod ime;
pub mod mouse;

use crate::input::key_event_to_nvim;
use crate::nvim::process::{NvimEvent, NvimSession};
use crate::{
    About, CloseBuffer, Copy, Cut, Escape, InstallCli, OpenFile, OpenFolder, Paste, Redo, ReloadNvim,
    SelectAll, Undo,
};
use font::resolve_default_font_family;
use gpui::prelude::*;
use gpui::*;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub const TITLEBAR_HEIGHT: f32 = 36.0;
pub const GRID_PADDING_TOP: f32 = 6.0;
pub const GRID_PADDING_LEFT: f32 = 4.0;
pub const TOP_OFFSET: f32 = TITLEBAR_HEIGHT + GRID_PADDING_TOP;

pub struct ZenviView {
    pub session: Arc<NvimSession>,
    pub focus_handle: FocusHandle,
    pub window_handle: AnyWindowHandle,
    pub font_family: String,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub char_width: f32,
    pub last_cols: usize,
    pub last_rows: usize,
    pub last_guifont: String,
    pub last_linespace: i64,
    pub is_mouse_down: bool,
    /// Last sent mouse grid coordinate, used to deduplicate drag events and prevent RPC queue congestion
    pub last_mouse_pos: Option<(usize, usize)>,
    pub scroll_accum_y: f32,
    pub cwd: Option<PathBuf>,
    pub marked_text: Option<String>,
    /// Whether the window is running in client-side decorations (borderless) mode.
    pub borderless: bool,
    /// The currently active window shadow inset (in pixels).
    pub current_shadow_size: f32,
    pub is_menu_open: bool,
    #[allow(dead_code)]
    pub active_submenu: Option<usize>,
    pub last_window_title: String,
    pub last_applied_shadow_size: f32,
    pub last_resize_instant: std::time::Instant,
    pub pending_resize: Option<(usize, usize)>,
    pub(crate) _resize_task: Option<Task<()>>,
    pub(crate) _drag_task: Option<Task<()>>,
    pub(crate) _event_task: Option<Task<()>>,
    pub(crate) _render_pump_task: Option<Task<()>>,
    pub last_interaction_instant: Option<std::time::Instant>,
    pub last_drag_instant: std::time::Instant,
    pub pending_mouse_drag: Option<(usize, usize, Modifiers)>,
    pub scrollbar_drag_col: Option<usize>,
    /// Persistent render cache for incremental grid rendering (dirty-row tracking).
    pub grid_cache: components::grid::GridRenderCache,
}

impl ZenviView {
    #[allow(dead_code)]
    pub fn new(window_handle: AnyWindowHandle, cx: &mut Context<Self>) -> Self {
        Self::with_cwd_and_targets(window_handle, None, Vec::new(), false, cx)
    }

    #[allow(dead_code)]
    pub fn with_cwd(
        window_handle: AnyWindowHandle,
        cwd: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::with_cwd_and_targets(window_handle, cwd, Vec::new(), false, cx)
    }

    pub fn with_cwd_and_targets(
        window_handle: AnyWindowHandle,
        cwd: Option<PathBuf>,
        targets: Vec<PathBuf>,
        borderless: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();

        let (event_tx, event_rx) = mpsc::unbounded_channel::<NvimEvent>();
        let session = match NvimSession::spawn(event_tx, cwd.clone(), targets.clone()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to spawn Neovim process: {:?}", e);
                std::process::exit(1);
            }
        };

        let font_family = resolve_default_font_family(cx);
        let font_size = px(14.0);
        let line_height = px((14.0_f32 * 1.2_f32).round());

        let font_id = cx.text_system().resolve_font(&font(&font_family));
        let char_width: f32 = cx
            .text_system()
            .advance(font_id, font_size, '0')
            .or_else(|_| cx.text_system().advance(font_id, font_size, 'm'))
            .map(|s| s.width.into())
            .unwrap_or(14.0 * 0.6015);

        // Initial attach with 100x35
        session.attach_ui(100, 35);
        session.send_command("set mouse=a");
        session.send_command("set title");
        session.send_command(&format!(
            r#"lua (function()
                if not vim.o.guifont or vim.o.guifont == "" then
                    vim.o.guifont = "{font_family}:h14"
                else
                    vim.o.guifont = vim.o.guifont
                end
                if vim.o.linespace and vim.o.linespace ~= 0 then
                    vim.o.linespace = vim.o.linespace
                end
            end)()"#
        ));

        if let Some(ref dir) = cwd {
            session.send_command(&format!("cd {}", dir.display()));
        }

        let event_task = Self::spawn_event_listener(event_rx, window_handle, cx);

        Self {
            session,
            focus_handle,
            window_handle,
            font_family,
            font_size,
            line_height,
            char_width,
            last_cols: 100,
            last_rows: 35,
            last_guifont: String::new(),
            last_linespace: 0,
            is_mouse_down: false,
            last_mouse_pos: None,
            scroll_accum_y: 0.0,
            cwd,
            marked_text: None,
            borderless,
            current_shadow_size: 0.0,
            is_menu_open: false,
            active_submenu: None,
            last_window_title: String::new(),
            last_applied_shadow_size: -1.0,
            last_resize_instant: std::time::Instant::now(),
            pending_resize: None,
            _resize_task: None,
            _drag_task: None,
            _event_task: Some(event_task),
            _render_pump_task: None,
            last_interaction_instant: None,
            last_drag_instant: std::time::Instant::now(),
            pending_mouse_drag: None,
            scrollbar_drag_col: None,
            grid_cache: components::grid::GridRenderCache::new(),
        }
    }

    pub(crate) fn spawn_event_listener(
        mut event_rx: mpsc::UnboundedReceiver<NvimEvent>,
        window_handle: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) -> Task<()> {
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                while let Some(event) = event_rx.recv().await {
                    let mut should_exit = false;
                    match event {
                        NvimEvent::Redraw => {
                            let Some(entity) = this.upgrade() else {
                                break;
                            };
                            if entity
                                .update(&mut cx, |this, cx| {
                                    this.trigger_interaction(cx);
                                    cx.notify();
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        NvimEvent::Exit => {
                            should_exit = true;
                        }
                    }

                    if should_exit {
                        let _ = cx.update(|cx| {
                            if cx.windows().len() <= 1 {
                                cx.quit();
                            } else if cx.windows().contains(&window_handle) {
                                let _ = window_handle.update(cx, |_, window, _cx| {
                                    window.remove_window();
                                });
                            }
                        });
                        break;
                    }
                }
            }
        })
    }

    /// Triggers the active 60 FPS swapchain presentation loop during user interaction
    /// (aligning with Neovide's active presentation model).
    /// Keeps pumping 60 FPS frames while user interaction or Neovim redraws occur,
    /// then silently stops after 300ms of inactivity to guarantee 0 FPS idle.
    pub(crate) fn trigger_interaction(&mut self, cx: &mut Context<Self>) {
        self.last_interaction_instant = Some(std::time::Instant::now());
        if self._render_pump_task.is_none() {
            self._render_pump_task = Some(cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
                let cx = cx.clone();
                async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_millis(16));
                    interval.tick().await; // Skip initial tick
                    loop {
                        interval.tick().await;
                        let active = cx
                            .update(|cx| {
                                let Some(entity) = this.upgrade() else {
                                    return false;
                                };
                                entity.update(cx, |this, cx| {
                                    if let Some(t) = this.last_interaction_instant {
                                        if t.elapsed() < std::time::Duration::from_millis(300) {
                                            cx.notify();
                                            return true;
                                        }
                                    }
                                    this.last_interaction_instant = None;
                                    this._render_pump_task = None;
                                    false
                                })
                            })
                            .unwrap_or(false);

                        if !active {
                            break;
                        }
                    }
                }
            }));
        }
    }

    /// Binds all GPUI action handlers to the root element.
    fn bind_actions(root: Stateful<Div>, cx: &mut Context<Self>) -> Stateful<Div> {
        root.on_action(cx.listener(|this, _: &About, _window, cx| {
            this.show_about(cx);
        }))
        .on_action(cx.listener(|this, _: &ReloadNvim, _window, cx| {
            this.reload_nvim(cx);
        }))
        .on_action(cx.listener(|this, _: &InstallCli, _window, cx| {
            this.install_cli(cx);
        }))
        .on_action(cx.listener(|this, _: &OpenFile, _window, cx| {
            this.open_file(cx);
        }))
        .on_action(cx.listener(|this, _: &OpenFolder, _window, cx| {
            this.open_folder(cx);
        }))
        .on_action(cx.listener(|this, _: &CloseBuffer, _window, cx| {
            this.close_buffer(cx);
        }))
        .on_action(cx.listener(|this, _: &Paste, _window, cx| {
            this.paste(cx);
        }))
        .on_action(cx.listener(|this, _: &Copy, _window, cx| {
            this.copy(cx);
        }))
        .on_action(cx.listener(|this, _: &Cut, _window, cx| {
            this.cut(cx);
        }))
        .on_action(cx.listener(|this, _: &SelectAll, _window, cx| {
            this.select_all(cx);
        }))
        .on_action(cx.listener(|this, _: &Undo, _window, cx| {
            this.undo(cx);
        }))
        .on_action(cx.listener(|this, _: &Redo, _window, cx| {
            this.redo(cx);
        }))
        .on_action(cx.listener(|this, _: &Escape, _window, _cx| {
            this.session.send_input("<Esc>");
        }))
    }

    /// Binds all mouse and scroll event handlers to the root element.
    fn bind_mouse_handlers(&self, root: Stateful<Div>, cx: &mut Context<Self>) -> Stateful<Div> {
        let root = root
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.handle_mouse_down("left", event.position, &event.modifiers, window, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.handle_mouse_down("right", event.position, &event.modifiers, window, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.handle_mouse_down("middle", event.position, &event.modifiers, window, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _window, cx| {
                    this.handle_mouse_up("left", event.position, &event.modifiers, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, event: &MouseUpEvent, _window, cx| {
                    this.handle_mouse_up("right", event.position, &event.modifiers, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, event: &MouseUpEvent, _window, cx| {
                    this.handle_mouse_up("middle", event.position, &event.modifiers, cx);
                }),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _window, cx| {
                this.handle_scroll_wheel(event, cx);
            }));

        // Dynamically bind on_mouse_move ONLY when left button is held down (dragging).
        // During normal cursor hovering, omits the listener entirely to guarantee 0 FPS idle rendering.
        if self.is_mouse_down {
            root.on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                this.handle_mouse_move(event, cx);
            }))
        } else {
            root
        }
    }
}

impl Render for ZenviView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Read Neovim state once for the entire render pass
        let session = Arc::clone(&self.session);
        let state = session.state.read();
        self.sync_font_from_state(&state.guifont, state.linespace, cx);

        let default_bg = state.default_bg;
        let style = components::style::derive_titlebar_style(state.default_bg, state.default_fg);

        let is_maximized = window.is_maximized();
        let shadow_size = if self.borderless && !is_maximized {
            px(8.0)
        } else {
            px(0.0)
        };
        let shadow_f32: f32 = shadow_size.into();
        self.current_shadow_size = shadow_f32;
        if (self.last_applied_shadow_size - shadow_f32).abs() > 0.001 {
            self.last_applied_shadow_size = shadow_f32;
            window.set_client_inset(shadow_size);
        }

        let display_title = components::titlebar::format_title(&state.title);
        if self.last_window_title != display_title {
            self.last_window_title = display_title.clone();
            window.set_window_title(&display_title);
        }

        let default_grid = crate::nvim::state::Grid::new(1, 80, 24);
        let grid = state
            .grids
            .get(&1)
            .or_else(|| state.grids.get(&state.active_grid))
            .unwrap_or(&default_grid);

        // Calculate grid dimensions and notify Neovim of resize with 25ms throttling
        let viewport = window.viewport_size();
        let window_w: f32 = viewport.width.into();
        let window_h: f32 = viewport.height.into();
        let content_w = (window_w - shadow_f32 * 2.0).max(100.0);
        let content_h = (window_h - shadow_f32 * 2.0).max(100.0);
        let lh: f32 = self.line_height.into();

        let horizontal_padding = GRID_PADDING_LEFT * 2.0 + 4.0;
        let cols = ((content_w - horizontal_padding) / self.char_width)
            .floor()
            .max(20.0) as usize;
        let rows = ((content_h - TOP_OFFSET) / lh).floor().max(5.0) as usize;

        if cols != self.last_cols || rows != self.last_rows {
            self.last_cols = cols;
            self.last_rows = rows;

            let now = std::time::Instant::now();
            let elapsed = now.duration_since(self.last_resize_instant);
            if elapsed >= std::time::Duration::from_millis(25) {
                self.last_resize_instant = now;
                self.pending_resize = None;
                self.session.try_resize(cols, rows);
            } else {
                self.pending_resize = Some((cols, rows));
                let remaining = std::time::Duration::from_millis(25).saturating_sub(elapsed);
                self._resize_task = Some(cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let cx = cx.clone();
                    async move {
                        tokio::time::sleep(remaining).await;
                        let _ = cx.update(|cx| {
                            if let Some(entity) = this.upgrade() {
                                entity.update(cx, |this, _cx| {
                                    if let Some((c, r)) = this.pending_resize.take() {
                                        this.last_resize_instant = std::time::Instant::now();
                                        this.session.try_resize(c, r);
                                    }
                                });
                            }
                        });
                    }
                }));
            }
        }

        let grid_element = components::grid::render_grid(
            &state,
            grid,
            &self.font_family,
            self.font_size,
            self.line_height,
            self.char_width,
            &mut self.grid_cache,
        );

        let focus_handle = self.focus_handle.clone();
        let entity = cx.entity().clone();

        let titlebar_element = components::titlebar::render_titlebar(
            &display_title,
            &style,
            default_bg,
            self.is_menu_open,
            self.borderless,
            window,
            cx,
        );

        // Build inner window element tree
        let inner = div()
            .id("zenvi-root")
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(rgb(default_bg))
            .track_focus(&self.focus_handle)
            .key_context("zenvi");

        let inner = Self::bind_actions(inner, cx);

        let inner = inner
            // IME input handler canvas
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
            // Keyboard input
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                this.trigger_interaction(cx);
                #[cfg(not(target_os = "macos"))]
                if this.is_menu_open {
                    let is_esc = event.keystroke.key == "escape" || event.keystroke.key == "Esc" || event.keystroke.key == "\u{1b}";
                    this.is_menu_open = false;
                    this.active_submenu = None;
                    cx.notify();
                    if is_esc {
                        return;
                    }
                }
                if this.marked_text.is_some() {
                    return;
                }
                if let Some(nvim_key) = key_event_to_nvim(event) {
                    this.session.send_input(&nvim_key);
                }
            }))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _window, _cx| {
                this.open_paths(paths.paths());
            }))
            .child(titlebar_element)
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .pt(px(GRID_PADDING_TOP))
                    .pl(px(GRID_PADDING_LEFT))
                    .overflow_hidden()
                    .when(self.borderless && !is_maximized, |d| {
                        d.rounded_b(px(10.0))
                    })
                    .child(grid_element),
            );

        #[cfg(not(target_os = "macos"))]
        let inner = if self.is_menu_open {
            inner.child(crate::menu::render_app_menu(&state, self.active_submenu, cx))
        } else {
            inner
        };

        let inner = self.bind_mouse_handlers(inner, cx);

        if self.borderless && !is_maximized {
            div()
                .id("zenvi-window-container")
                .size_full()
                .relative()
                .p(shadow_size)
                // Top edge resize handle
                .child(
                    div()
                        .id("resize-handle-top")
                        .absolute()
                        .top_0()
                        .left(shadow_size)
                        .right(shadow_size)
                        .h(shadow_size)
                        .cursor(CursorStyle::ResizeUpDown)
                        .on_mouse_down(MouseButton::Left, cx.listener(|_this, _, window, cx| {
                            cx.stop_propagation();
                            window.start_window_resize(ResizeEdge::Top);
                        })),
                )
                // Bottom edge resize handle
                .child(
                    div()
                        .id("resize-handle-bottom")
                        .absolute()
                        .bottom_0()
                        .left(shadow_size)
                        .right(shadow_size)
                        .h(shadow_size)
                        .cursor(CursorStyle::ResizeUpDown)
                        .on_mouse_down(MouseButton::Left, cx.listener(|_this, _, window, cx| {
                            cx.stop_propagation();
                            window.start_window_resize(ResizeEdge::Bottom);
                        })),
                )
                // Left edge resize handle
                .child(
                    div()
                        .id("resize-handle-left")
                        .absolute()
                        .left_0()
                        .top(shadow_size)
                        .bottom(shadow_size)
                        .w(shadow_size)
                        .cursor(CursorStyle::ResizeLeftRight)
                        .on_mouse_down(MouseButton::Left, cx.listener(|_this, _, window, cx| {
                            cx.stop_propagation();
                            window.start_window_resize(ResizeEdge::Left);
                        })),
                )
                // Right edge resize handle
                .child(
                    div()
                        .id("resize-handle-right")
                        .absolute()
                        .right_0()
                        .top(shadow_size)
                        .bottom(shadow_size)
                        .w(shadow_size)
                        .cursor(CursorStyle::ResizeLeftRight)
                        .on_mouse_down(MouseButton::Left, cx.listener(|_this, _, window, cx| {
                            cx.stop_propagation();
                            window.start_window_resize(ResizeEdge::Right);
                        })),
                )
                // Top-Left corner resize handle
                .child(
                    div()
                        .id("resize-handle-top-left")
                        .absolute()
                        .top_0()
                        .left_0()
                        .size(shadow_size)
                        .cursor(CursorStyle::ResizeUpLeftDownRight)
                        .on_mouse_down(MouseButton::Left, cx.listener(|_this, _, window, cx| {
                            cx.stop_propagation();
                            window.start_window_resize(ResizeEdge::TopLeft);
                        })),
                )
                // Top-Right corner resize handle
                .child(
                    div()
                        .id("resize-handle-top-right")
                        .absolute()
                        .top_0()
                        .right_0()
                        .size(shadow_size)
                        .cursor(CursorStyle::ResizeUpRightDownLeft)
                        .on_mouse_down(MouseButton::Left, cx.listener(|_this, _, window, cx| {
                            cx.stop_propagation();
                            window.start_window_resize(ResizeEdge::TopRight);
                        })),
                )
                // Bottom-Left corner resize handle
                .child(
                    div()
                        .id("resize-handle-bottom-left")
                        .absolute()
                        .bottom_0()
                        .left_0()
                        .size(shadow_size)
                        .cursor(CursorStyle::ResizeUpRightDownLeft)
                        .on_mouse_down(MouseButton::Left, cx.listener(|_this, _, window, cx| {
                            cx.stop_propagation();
                            window.start_window_resize(ResizeEdge::BottomLeft);
                        })),
                )
                // Bottom-Right corner resize handle
                .child(
                    div()
                        .id("resize-handle-bottom-right")
                        .absolute()
                        .bottom_0()
                        .right_0()
                        .size(shadow_size)
                        .cursor(CursorStyle::ResizeUpLeftDownRight)
                        .on_mouse_down(MouseButton::Left, cx.listener(|_this, _, window, cx| {
                            cx.stop_propagation();
                            window.start_window_resize(ResizeEdge::BottomRight);
                        })),
                )
                .child(
                    inner
                        .rounded(px(10.0))
                        .border_1()
                        .border_color(style.border_color)
                        .shadow(vec![
                            // Ambient close contact contour shadow
                            BoxShadow {
                                color: Hsla {
                                    h: 0.0,
                                    s: 0.0,
                                    l: 0.0,
                                    a: 0.26,
                                },
                                blur_radius: px(2.0),
                                spread_radius: px(0.0),
                                offset: point(px(0.0), px(1.0)),
                            },
                            // Compact soft ambient shadow (matching Zed's subtle shadow)
                            BoxShadow {
                                color: Hsla {
                                    h: 0.0,
                                    s: 0.0,
                                    l: 0.0,
                                    a: 0.18,
                                },
                                blur_radius: px(4.5),
                                spread_radius: px(0.0),
                                offset: point(px(0.0), px(1.5)),
                            },
                            // Light soft feather edge
                            BoxShadow {
                                color: Hsla {
                                    h: 0.0,
                                    s: 0.0,
                                    l: 0.0,
                                    a: 0.12,
                                },
                                blur_radius: px(8.0),
                                spread_radius: px(0.5),
                                offset: point(px(0.0), px(2.0)),
                            },
                        ])
                        .overflow_hidden(),
                )
        } else {
            inner
        }
    }
}



