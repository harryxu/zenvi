use super::style::derive_titlebar_style;
use crate::nvim::state::NvimState;
use crate::ui::{ZenviView, TITLEBAR_HEIGHT};
use gpui::prelude::*;
use gpui::*;

#[cfg(not(target_os = "macos"))]
use super::menu::ActiveMenu;

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
