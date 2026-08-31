use super::style::TitlebarStyle;
use crate::ui::{ZenviView, TITLEBAR_HEIGHT};
use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;

pub type MenuAction = Arc<dyn Fn(&mut ZenviView, &mut Window, &mut Context<ZenviView>) + 'static>;

#[derive(Clone)]
pub enum MenuItem {
    Action {
        label: &'static str,
        shortcut: &'static str,
        action: MenuAction,
    },
    Submenu {
        label: &'static str,
        items: Vec<MenuItem>,
    },
    Separator,
}

impl MenuItem {
    pub fn action<F>(label: &'static str, shortcut: &'static str, f: F) -> Self
    where
        F: Fn(&mut ZenviView, &mut Window, &mut Context<ZenviView>) + 'static,
    {
        Self::Action {
            label,
            shortcut,
            action: Arc::new(f),
        }
    }

    pub fn submenu(label: &'static str, items: Vec<MenuItem>) -> Self {
        Self::Submenu { label, items }
    }

    pub fn separator() -> Self {
        Self::Separator
    }
}

const MENU_WIDTH: f32 = 220.0;
const ITEM_HEIGHT: f32 = 26.0;
const SEPARATOR_HEIGHT: f32 = 9.0;
const MENU_PADDING_Y: f32 = 4.0;

/// Renders a flat submenu panel
fn render_flat_panel(
    items: &[MenuItem],
    right_offset: Pixels,
    top_offset: Pixels,
    style: &TitlebarStyle,
    cx: &mut Context<ZenviView>,
) -> impl IntoElement {
    let style_border = style.border_color;
    let style_dropdown_bg = style.dropdown_bg;
    let style_item_hover = style.menu_hover_bg;
    let style_title_color = style.title_color;
    let style_badge_color = style.badge_color;

    let mut item_elements = Vec::with_capacity(items.len());

    for item in items {
        match item {
            MenuItem::Separator => {
                item_elements.push(
                    div()
                        .h(px(1.0))
                        .my(px(4.0))
                        .mx(px(6.0))
                        .bg(style_border)
                        .into_any_element(),
                );
            }
            MenuItem::Action {
                label,
                shortcut,
                action,
            } => {
                let action = action.clone();
                item_elements.push(
                    div()
                        .h(px(ITEM_HEIGHT))
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
                                this.is_menu_open = false;
                                this.active_submenu = None;
                                action(this, window, cx);
                            }),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(style_title_color)
                                .child(*label),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(style_badge_color)
                                .child(*shortcut),
                        )
                        .into_any_element(),
                );
            }
            MenuItem::Submenu { .. } => {}
        }
    }

    div()
        .absolute()
        .top(top_offset)
        .right(right_offset)
        .w(px(MENU_WIDTH))
        .bg(style_dropdown_bg)
        .border_1()
        .border_color(style_border)
        .rounded_md()
        .shadow_md()
        .py(px(MENU_PADDING_Y))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_, _, _, cx| cx.stop_propagation()),
        )
        .children(item_elements)
}

/// Renders the root application menu with cascading submenus
pub fn render_cascading_menu(
    items: Vec<MenuItem>,
    right_offset: Pixels,
    style: &TitlebarStyle,
    active_submenu: Option<usize>,
    cx: &mut Context<ZenviView>,
) -> impl IntoElement {
    let style_border = style.border_color;
    let style_dropdown_bg = style.dropdown_bg;
    let style_item_hover = style.menu_hover_bg;
    let style_title_color = style.title_color;
    let style_badge_color = style.badge_color;

    // Track active submenu and its top vertical position
    let mut submenu_to_render: Option<(Vec<MenuItem>, Pixels)> = None;
    let mut current_top = MENU_PADDING_Y;

    for (idx, item) in items.iter().enumerate() {
        if let MenuItem::Submenu { items: sub_items, .. } = item {
            if active_submenu == Some(idx) {
                submenu_to_render = Some((sub_items.clone(), px(TITLEBAR_HEIGHT + current_top)));
            }
        }
        match item {
            MenuItem::Separator => current_top += SEPARATOR_HEIGHT,
            _ => current_top += ITEM_HEIGHT,
        }
    }

    let mut main_item_elements = Vec::with_capacity(items.len());

    for (idx, item) in items.iter().enumerate() {
        match item {
            MenuItem::Separator => {
                main_item_elements.push(
                    div()
                        .h(px(1.0))
                        .my(px(4.0))
                        .mx(px(6.0))
                        .bg(style_border)
                        .into_any_element(),
                );
            }
            MenuItem::Action {
                label,
                shortcut,
                action,
            } => {
                let action = action.clone();
                main_item_elements.push(
                    div()
                        .h(px(ITEM_HEIGHT))
                        .mx(px(4.0))
                        .px(px(8.0))
                        .rounded_sm()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .cursor_pointer()
                        .hover(move |s| s.bg(style_item_hover))
                        .on_mouse_move(cx.listener(move |this, _, _window, cx| {
                            if this.active_submenu.is_some() {
                                this.active_submenu = None;
                                cx.notify();
                            }
                        }))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.is_menu_open = false;
                                this.active_submenu = None;
                                action(this, window, cx);
                            }),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(style_title_color)
                                .child(*label),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(style_badge_color)
                                .child(*shortcut),
                        )
                        .into_any_element(),
                );
            }
            MenuItem::Submenu { label, .. } => {
                let is_active = active_submenu == Some(idx);
                main_item_elements.push(
                    div()
                        .h(px(ITEM_HEIGHT))
                        .mx(px(4.0))
                        .px(px(8.0))
                        .rounded_sm()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .cursor_pointer()
                        .when(is_active, |s| s.bg(style_item_hover))
                        .hover(move |s| s.bg(style_item_hover))
                        .on_mouse_move(cx.listener(move |this, _, _window, cx| {
                            if this.active_submenu != Some(idx) {
                                this.active_submenu = Some(idx);
                                cx.notify();
                            }
                        }))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(style_title_color)
                                .child(*label),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(style_badge_color)
                                .child("›"),
                        )
                        .into_any_element(),
                );
            }
        }
    }

    let main_panel = div()
        .absolute()
        .top(px(TITLEBAR_HEIGHT))
        .right(right_offset)
        .w(px(MENU_WIDTH))
        .bg(style_dropdown_bg)
        .border_1()
        .border_color(style_border)
        .rounded_md()
        .shadow_md()
        .py(px(MENU_PADDING_Y))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_, _, _, cx| cx.stop_propagation()),
        )
        .children(main_item_elements);

    let submenu_panel = submenu_to_render.map(|(sub_items, top_pos)| {
        render_flat_panel(
            &sub_items,
            right_offset + px(MENU_WIDTH + 2.0),
            top_pos,
            style,
            cx,
        )
    });

    div()
        .id("zenvi-menu-overlay")
        .absolute()
        .inset_0()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _, _window, cx| {
                cx.stop_propagation();
                this.is_menu_open = false;
                this.active_submenu = None;
                cx.notify();
            }),
        )
        .child(main_panel)
        .children(submenu_panel)
}
