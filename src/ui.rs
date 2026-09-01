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
    pub scroll_accum_y: f32,
    pub cwd: Option<PathBuf>,
    pub marked_text: Option<String>,
    /// Whether the window is running in client-side decorations (borderless) mode.
    pub borderless: bool,
    /// The currently active window shadow inset (in pixels).
    pub current_shadow_size: f32,
    #[cfg(not(target_os = "macos"))]
    pub is_menu_open: bool,
    #[cfg(not(target_os = "macos"))]
    pub active_submenu: Option<usize>,
    pub(crate) _event_task: Option<Task<()>>,
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
            scroll_accum_y: 0.0,
            cwd,
            marked_text: None,
            borderless,
            current_shadow_size: 0.0,
            #[cfg(not(target_os = "macos"))]
            is_menu_open: false,
            #[cfg(not(target_os = "macos"))]
            active_submenu: None,
            _event_task: Some(event_task),
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
                    let mut needs_redraw = false;
                    let mut should_exit = false;

                    match event {
                        NvimEvent::Redraw => needs_redraw = true,
                        NvimEvent::Exit => should_exit = true,
                    }

                    // Coalesce burst redraw notifications into a single render pass
                    while let Ok(next_event) = event_rx.try_recv() {
                        match next_event {
                            NvimEvent::Redraw => needs_redraw = true,
                            NvimEvent::Exit => should_exit = true,
                        }
                    }

                    if needs_redraw {
                        let _ = this.update(&mut cx, |_this, cx| {
                            cx.notify();
                        });
                    }

                    if should_exit {
                        let _ = cx.update(|cx| {
                            if cx.windows().len() <= 1 {
                                cx.quit();
                            } else {
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
    fn bind_mouse_handlers(root: Stateful<Div>, cx: &mut Context<Self>) -> Stateful<Div> {
        root.on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, event: &MouseDownEvent, window, _cx| {
                this.handle_mouse_down("left", event.position, &event.modifiers, window);
            }),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|this, event: &MouseDownEvent, window, _cx| {
                this.handle_mouse_down("right", event.position, &event.modifiers, window);
            }),
        )
        .on_mouse_down(
            MouseButton::Middle,
            cx.listener(|this, event: &MouseDownEvent, window, _cx| {
                this.handle_mouse_down("middle", event.position, &event.modifiers, window);
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, event: &MouseUpEvent, _window, _cx| {
                this.handle_mouse_up("left", event.position, &event.modifiers);
            }),
        )
        .on_mouse_up(
            MouseButton::Right,
            cx.listener(|this, event: &MouseUpEvent, _window, _cx| {
                this.handle_mouse_up("right", event.position, &event.modifiers);
            }),
        )
        .on_mouse_up(
            MouseButton::Middle,
            cx.listener(|this, event: &MouseUpEvent, _window, _cx| {
                this.handle_mouse_up("middle", event.position, &event.modifiers);
            }),
        )
        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, _cx| {
            this.handle_mouse_move(event);
        }))
        .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _window, _cx| {
            this.handle_scroll_wheel(event);
        }))
    }
}

/// Determines which edge or corner of the window the mouse is hovering over for resizing.
pub fn resize_edge(pos: Point<Pixels>, hit_size: Pixels, size: Size<Pixels>) -> Option<ResizeEdge> {
    let edge = if pos.y < hit_size && pos.x < hit_size {
        ResizeEdge::TopLeft
    } else if pos.y < hit_size && pos.x > size.width - hit_size {
        ResizeEdge::TopRight
    } else if pos.y < hit_size {
        ResizeEdge::Top
    } else if pos.y > size.height - hit_size && pos.x < hit_size {
        ResizeEdge::BottomLeft
    } else if pos.y > size.height - hit_size && pos.x > size.width - hit_size {
        ResizeEdge::BottomRight
    } else if pos.y > size.height - hit_size {
        ResizeEdge::Bottom
    } else if pos.x < hit_size {
        ResizeEdge::Left
    } else if pos.x > size.width - hit_size {
        ResizeEdge::Right
    } else {
        return None;
    };
    Some(edge)
}

impl Render for ZenviView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_font_if_changed(cx);

        // Read Neovim state for rendering
        let state = self.session.state.read();
        let default_bg = state.default_bg;
        let style = components::style::derive_titlebar_style(state.default_bg, state.default_fg);

        let is_maximized = window.is_maximized();
        let shadow_size = if self.borderless && !is_maximized {
            px(10.0)
        } else {
            px(0.0)
        };
        self.current_shadow_size = shadow_size.into();
        window.set_client_inset(shadow_size);

        let title = if state.title.is_empty() {
            "Zenvi"
        } else {
            &state.title
        };
        window.set_window_title(title);

        let default_grid = crate::nvim::state::Grid::new(1, 80, 24);
        let grid = state
            .grids
            .get(&1)
            .or_else(|| state.grids.get(&state.active_grid))
            .unwrap_or(&default_grid);

        // Calculate grid dimensions and notify Neovim of resize
        let viewport = window.viewport_size();
        let window_w: f32 = viewport.width.into();
        let window_h: f32 = viewport.height.into();
        let shadow_f32: f32 = shadow_size.into();
        let content_w = (window_w - shadow_f32 * 2.0).max(100.0);
        let content_h = (window_h - shadow_f32 * 2.0).max(100.0);
        let lh: f32 = self.line_height.into();

        let cols = ((content_w - GRID_PADDING_LEFT) / self.char_width)
            .floor()
            .max(20.0) as usize;
        let rows = ((content_h - TOP_OFFSET) / lh).floor().max(5.0) as usize;

        if cols != self.last_cols || rows != self.last_rows {
            self.last_cols = cols;
            self.last_rows = rows;
            self.session.try_resize(cols, rows);
        }

        let grid_element = components::grid::render_grid(
            &state,
            grid,
            &self.font_family,
            self.font_size,
            self.line_height,
            self.char_width,
        );

        let focus_handle = self.focus_handle.clone();
        let entity = cx.entity().clone();

        #[cfg(target_os = "macos")]
        let titlebar_element = components::titlebar::render_titlebar(&state, cx);
        #[cfg(not(target_os = "macos"))]
        let titlebar_element = components::titlebar::render_titlebar(&state, self.is_menu_open, self.borderless, window, cx);

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
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, _cx| {
                #[cfg(not(target_os = "macos"))]
                if this.is_menu_open {
                    let is_esc = event.keystroke.key == "escape" || event.keystroke.key == "Esc" || event.keystroke.key == "\u{1b}";
                    this.is_menu_open = false;
                    this.active_submenu = None;
                    _cx.notify();
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
                    .child(grid_element),
            );

        #[cfg(not(target_os = "macos"))]
        let inner = if self.is_menu_open {
            inner.child(crate::menu::render_app_menu(&state, self.active_submenu, cx))
        } else {
            inner
        };

        let inner = Self::bind_mouse_handlers(inner, cx);

        if self.borderless && !is_maximized {
            let resize_hit_size = px(8.0).max(shadow_size);
            div()
                .id("zenvi-window-container")
                .size_full()
                .relative()
                .bg(transparent_black())
                .p(shadow_size)
                // Canvas with hitbox to detect mouse edge and set cursor style
                .child(
                    canvas(
                        |_bounds, window, _cx| {
                            window.insert_hitbox(
                                Bounds::new(
                                    point(px(0.0), px(0.0)),
                                    window.window_bounds().get_bounds().size,
                                ),
                                HitboxBehavior::Normal,
                            )
                        },
                        move |_bounds, hitbox, window, _cx| {
                            let mouse = window.mouse_position();
                            let size = window.window_bounds().get_bounds().size;
                            let Some(edge) = resize_edge(mouse, resize_hit_size, size) else {
                                return;
                            };
                            window.set_cursor_style(
                                match edge {
                                    ResizeEdge::Top | ResizeEdge::Bottom => {
                                        CursorStyle::ResizeUpDown
                                    }
                                    ResizeEdge::Left | ResizeEdge::Right => {
                                        CursorStyle::ResizeLeftRight
                                    }
                                    ResizeEdge::TopLeft | ResizeEdge::BottomRight => {
                                        CursorStyle::ResizeUpLeftDownRight
                                    }
                                    ResizeEdge::TopRight | ResizeEdge::BottomLeft => {
                                        CursorStyle::ResizeUpRightDownLeft
                                    }
                                },
                                &hitbox,
                            );
                        },
                    )
                    .size_full()
                    .absolute(),
                )
                .on_mouse_move(|_e, window, _cx| window.refresh())
                .on_mouse_down(MouseButton::Left, move |e, window, _cx| {
                    let size = window.window_bounds().get_bounds().size;
                    if let Some(edge) = resize_edge(e.position, resize_hit_size, size) {
                        window.start_window_resize(edge);
                    }
                })
                .child(
                    inner
                        .rounded(px(8.0))
                        .border_1()
                        .border_color(style.border_color)
                        .shadow(vec![BoxShadow {
                            color: Hsla {
                                h: 0.0,
                                s: 0.0,
                                l: 0.0,
                                a: 0.45,
                            },
                            blur_radius: shadow_size,
                            spread_radius: px(0.0),
                            offset: point(px(0.0), px(2.0)),
                        }])
                        .overflow_hidden()
                        .on_mouse_move(|_e, _, cx| {
                            cx.stop_propagation();
                        }),
                )
        } else {
            inner
        }
    }
}


