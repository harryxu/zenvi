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
    let bar_bg = data.bg.unwrap_or(default_bg);
    let mut root = div()
        .id("delicate-statusline")
        .h(height)
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .bg(rgb(bar_bg))
        .overflow_hidden()
        .text_size(font_size);

    if !font_family.is_empty() {
        root = root.font_family(font_family.to_string());
    }

    let render_group = |spans: &[crate::nvim::state::StatuslineSpan], prefix: &'static str| {
        let mut group = div()
            .flex()
            .flex_row()
            .items_center()
            .h_full()
            .flex_shrink_0();

        for (i, span) in spans.iter().enumerate() {
            if span.text.is_empty() {
                continue;
            }

            let fg = span.fg.unwrap_or(default_fg);
            let mut span_el = div()
                .id(ElementId::NamedInteger(prefix.into(), i as u64))
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
            group = group.child(span_el);
        }

        group
    };

    let has_multipart = !data.left_spans.is_empty()
        || !data.center_spans.is_empty()
        || !data.right_spans.is_empty();

    if has_multipart {
        let left = render_group(&data.left_spans, "delicate-left");
        root = root.child(left);

        if !data.center_spans.is_empty() {
            let center = render_group(&data.center_spans, "delicate-center");
            root = root.child(center);
        }

        let right = render_group(&data.right_spans, "delicate-right");
        root = root.child(right);
    } else if !data.spans.is_empty() {
        let all = render_group(&data.spans, "delicate-span");
        root = root.child(all);
    } else if !data.raw_str.is_empty() {
        root = root.child(
            div()
                .px(px(8.0))
                .text_color(rgb(default_fg))
                .child(data.raw_str.clone()),
        );
    }

    root
}
