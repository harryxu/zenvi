use crate::nvim::state::{Grid, NvimState};
use gpui::*;
use std::ops::Range;

pub fn render_grid(
    state: &NvimState,
    grid: &Grid,
    font_family: &str,
    font_size: Pixels,
    line_height: Pixels,
    char_width: f32,
) -> impl IntoElement {
    let default_fg = state.default_fg;
    let default_bg = state.default_bg;

    let default_style = TextStyle {
        font_family: font_family.to_string().into(),
        font_size: font_size.into(),
        line_height: line_height.into(),
        color: rgb(default_fg).into(),
        white_space: WhiteSpace::Nowrap,
        ..Default::default()
    };

    let mut row_elements = Vec::with_capacity(grid.height);

    for row in grid.cells.iter() {
        // Find the rightmost cell that has content or non-default styling
        let last_content_col = row.iter().rposition(|cell| {
            if cell.text != " " && !cell.text.is_empty() {
                return true;
            }
            if let Some(attr) = state.highlights.get(&cell.hl_id) {
                let bg = attr.background.unwrap_or(default_bg);
                if bg != default_bg || attr.reverse || attr.underline || attr.undercurl {
                    return true;
                }
            }
            false
        });

        let Some(last_col) = last_content_col else {
            // Entire line is empty with default background: zero text shaping required
            row_elements.push(div().h(line_height).w_full());
            continue;
        };

        let visible_cells = &row[..=last_col];
        let mut line_text = String::with_capacity(visible_cells.len());
        let mut highlights: Vec<(Range<usize>, HighlightStyle)> = Vec::new();

        for cell in visible_cells {
            // In Neovim ext_linegrid, a double-width character occupies 2 cells:
            // the first cell contains the character, and the second cell has width == 0 and text == "".
            // Skip the second trailing cell so we don't insert an extra space.
            if cell.width == 0 && cell.text.is_empty() {
                continue;
            }

            let start_byte = line_text.len();
            if cell.text.is_empty() {
                line_text.push(' ');
            } else {
                line_text.push_str(&cell.text);
            }
            let end_byte = line_text.len();

            let attr = state.highlights.get(&cell.hl_id);

            let mut fg = attr.and_then(|a| a.foreground).unwrap_or(default_fg);
            let mut bg = attr.and_then(|a| a.background).unwrap_or(default_bg);
            let reverse = attr.map(|a| a.reverse).unwrap_or(false);
            let underline = attr.map(|a| a.underline).unwrap_or(false);
            let bold = attr.map(|a| a.bold).unwrap_or(false);
            let italic = attr.map(|a| a.italic).unwrap_or(false);

            if reverse {
                std::mem::swap(&mut fg, &mut bg);
            }

            let underline_style = if underline {
                Some(UnderlineStyle {
                    color: Some(rgb(fg).into()),
                    thickness: px(1.0),
                    wavy: false,
                })
            } else {
                None
            };

            let style = HighlightStyle {
                color: Some(rgb(fg).into()),
                background_color: if bg != default_bg {
                    Some(rgb(bg).into())
                } else {
                    None
                },
                font_weight: if bold {
                    Some(FontWeight::BOLD)
                } else {
                    None
                },
                font_style: if italic {
                    Some(FontStyle::Italic)
                } else {
                    None
                },
                underline: underline_style,
                ..Default::default()
            };

            // Merge adjacent spans with identical highlight style
            if let Some((last_range, last_style)) = highlights.last_mut() {
                if *last_style == style && last_range.end == start_byte {
                    last_range.end = end_byte;
                    continue;
                }
            }

            highlights.push((start_byte..end_byte, style));
        }

        row_elements.push(
            div()
                .h(line_height)
                .w_full()
                .overflow_hidden()
                .font_family(font_family.to_string())
                .text_size(font_size)
                .line_height(line_height)
                .child(StyledText::new(line_text).with_default_highlights(&default_style, highlights)),
        );
    }

    // Floating Cursor Overlay: Decoupled from line text layout to eliminate subpixel text jitter
    let cursor_row = grid.cursor_row;
    let cursor_col = grid.cursor_col;
    let lh_f32: f32 = line_height.into();
    let cursor_x = cursor_col as f32 * char_width;
    let cursor_y = cursor_row as f32 * lh_f32;

    let cell_under_cursor = grid
        .cells
        .get(cursor_row)
        .and_then(|row| row.get(cursor_col));

    let cursor_text = cell_under_cursor
        .map(|c| if c.text.is_empty() { " " } else { c.text.as_str() })
        .unwrap_or(" ");

    let cursor_cell_width = cell_under_cursor.map(|c| c.width.max(1)).unwrap_or(1);
    let cursor_w = char_width * cursor_cell_width as f32;

    let cursor_shape = state
        .mode_info
        .iter()
        .find(|m| m.name.eq_ignore_ascii_case(&state.current_mode))
        .map(|m| m.cursor_shape.as_str())
        .unwrap_or("block");

    let cursor_element = match cursor_shape {
        "vertical" => div()
            .absolute()
            .top(px(cursor_y))
            .left(px(cursor_x))
            .w(px(2.0))
            .h(line_height)
            .bg(rgb(default_fg)),
        "horizontal" => div()
            .absolute()
            .top(px(cursor_y + lh_f32 - 2.0))
            .left(px(cursor_x))
            .w(px(cursor_w))
            .h(px(2.0))
            .bg(rgb(default_fg)),
        _ => {
            let cursor_bg = default_fg;
            let cursor_fg = default_bg;

            div()
                .absolute()
                .top(px(cursor_y))
                .left(px(cursor_x))
                .w(px(cursor_w))
                .h(line_height)
                .bg(rgb(cursor_bg))
                .text_color(rgb(cursor_fg))
                .font_family(font_family.to_string())
                .text_size(font_size)
                .line_height(line_height)
                .child(cursor_text.to_string())
        }
    };

    div()
        .relative()
        .flex()
        .flex_col()
        .w_full()
        .font_family(font_family.to_string())
        .text_size(font_size)
        .line_height(line_height)
        .children(row_elements)
        .child(cursor_element)
}
