use super::style::TitlebarStyle;
use crate::ui::{ZenviView, TITLEBAR_HEIGHT};
use gpui::prelude::*;
use gpui::*;

/// Formats the raw Neovim title, preserving the active file path while branding the shell as Zenvi.
pub fn format_title(raw_title: &str) -> String {
    let t = raw_title.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("nvim") {
        "Zenvi".to_string()
    } else if let Some(prefix) = t.strip_suffix(" - NVIM")
        .or_else(|| t.strip_suffix(" - Nvim"))
        .or_else(|| t.strip_suffix(" - nvim"))
    {
        format!("{prefix} - Zenvi")
    } else {
        t.to_string()
    }
}

/// Renders the hamburger menu toggle button (Linux / Windows).
#[cfg(not(target_os = "macos"))]
fn render_menu_button(
    is_menu_open: bool,
    style: &TitlebarStyle,
    cx: &mut Context<ZenviView>,
) -> impl IntoElement {
    div()
        .id("menu-btn-toggle")
        .flex()
        .items_center()
        .justify_center()
        .px(px(6.0))
        .py(px(4.0))
        .rounded_sm()
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
        .child(
            svg()
                .path("icons/menu.svg")
                .size(px(14.0))
                .text_color(style.title_color),
        )
}

/// Renders the window controls (minimize, maximize/restore, close) for client-side decorations (Linux / Windows).
#[cfg(not(target_os = "macos"))]
fn render_window_controls(
    default_bg: u32,
    style: &TitlebarStyle,
    window: &Window,
    cx: &mut Context<ZenviView>,
) -> impl IntoElement {
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
            cx.listener(|this, _, window, cx| {
                cx.stop_propagation();
                this.session.send_command("qa");
                let window_handle = window.window_handle();
                cx.defer(move |cx| {
                    if cx.windows().len() <= 1 {
                        cx.quit();
                    } else if cx.windows().contains(&window_handle) {
                        let _ = window_handle.update(cx, |_, window, _cx| {
                            window.remove_window();
                        });
                    }
                });
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
}

/// Renders the left panel toggle button on the right side of the titlebar.
fn render_left_panel_button(
    is_panel_open: bool,
    style: &TitlebarStyle,
    cx: &mut Context<ZenviView>,
) -> impl IntoElement {
    let icon_path = if is_panel_open {
        "icons/panel-left-open.svg"
    } else {
        "icons/panel-left.svg"
    };

    div()
        .id("panel-left-btn-toggle")
        .flex()
        .items_center()
        .justify_center()
        .px(px(6.0))
        .py(px(4.0))
        .rounded_sm()
        .cursor_pointer()
        .hover(move |s| s.bg(style.menu_hover_bg))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _, _window, cx| {
                cx.stop_propagation();
                this.toggle_left_panel(cx);
            }),
        )
        .child(
            svg()
                .path(icon_path)
                .size(px(16.0))
                .text_color(style.title_color),
        )
}

/// Builds the custom titlebar element using precomputed title and style.
pub fn render_titlebar(
    title: &str,
    style: &TitlebarStyle,
    default_bg: u32,
    is_left_panel_open: bool,
    #[cfg_attr(target_os = "macos", allow(unused_variables))] is_menu_open: bool,
    #[cfg_attr(target_os = "macos", allow(unused_variables))] borderless: bool,
    #[cfg_attr(target_os = "macos", allow(unused_variables))] window: &Window,
    cx: &mut Context<ZenviView>,
) -> Stateful<Div> {
    let left_side = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0));

    #[cfg(not(target_os = "macos"))]
    let left_side = left_side.child(render_menu_button(is_menu_open, style, cx));

    let left_side = left_side.child(
        div()
            .text_size(px(12.0))
            .font_weight(FontWeight::BOLD)
            .text_color(style.title_color)
            .child(title.to_string()),
    );

    let panel_button = render_left_panel_button(is_left_panel_open, style, cx);

    let bar = div()
        .id("zenvi-titlebar")
        .relative()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .bg(rgb(default_bg))
        .border_b_1()
        .border_color(style.border_color);

    #[cfg(target_os = "macos")]
    let bar = bar
        .pl(px(78.0))
        .pr(px(12.0))
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
        .child(panel_button);

    #[cfg(not(target_os = "macos"))]
    let bar = {
        let is_maximized = window.is_maximized();
        let window_controls = if borderless {
            Some(render_window_controls(default_bg, &style, window, cx))
        } else {
            None
        };

        let right_side = div()
            .flex()
            .flex_row()
            .items_center()
            .child(
                div()
                    .pr(if borderless { px(8.0) } else { px(0.0) })
                    .child(panel_button),
            )
            .children(window_controls);

        let bar = bar
            .pl(px(8.0))
            .pr(if borderless { px(0.0) } else { px(12.0) })
            .when(borderless && !is_maximized, |d| {
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
            .child(left_side)
            .child(right_side);

        if borderless {
            bar.window_control_area(WindowControlArea::Drag)
        } else {
            bar
        }
    };

    bar
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
