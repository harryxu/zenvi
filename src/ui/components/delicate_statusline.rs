//! # Delicate Statusline Component
//!
//! Renders an independent, high-fidelity statusbar at the bottom of the editor window.
//! Receives real-time evaluated statusline spans from Neovim, supporting custom fonts,
//! independent font sizing (defaulting to 16px), and accurate highlight colors.

use crate::nvim::state::StatuslineData;
use gpui::prelude::*;
use gpui::*;

/// Renders the delicate statusline component with styled segments.
pub fn render_delicate_statusline(
    data: &StatuslineData,
    default_fg: u32,
    default_bg: u32,
    font_family: &str,
    font_size: Pixels,
    height: Pixels,
) -> impl IntoElement {
    let mut root = div()
        .id("delicate-statusline")
        .h(height)
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .bg(rgb(default_bg))
        .overflow_hidden()
        .text_size(font_size);

    if !font_family.is_empty() {
        root = root.font_family(font_family.to_string());
    }

    if data.spans.is_empty() {
        if !data.raw_str.is_empty() {
            root = root.child(
                div()
                    .px(px(8.0))
                    .text_color(rgb(default_fg))
                    .child(data.raw_str.clone()),
            );
        }
        return root;
    }

    for (i, span) in data.spans.iter().enumerate() {
        if span.text.is_empty() {
            continue;
        }

        let fg = span.fg.unwrap_or(default_fg);
        let mut span_el = div()
            .id(ElementId::NamedInteger("delicate-span".into(), i as u64))
            .text_color(rgb(fg))
            .flex_shrink_0()
            .h_full()
            .flex()
            .items_center();

        if let Some(bg) = span.bg {
            span_el = span_el.bg(rgb(bg));
        }

        if span.bold {
            span_el = span_el.font_weight(FontWeight::BOLD);
        }
        if span.italic {
            span_el = span_el.italic();
        }
        if span.underline {
            span_el = span_el.underline();
        }

        span_el = span_el.child(span.text.clone());
        root = root.child(span_el);
    }

    root
}
