use super::ZenviView;
use super::TITLEBAR_HEIGHT;
use gpui::*;

/// Pre-computed titlebar colors derived from the Neovim theme.
pub struct TitlebarStyle {
    pub title_color: Rgba,
    pub badge_color: Rgba,
    pub border_color: Rgba,
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

    TitlebarStyle {
        title_color,
        badge_color,
        border_color,
    }
}

/// Builds the custom titlebar element with title text and theme badge.
pub fn render_titlebar(
    title: &str,
    style: &TitlebarStyle,
    default_bg: u32,
    cx: &mut Context<ZenviView>,
) -> Stateful<Div> {
    #[cfg(target_os = "macos")]
    let titlebar_pl = px(78.0);
    #[cfg(not(target_os = "macos"))]
    let titlebar_pl = px(12.0);

    div()
        .id("zenvi-titlebar")
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .pl(titlebar_pl)
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
                        .text_color(style.title_color)
                        .child(title.to_string()),
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
                        .text_color(style.badge_color)
                        .child("⚡️ GPUI"),
                ),
        )
}
