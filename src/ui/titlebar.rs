use super::ZenviView;
use super::TITLEBAR_HEIGHT;
use crate::nvim::state::NvimState;
use gpui::prelude::*;
use gpui::*;

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveMenu {
    File,
    Edit,
}

/// Pre-computed titlebar colors derived from the Neovim theme.
pub struct TitlebarStyle {
    pub title_color: Rgba,
    pub badge_color: Rgba,
    pub border_color: Rgba,
    #[cfg(not(target_os = "macos"))]
    pub menu_hover_bg: Rgba,
    #[cfg(not(target_os = "macos"))]
    pub menu_active_bg: Rgba,
    #[cfg(not(target_os = "macos"))]
    pub dropdown_bg: Rgba,
}

/// Packs three floating-point channel values into a single `u32` RGB color.
fn pack_rgb(r: f32, g: f32, b: f32) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Derives harmonious titlebar colors from Neovim's active theme colors.
pub fn derive_titlebar_style(default_bg: u32, default_fg: u32) -> TitlebarStyle {
    let (bg_r, bg_g, bg_b) = (
        ((default_bg >> 16) & 0xff) as f32,
        ((default_bg >> 8) & 0xff) as f32,
        (default_bg & 0xff) as f32,
    );
    let luminance = 0.299 * bg_r + 0.587 * bg_g + 0.114 * bg_b;
    let is_dark = luminance < 128.0;

    let title_color = rgb(default_fg);

    let badge_color = if is_dark {
        rgb(pack_rgb(
            (bg_r + 80.0).min(200.0),
            (bg_g + 80.0).min(200.0),
            (bg_b + 80.0).min(200.0),
        ))
    } else {
        rgb(pack_rgb(
            (bg_r - 80.0).max(60.0),
            (bg_g - 80.0).max(60.0),
            (bg_b - 80.0).max(60.0),
        ))
    };

    let border_color = if is_dark {
        rgb(pack_rgb(
            (bg_r + 18.0).min(255.0),
            (bg_g + 18.0).min(255.0),
            (bg_b + 18.0).min(255.0),
        ))
    } else {
        rgb(pack_rgb(
            (bg_r - 25.0).max(0.0),
            (bg_g - 25.0).max(0.0),
            (bg_b - 25.0).max(0.0),
        ))
    };

    #[cfg(not(target_os = "macos"))]
    let menu_hover_bg = if is_dark {
        rgb(pack_rgb(
            (bg_r + 20.0).min(255.0),
            (bg_g + 20.0).min(255.0),
            (bg_b + 20.0).min(255.0),
        ))
    } else {
        rgb(pack_rgb(
            (bg_r - 20.0).max(0.0),
            (bg_g - 20.0).max(0.0),
            (bg_b - 20.0).max(0.0),
        ))
    };

    #[cfg(not(target_os = "macos"))]
    let menu_active_bg = if is_dark {
        rgb(pack_rgb(
            (bg_r + 35.0).min(255.0),
            (bg_g + 35.0).min(255.0),
            (bg_b + 35.0).min(255.0),
        ))
    } else {
        rgb(pack_rgb(
            (bg_r - 35.0).max(0.0),
            (bg_g - 35.0).max(0.0),
            (bg_b - 35.0).max(0.0),
        ))
    };

    #[cfg(not(target_os = "macos"))]
    let dropdown_bg = if is_dark {
        rgb(pack_rgb(
            (bg_r + 14.0).min(255.0),
            (bg_g + 14.0).min(255.0),
            (bg_b + 14.0).min(255.0),
        ))
    } else {
        rgb(pack_rgb(
            (bg_r - 12.0).max(0.0),
            (bg_g - 12.0).max(0.0),
            (bg_b - 12.0).max(0.0),
        ))
    };

    TitlebarStyle {
        title_color,
        badge_color,
        border_color,
        #[cfg(not(target_os = "macos"))]
        menu_hover_bg,
        #[cfg(not(target_os = "macos"))]
        menu_active_bg,
        #[cfg(not(target_os = "macos"))]
        dropdown_bg,
    }
}

#[cfg(not(target_os = "macos"))]
type MenuAction = Box<dyn Fn(&mut ZenviView, &mut Window, &mut Context<ZenviView>) + 'static>;

#[cfg(not(target_os = "macos"))]
struct DropdownItem {
    label: &'static str,
    shortcut: &'static str,
    is_separator: bool,
    action: Option<MenuAction>,
}

#[cfg(not(target_os = "macos"))]
impl DropdownItem {
    fn action<F>(label: &'static str, shortcut: &'static str, f: F) -> Self
    where
        F: Fn(&mut ZenviView, &mut Window, &mut Context<ZenviView>) + 'static,
    {
        Self {
            label,
            shortcut,
            is_separator: false,
            action: Some(Box::new(f)),
        }
    }

    fn separator() -> Self {
        Self {
            label: "",
            shortcut: "",
            is_separator: true,
            action: None,
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn file_menu_items() -> Vec<DropdownItem> {
    vec![
        DropdownItem::action("New Window", "Ctrl+Shift+N", |this, _window, cx| {
            crate::window::open_zenvi_window(this.cwd.clone(), Vec::new(), cx);
        }),
        DropdownItem::separator(),
        DropdownItem::action("Open File...", "Ctrl+Shift+O", |this, _window, cx| {
            this.open_file(cx);
        }),
        DropdownItem::action("Open Folder...", "Ctrl+Alt+O", |this, _window, cx| {
            this.open_folder(cx);
        }),
        DropdownItem::action("Open Neovim Config", "Ctrl+Shift+,", |_this, _window, cx| {
            let config_dir = crate::window::get_nvim_config_dir();
            if !config_dir.exists() {
                let _ = std::fs::create_dir_all(&config_dir);
            }
            crate::window::open_zenvi_window(Some(config_dir), Vec::new(), cx);
        }),
        DropdownItem::separator(),
        DropdownItem::action("Reload Neovim", "Ctrl+Shift+R", |this, _window, cx| {
            this.reload_nvim(cx);
        }),
        DropdownItem::action("Install Shell Command", "", |this, _window, cx| {
            this.install_cli(cx);
        }),
        DropdownItem::separator(),
        DropdownItem::action("Close Buffer", "Ctrl+Shift+W", |this, _window, cx| {
            this.close_buffer(cx);
        }),
        DropdownItem::separator(),
        DropdownItem::action("Exit Zenvi", "Ctrl+Shift+Q", |_this, _window, cx| {
            cx.quit();
        }),
    ]
}

#[cfg(not(target_os = "macos"))]
fn edit_menu_items() -> Vec<DropdownItem> {
    vec![
        DropdownItem::action("Undo", "Ctrl+Shift+Z", |this, _window, cx| {
            this.undo(cx);
        }),
        DropdownItem::action("Redo", "Ctrl+Shift+Y", |this, _window, cx| {
            this.redo(cx);
        }),
        DropdownItem::separator(),
        DropdownItem::action("Cut", "Ctrl+Shift+X", |this, _window, cx| {
            this.cut(cx);
        }),
        DropdownItem::action("Copy", "Ctrl+Shift+C", |this, _window, cx| {
            this.copy(cx);
        }),
        DropdownItem::action("Paste", "Ctrl+Shift+V", |this, _window, cx| {
            this.paste(cx);
        }),
        DropdownItem::action("Select All", "Ctrl+Shift+A", |this, _window, cx| {
            this.select_all(cx);
        }),
    ]
}

#[cfg(not(target_os = "macos"))]
fn render_dropdown(
    items: Vec<DropdownItem>,
    left_offset: Pixels,
    style: &TitlebarStyle,
    cx: &mut Context<ZenviView>,
) -> impl IntoElement {
    let style_border = style.border_color;
    let style_dropdown_bg = style.dropdown_bg;
    let style_item_hover = style.menu_hover_bg;
    let style_title_color = style.title_color;
    let style_badge_color = style.badge_color;

    div()
        .absolute()
        .top(px(TITLEBAR_HEIGHT))
        .left(left_offset)
        .min_w(px(230.0))
        .bg(style_dropdown_bg)
        .border_1()
        .border_color(style_border)
        .rounded_md()
        .shadow_md()
        .py(px(4.0))
        .children(items.into_iter().map(move |item| {
            if item.is_separator {
                div()
                    .h(px(1.0))
                    .my(px(4.0))
                    .mx(px(6.0))
                    .bg(style_border)
                    .into_any_element()
            } else {
                let action = item.action;
                div()
                    .h(px(26.0))
                    .mx(px(4.0))
                    .px(px(8.0))
                    .rounded_sm()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .hover(move |s| s.bg(style_item_hover))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.active_menu = None;
                            if let Some(ref f) = action {
                                f(this, window, cx);
                            }
                        }),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(style_title_color)
                            .child(item.label),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(style_badge_color)
                            .child(item.shortcut),
                    )
                    .into_any_element()
            }
        }))
}

/// Exported dropdown renderer to mount at root level with topmost z-index
#[cfg(not(target_os = "macos"))]
pub fn render_menu_dropdown(
    active: ActiveMenu,
    state: &NvimState,
    cx: &mut Context<ZenviView>,
) -> impl IntoElement {
    let style = derive_titlebar_style(state.default_bg, state.default_fg);
    match active {
        ActiveMenu::File => render_dropdown(file_menu_items(), px(12.0), &style, cx),
        ActiveMenu::Edit => render_dropdown(edit_menu_items(), px(56.0), &style, cx),
    }
}

/// Builds the custom titlebar element directly from Neovim's state (macOS version).
#[cfg(target_os = "macos")]
pub fn render_titlebar(
    state: &NvimState,
    cx: &mut Context<ZenviView>,
) -> Stateful<Div> {
    let title = if state.title.is_empty() {
        "Zenvi"
    } else {
        &state.title
    };
    let style = derive_titlebar_style(state.default_bg, state.default_fg);
    let default_bg = state.default_bg;

    let left_side = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::BOLD)
                .text_color(style.title_color)
                .child(title.to_string()),
        );

    div()
        .id("zenvi-titlebar")
        .relative()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .pl(px(78.0))
        .pr(px(12.0))
        .bg(rgb(default_bg))
        .border_b_1()
        .border_color(style.border_color)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                if event.click_count == 2 {
                    window.titlebar_double_click();
                }
            }),
        )
        .child(left_side)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(style.badge_color)
                        .child("⚡️ GPUI"),
                ),
        )
}

/// Builds the custom titlebar element directly from Neovim's state (Linux / Windows version).
#[cfg(not(target_os = "macos"))]
pub fn render_titlebar(
    state: &NvimState,
    active_menu: Option<ActiveMenu>,
    cx: &mut Context<ZenviView>,
) -> Stateful<Div> {
    let style = derive_titlebar_style(state.default_bg, state.default_fg);
    let default_bg = state.default_bg;

    let file_active = active_menu == Some(ActiveMenu::File);
    let edit_active = active_menu == Some(ActiveMenu::Edit);

    let left_side = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .child(
                    div()
                        .id("menu-btn-file")
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded_sm()
                        .text_size(px(12.0))
                        .text_color(style.title_color)
                        .cursor_pointer()
                        .when(file_active, |s| s.bg(style.menu_active_bg))
                        .hover(move |s| s.bg(style.menu_hover_bg))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _window, cx| {
                                cx.stop_propagation();
                                this.active_menu = if this.active_menu == Some(ActiveMenu::File) {
                                    None
                                } else {
                                    Some(ActiveMenu::File)
                                };
                                cx.notify();
                            }),
                        )
                        .on_mouse_move(cx.listener(|this, _, _window, cx| {
                            if this.active_menu.is_some() && this.active_menu != Some(ActiveMenu::File) {
                                this.active_menu = Some(ActiveMenu::File);
                                cx.notify();
                            }
                        }))
                        .child("File"),
                )
                .child(
                    div()
                        .id("menu-btn-edit")
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded_sm()
                        .text_size(px(12.0))
                        .text_color(style.title_color)
                        .cursor_pointer()
                        .when(edit_active, |s| s.bg(style.menu_active_bg))
                        .hover(move |s| s.bg(style.menu_hover_bg))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _window, cx| {
                                cx.stop_propagation();
                                this.active_menu = if this.active_menu == Some(ActiveMenu::Edit) {
                                    None
                                } else {
                                    Some(ActiveMenu::Edit)
                                };
                                cx.notify();
                            }),
                        )
                        .on_mouse_move(cx.listener(|this, _, _window, cx| {
                            if this.active_menu.is_some() && this.active_menu != Some(ActiveMenu::Edit) {
                                this.active_menu = Some(ActiveMenu::Edit);
                                cx.notify();
                            }
                        }))
                        .child("Edit"),
                ),
        );

    div()
        .id("zenvi-titlebar")
        .relative()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .pl(px(12.0))
        .pr(px(12.0))
        .bg(rgb(default_bg))
        .border_b_1()
        .border_color(style.border_color)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                if event.click_count == 2 {
                    window.titlebar_double_click();
                }
            }),
        )
        .child(left_side)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(style.badge_color)
                        .child("⚡️ GPUI"),
                ),
        )
}
