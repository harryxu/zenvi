use crate::nvim::state::{Grid, NvimState};
use gpui::*;
use std::ops::Range;

/// Cached row data for incremental rendering. When a row is not dirty,
/// we reuse the cached text and highlights instead of re-processing all cells.
#[derive(Clone)]
pub struct CachedRow {
    pub line_text: SharedString,
    pub highlights: Vec<(Range<usize>, HighlightStyle)>,
    pub is_empty: bool,
}

/// Persistent render cache that survives across frames.
/// Only dirty rows (where grid.row_versions[i] != cache.row_versions[i]) are recomputed;
/// clean rows reuse cached StyledText and highlight data.
pub struct GridRenderCache {
    pub rows: Vec<Option<CachedRow>>,
    pub row_versions: Vec<u32>,
    pub last_cursor_row: usize,
    pub last_cursor_col: usize,
}

impl GridRenderCache {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            row_versions: Vec::new(),
            last_cursor_row: usize::MAX,
            last_cursor_col: usize::MAX,
        }
    }

    /// Ensures the cache has enough slots for the given grid height.
    fn ensure_capacity(&mut self, height: usize) {
        if self.rows.len() != height {
            self.rows.resize(height, None);
            self.row_versions.resize(height, 0);
        }
    }

    /// Shifts cached rows in response to full-width grid_scroll events,
    /// preserving pre-parsed strings and highlights for surviving rows.
    pub fn scroll(&mut self, top: usize, bot: usize, rows: i64) {
        let top = top.min(self.rows.len());
        let bot = bot.min(self.rows.len());
        if top >= bot || rows == 0 {
            return;
        }

        if rows > 0 {
            let count = rows as usize;
            if count >= bot - top {
                for r in top..bot {
                    self.rows[r] = None;
                    self.row_versions[r] = 0;
                }
                return;
            }
            let valid_rows = bot - top - count;
            for i in 0..valid_rows {
                self.rows[top + i] = self.rows[top + count + i].take();
            }
            self.row_versions.copy_within((top + count)..bot, top);
            for r in (bot - count)..bot {
                self.rows[r] = None;
                self.row_versions[r] = 0;
            }
        } else {
            let count = (-rows) as usize;
            if count >= bot - top {
                for r in top..bot {
                    self.rows[r] = None;
                    self.row_versions[r] = 0;
                }
                return;
            }
            let valid_rows = bot - top - count;
            for i in (0..valid_rows).rev() {
                self.rows[top + count + i] = self.rows[top + i].take();
            }
            self.row_versions.copy_within(top..(top + valid_rows), top + count);
            for r in top..(top + count) {
                self.rows[r] = None;
                self.row_versions[r] = 0;
            }
        }
    }
}

/// Builds cached row data from grid cells and highlight attributes.
fn build_cached_row(
    row: &[crate::nvim::state::Cell],
    state: &NvimState,
    default_fg: u32,
    default_bg: u32,
) -> CachedRow {
    // Find the rightmost cell that has content or non-default styling
    let last_content_col = row.iter().rposition(|cell| {
        let s = cell.text_str();
        if s != " " && !s.is_empty() {
            return true;
        }
        // Fast-path: hl_id 0 is always default, skip lookup
        if cell.hl_id == 0 {
            return false;
        }
        if let Some(attr) = state.get_highlight(cell.hl_id) {
            let bg = attr.background.unwrap_or(default_bg);
            if bg != default_bg || attr.reverse || attr.underline || attr.undercurl {
                return true;
            }
        }
        false
    });

    let Some(last_col) = last_content_col else {
        return CachedRow {
            line_text: SharedString::default(),
            highlights: Vec::new(),
            is_empty: true,
        };
    };

    let visible_cells = &row[..=last_col];
    let mut line_text = String::with_capacity(visible_cells.len() * 2);
    let mut highlights: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
    let mut current_override: Option<(usize, usize, HighlightStyle)> = None;

    for cell in visible_cells {
        let text = cell.text_str();
        if cell.width == 0 && text.is_empty() {
            continue;
        }

        let start_byte = line_text.len();
        if text.is_empty() {
            line_text.push(' ');
        } else {
            line_text.push_str(text);
        }
        let end_byte = line_text.len();

        let override_style = if cell.hl_id == 0 {
            None
        } else if let Some(attr) = state.get_highlight(cell.hl_id) {
            let mut fg = attr.foreground.unwrap_or(default_fg);
            let mut bg = attr.background.unwrap_or(default_bg);
            if attr.reverse {
                std::mem::swap(&mut fg, &mut bg);
            }

            let is_non_default = fg != default_fg
                || bg != default_bg
                || attr.reverse
                || attr.bold
                || attr.italic
                || attr.underline
                || attr.undercurl;

            if is_non_default {
                let underline_style = if attr.underline || attr.undercurl {
                    Some(UnderlineStyle {
                        color: Some(rgb(fg).into()),
                        thickness: px(1.0),
                        wavy: attr.undercurl,
                    })
                } else {
                    None
                };

                Some(HighlightStyle {
                    color: if fg != default_fg {
                        Some(rgb(fg).into())
                    } else {
                        None
                    },
                    background_color: if bg != default_bg {
                        Some(rgb(bg).into())
                    } else {
                        None
                    },
                    font_weight: if attr.bold {
                        Some(FontWeight::BOLD)
                    } else {
                        None
                    },
                    font_style: if attr.italic {
                        Some(FontStyle::Italic)
                    } else {
                        None
                    },
                    underline: underline_style,
                    ..Default::default()
                })
            } else {
                None
            }
        } else {
            None
        };

        if let Some(style) = override_style {
            if let Some((s_start, s_end, ref s_style)) = current_override {
                if *s_style == style && s_end == start_byte {
                    current_override = Some((s_start, end_byte, style));
                } else {
                    highlights.push((s_start..s_end, s_style.clone()));
                    current_override = Some((start_byte, end_byte, style));
                }
            } else {
                current_override = Some((start_byte, end_byte, style));
            }
        } else {
            if let Some((s_start, s_end, s_style)) = current_override.take() {
                highlights.push((s_start..s_end, s_style));
            }
        }
    }

    if let Some((s_start, s_end, s_style)) = current_override.take() {
        highlights.push((s_start..s_end, s_style));
    }

    CachedRow {
        line_text: SharedString::from(line_text),
        highlights,
        is_empty: false,
    }
}

pub fn render_grid(
    state: &NvimState,
    grid: &Grid,
    font_family: &str,
    font_size: Pixels,
    line_height: Pixels,
    char_width: f32,
    cache: &mut GridRenderCache,
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

    cache.ensure_capacity(grid.height);

    // Drain pending full-width scrolls and shift cached rows in-place
    let pending_scrolls = std::mem::take(&mut *grid.pending_scrolls.lock());
    for s in pending_scrolls {
        if s.left == 0 && s.right == grid.width {
            cache.scroll(s.top, s.bot, s.rows);
        } else {
            // For partial-width scrolls, invalidate affected cached rows
            for r in s.top..s.bot.min(cache.rows.len()) {
                cache.rows[r] = None;
                cache.row_versions[r] = 0;
            }
        }
    }

    let mut row_elements = Vec::with_capacity(grid.height);
    let start_t = std::time::Instant::now();
    let mut dirty_count = 0;
    let mut clean_count = 0;

    for (row_idx, row) in grid.rows().enumerate() {
        let grid_ver = grid.row_versions.get(row_idx).copied().unwrap_or(0);
        let cache_ver = cache.row_versions.get(row_idx).copied().unwrap_or(0);
        let is_dirty = grid_ver != cache_ver || cache.rows.get(row_idx).map_or(true, |c| c.is_none());

        // Rebuild cached data only for dirty rows
        if is_dirty {
            dirty_count += 1;
            let cached = build_cached_row(row, state, default_fg, default_bg);
            if row_idx < cache.rows.len() {
                cache.rows[row_idx] = Some(cached);
                cache.row_versions[row_idx] = grid_ver;
            }
        } else {
            clean_count += 1;
        }

        // Use cached data to build the element
        let cached = cache.rows.get(row_idx).and_then(|c| c.as_ref());

        if let Some(cached_row) = cached {
            if cached_row.is_empty {
                row_elements.push(div().h(line_height).w_full());
            } else {
                row_elements.push(
                    div()
                        .h(line_height)
                        .w_full()
                        .overflow_hidden()
                        .child(
                            StyledText::new(cached_row.line_text.clone())
                                .with_default_highlights(&default_style, cached_row.highlights.clone()),
                        ),
                );
            }
        } else {
            row_elements.push(div().h(line_height).w_full());
        }
    }

    if dirty_count > 0 {
        eprintln!("[GRID_STATS] dirty: {}, clean: {}, build_time: {:?}", dirty_count, clean_count, start_t.elapsed());
    }

    // Floating Cursor Overlay: Decoupled from line text layout to eliminate subpixel text jitter
    let cursor_row = grid.cursor_row;
    let cursor_col = grid.cursor_col;
    let lh_f32: f32 = line_height.into();
    let cursor_x = cursor_col as f32 * char_width;
    let cursor_y = cursor_row as f32 * lh_f32;

    let cell_under_cursor = grid.get_cell(cursor_row, cursor_col);

    let cursor_text = cell_under_cursor
        .map(|c| {
            let s = c.text_str();
            if s.is_empty() {
                " "
            } else {
                s
            }
        })
        .unwrap_or(" ");

    let cursor_cell_width = cell_under_cursor.map(|c| c.width.max(1)).unwrap_or(1);
    let cursor_w = char_width * cursor_cell_width as f32;

    // Use mode_idx for O(1) lookup instead of linear search
    let cursor_shape = state
        .mode_info
        .get(state.current_mode_idx)
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

    // Track cursor position for dirty detection
    cache.last_cursor_row = cursor_row;
    cache.last_cursor_col = cursor_col;

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
