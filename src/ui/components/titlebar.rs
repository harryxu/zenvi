use super::style::derive_titlebar_style;
use crate::nvim::state::NvimState;
use crate::ui::{ZenviView, TITLEBAR_HEIGHT};
use gpui::prelude::*;
use gpui::*;

/// Formats the raw Neovim title, preserving the active file path while branding the shell as Zenvi.
pub fn format_title(raw_title: &str) -> String {
    let t = raw_title.trim();
    if t.is_empty() || t == "Nvim" || t == "nvim" || t == "NVIM" {
        "Zenvi".to_string()
    } else {
        t.replace(" - NVIM", " - Zenvi")
            .replace(" - Nvim", " - Zenvi")
            .replace(" - nvim", " - Zenvi")
    }
}

/// Builds the custom titlebar element directly from Neovim's state (macOS version).
#[cfg(target_os = "macos")]
pub fn render_titlebar(
    state: &NvimState,
    cx: &mut Context<ZenviView>,
) -> Stateful<Div> {
    let title = format_title(&state.title);
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
                .child(title),
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
}

/// Builds the custom titlebar element directly from Neovim's state (Linux / Windows version).
#[cfg(not(target_os = "macos"))]
pub fn render_titlebar(
    state: &NvimState,
    is_menu_open: bool,
    borderless: bool,
    window: &Window,
    cx: &mut Context<ZenviView>,
) -> Stateful<Div> {
    let title = format_title(&state.title);
    let style = derive_titlebar_style(state.default_bg, state.default_fg);
    let default_bg = state.default_bg;

    let left_side = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .id("menu-btn-toggle")
                .px(px(6.0))
                .py(px(2.0))
                .rounded_sm()
                .text_size(px(13.0))
                .text_color(style.title_color)
                .cursor_pointer()
                .when(is_menu_open, |s| s.bg(style.menu_active_bg))
                .hover(move |s| s.bg(style.menu_hover_bg))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _window, cx| {
                        cx.stop_propagation();
                        this.is_menu_open = !this.is_menu_open;
                        this.active_submenu = None;
                        cx.notify();
                    }),
                )
                .child("☰"),
        )
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::BOLD)
                .text_color(style.title_color)
                .child(title.to_string()),
        );

    let right_side = if borderless {
        let is_maximized = window.is_maximized();

        let min_btn = div()
            .id("win-ctrl-min")
            .w(px(38.0))
            .h(px(TITLEBAR_HEIGHT))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(move |s| s.bg(style.menu_hover_bg))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_this, _, window, cx| {
                    cx.stop_propagation();
                    window.minimize_window();
                }),
            )
            .child(
                div()
                    .w(px(10.0))
                    .h(px(1.5))
                    .bg(style.title_color),
            );

        let max_btn = div()
            .id("win-ctrl-max")
            .w(px(38.0))
            .h(px(TITLEBAR_HEIGHT))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(move |s| s.bg(style.menu_hover_bg))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_this, _, window, cx| {
                    cx.stop_propagation();
                    window.zoom_window();
                }),
            )
            .child(if is_maximized {
                // Restore icon (two layered boxes)
                div()
                    .relative()
                    .size(px(10.0))
                    .child(
                        div()
                            .absolute()
                            .top(px(0.0))
                            .right(px(0.0))
                            .size(px(7.5))
                            .border_1()
                            .border_color(style.title_color),
                    )
                    .child(
                        div()
                            .absolute()
                            .bottom(px(0.0))
                            .left(px(0.0))
                            .size(px(7.5))
                            .bg(rgb(default_bg))
                            .border_1()
                            .border_color(style.title_color),
                    )
            } else {
                // Maximize icon (single box)
                div()
                    .size(px(9.0))
                    .border_1()
                    .border_color(style.title_color)
            });

        let close_btn = div()
            .id("win-ctrl-close")
            .w(px(42.0))
            .h(px(TITLEBAR_HEIGHT))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .when(!is_maximized, |d| d.rounded_tr(px(10.0)))
            .hover(|s| s.bg(rgb(0xe81123)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_this, _, window, cx| {
                    cx.stop_propagation();
                    window.remove_window();
                }),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(style.title_color)
                    .child("✕"),
            );

        div()
            .flex()
            .flex_row()
            .items_center()
            .h_full()
            .child(min_btn)
            .child(max_btn)
            .child(close_btn)
    } else {
        div()
    };

    let bar = div()
        .id("zenvi-titlebar")
        .relative()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .pl(px(8.0))
        .pr(if borderless { px(0.0) } else { px(12.0) })
        .bg(rgb(default_bg))
        .border_b_1()
        .border_color(style.border_color)
        .when(borderless && !window.is_maximized(), |d| {
            d.rounded_t(px(10.0))
        })
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                if event.click_count == 2 {
                    window.zoom_window();
                } else if event.click_count == 1 {
                    window.start_window_move();
                }
            }),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|_this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                window.show_window_menu(event.position);
            }),
        )
        .child(left_side)
        .child(right_side);

    if borderless {
        bar.window_control_area(WindowControlArea::Drag)
    } else {
        bar
    }
}

#[cfg(test)]
mod tests {
    use super::format_title;

    #[test]
    fn test_format_title_empty_and_default() {
        assert_eq!(format_title(""), "Zenvi");
        assert_eq!(format_title("Nvim"), "Zenvi");
        assert_eq!(format_title("nvim"), "Zenvi");
        assert_eq!(format_title("NVIM"), "Zenvi");
    }

    #[test]
    fn test_format_title_with_filepath() {
        assert_eq!(
            format_title("Cargo.toml (/home/user/dev) - NVIM"),
            "Cargo.toml (/home/user/dev) - Zenvi"
        );
        assert_eq!(
            format_title("src/main.rs [+] - Nvim"),
            "src/main.rs [+] - Zenvi"
        );
        assert_eq!(
            format_title("/etc/hosts"),
            "/etc/hosts"
        );
    }
}
