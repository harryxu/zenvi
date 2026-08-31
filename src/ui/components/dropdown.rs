use super::style::TitlebarStyle;
use crate::ui::{ZenviView, TITLEBAR_HEIGHT};
use gpui::prelude::*;
use gpui::*;

pub type MenuAction = Box<dyn Fn(&mut ZenviView, &mut Window, &mut Context<ZenviView>) + 'static>;

pub struct DropdownItem {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub is_separator: bool,
    pub action: Option<MenuAction>,
}

impl DropdownItem {
    pub fn action<F>(label: &'static str, shortcut: &'static str, f: F) -> Self
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

    pub fn separator() -> Self {
        Self {
            label: "",
            shortcut: "",
            is_separator: true,
            action: None,
        }
    }
}

pub fn render_dropdown(
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
