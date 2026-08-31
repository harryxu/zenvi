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

/// Specifies the anchor position for the root menu panel
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuAnchor {
    TopLeft { left: Pixels, top: Pixels },
    TopRight { right: Pixels, top: Pixels },
}

/// Specifies the direction in which submenus should fly out
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SubmenuDirection {
    Right,
    Left,
}

/// Configuration options for the cascading menu component
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct MenuOptions {
    pub anchor: MenuAnchor,
    pub submenu_direction: SubmenuDirection,
    pub width: Pixels,
    pub gap: Pixels,
}

const DEFAULT_MENU_WIDTH: f32 = 220.0;
const DEFAULT_MENU_GAP: f32 = 2.0;
const ITEM_HEIGHT: f32 = 26.0;
const SEPARATOR_HEIGHT: f32 = 9.0;
const MENU_PADDING_Y: f32 = 4.0;

impl Default for MenuOptions {
    fn default() -> Self {
        Self {
            anchor: MenuAnchor::TopLeft {
                left: px(8.0),
                top: px(TITLEBAR_HEIGHT),
            },
            submenu_direction: SubmenuDirection::Right,
            width: px(DEFAULT_MENU_WIDTH),
            gap: px(DEFAULT_MENU_GAP),
        }
    }
}

#[allow(dead_code)]
impl MenuOptions {
    /// Creates options for a menu anchored to the top-left, expanding submenus to the right
    pub fn top_left(left: Pixels, top: Pixels) -> Self {
        Self {
            anchor: MenuAnchor::TopLeft { left, top },
            submenu_direction: SubmenuDirection::Right,
            ..Default::default()
        }
    }

    /// Creates options for a menu anchored to the top-right, expanding submenus to the left
    pub fn top_right(right: Pixels, top: Pixels) -> Self {
        Self {
            anchor: MenuAnchor::TopRight { right, top },
            submenu_direction: SubmenuDirection::Left,
            ..Default::default()
        }
    }

    pub fn with_width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn with_submenu_direction(mut self, direction: SubmenuDirection) -> Self {
        self.submenu_direction = direction;
        self
    }

    pub fn with_gap(mut self, gap: Pixels) -> Self {
        self.gap = gap;
        self
    }
}

/// Renders a flat submenu panel
pub fn render_flat_panel(
    items: &[MenuItem],
    anchor: MenuAnchor,
    width: Pixels,
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

    let panel = div()
        .absolute()
        .w(width)
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
        .children(item_elements);

    match anchor {
        MenuAnchor::TopLeft { left, top } => panel.top(top).left(left),
        MenuAnchor::TopRight { right, top } => panel.top(top).right(right),
    }
}

/// Renders the root application menu with cascading submenus based on configurable MenuOptions
pub fn render_cascading_menu(
    items: Vec<MenuItem>,
    options: MenuOptions,
    style: &TitlebarStyle,
    active_submenu: Option<usize>,
    cx: &mut Context<ZenviView>,
) -> impl IntoElement {
    let style_border = style.border_color;
    let style_dropdown_bg = style.dropdown_bg;
    let style_item_hover = style.menu_hover_bg;
    let style_title_color = style.title_color;
    let style_badge_color = style.badge_color;

    let top_base = match options.anchor {
        MenuAnchor::TopLeft { top, .. } | MenuAnchor::TopRight { top, .. } => top,
    };

    // Track active submenu and its top vertical position
    let mut submenu_to_render: Option<(Vec<MenuItem>, Pixels)> = None;
    let mut current_top = MENU_PADDING_Y;

    for (idx, item) in items.iter().enumerate() {
        if let MenuItem::Submenu { items: sub_items, .. } = item {
            if active_submenu == Some(idx) {
                submenu_to_render = Some((sub_items.clone(), top_base + px(current_top)));
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
                let arrow_symbol = match options.submenu_direction {
                    SubmenuDirection::Right => "›",
                    SubmenuDirection::Left => "‹",
                };

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
                                .child(arrow_symbol),
                        )
                        .into_any_element(),
                );
            }
        }
    }

    let main_panel_div = div()
        .absolute()
        .w(options.width)
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

    let main_panel = match options.anchor {
        MenuAnchor::TopLeft { left, top } => main_panel_div.top(top).left(left),
        MenuAnchor::TopRight { right, top } => main_panel_div.top(top).right(right),
    };

    // Calculate submenu anchor position based on options
    let submenu_panel = submenu_to_render.map(|(sub_items, top_pos)| {
        let submenu_anchor = match options.anchor {
            MenuAnchor::TopLeft { left, .. } => match options.submenu_direction {
                SubmenuDirection::Right => MenuAnchor::TopLeft {
                    left: left + options.width + options.gap,
                    top: top_pos,
                },
                SubmenuDirection::Left => MenuAnchor::TopLeft {
                    left: (left - options.width - options.gap).max(px(0.0)),
                    top: top_pos,
                },
            },
            MenuAnchor::TopRight { right, .. } => match options.submenu_direction {
                SubmenuDirection::Left => MenuAnchor::TopRight {
                    right: right + options.width + options.gap,
                    top: top_pos,
                },
                SubmenuDirection::Right => MenuAnchor::TopRight {
                    right: (right - options.width - options.gap).max(px(0.0)),
                    top: top_pos,
                },
            },
        };

        render_flat_panel(&sub_items, submenu_anchor, options.width, style, cx)
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
