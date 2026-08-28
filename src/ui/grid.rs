use crate::nvim::state::{Grid, NvimState};
use gpui::*;

pub struct CellSpan {
    pub text: String,
    pub fg: u32,
    pub bg: u32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub is_cursor: bool,
}

pub fn render_grid(
    state: &NvimState,
    grid: &Grid,
    font_family: &str,
    font_size: Pixels,
    line_height: Pixels,
) -> impl IntoElement {
    let default_fg = state.default_fg;
    let default_bg = state.default_bg;

    let mut row_elements = Vec::with_capacity(grid.height);

    for (r_idx, row) in grid.cells.iter().enumerate() {
        let is_cursor_row = r_idx == grid.cursor_row;
        let mut spans: Vec<CellSpan> = Vec::new();

        for (c_idx, cell) in row.iter().enumerate() {
            // In Neovim ext_linegrid, a double-width character occupies 2 cells:
            // the first cell contains the character, and the second cell has width == 0 and text == "".
            // Skip the second trailing cell so we don't insert an extra space.
            if cell.width == 0 && cell.text.is_empty() {
                continue;
            }

            let is_cursor = is_cursor_row && c_idx == grid.cursor_col;

            let attr = state
                .highlights
                .get(&cell.hl_id)
                .cloned()
                .unwrap_or_default();

            let mut fg = attr.foreground.unwrap_or(default_fg);
            let mut bg = attr.background.unwrap_or(default_bg);

            if attr.reverse || is_cursor {
                std::mem::swap(&mut fg, &mut bg);
                if is_cursor && bg == default_bg {
                    bg = 0xffffff;
                    fg = 0x000000;
                }
            }

            let text = if cell.text.is_empty() {
                " ".to_string()
            } else {
                cell.text.clone()
            };

            // Try to merge with previous span if attributes match and neither is cursor
            if let Some(last) = spans.last_mut() {
                if !is_cursor
                    && !last.is_cursor
                    && last.fg == fg
                    && last.bg == bg
                    && last.bold == attr.bold
                    && last.italic == attr.italic
                    && last.underline == attr.underline
                {
                    last.text.push_str(&text);
                    continue;
                }
            }

            spans.push(CellSpan {
                text,
                fg,
                bg,
                bold: attr.bold,
                italic: attr.italic,
                underline: attr.underline,
                is_cursor,
            });
        }

        let mut span_elements = Vec::with_capacity(spans.len());
        for span in spans {
            let mut el = div()
                .h(line_height)
                .bg(rgb(span.bg))
                .text_color(rgb(span.fg))
                .child(span.text);

            if span.underline {
                el = el.border_b_1().border_color(rgb(span.fg));
            }

            span_elements.push(el);
        }

        row_elements.push(
            div()
                .flex()
                .flex_row()
                .w_full()
                .h(line_height)
                .children(span_elements),
        );
    }

    div()
        .flex()
        .flex_col()
        .w_full()
        .font_family(font_family.to_string())
        .text_size(font_size)
        .line_height(line_height)
        .bg(rgb(default_bg))
        .children(row_elements)
}
