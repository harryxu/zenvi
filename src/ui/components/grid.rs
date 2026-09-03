use crate::nvim::state::{Grid, NvimState};
use gpui::*;
use std::collections::HashMap;

/// Inline fast 64-bit FNV-1a hash of a row's cells (text + hl_id).
/// Processes ~100 cells in under 100 nanoseconds.
#[inline]
fn hash_cells(cells: &[crate::nvim::state::Cell]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for cell in cells {
        hash ^= cell.hl_id as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        let s = cell.text_str();
        for &b in s.as_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

/// Pre-created font variants to avoid allocating Font / SharedString per cell.
pub struct FontVariants {
    pub regular: Font,
    pub bold: Font,
    pub italic: Font,
    pub bold_italic: Font,
}

/// Cached pre-shaped row for GPU Canvas direct rendering.
/// Survives across frames and linegrid scrolls.
#[derive(Clone)]
pub struct CachedRow {
    pub shaped_line: ShapedLine,
    pub is_empty: bool,
}

/// Persistent render cache that survives across frames.
/// Only dirty rows (where grid.row_versions[i] != cache.row_versions[i]) are recomputed;
/// clean rows reuse pre-shaped GPU lines.
pub struct GridRenderCache {
    pub rows: Vec<Option<CachedRow>>,
    pub row_versions: Vec<u32>,
    pub last_cursor_row: usize,
    pub last_cursor_col: usize,
    pub cached_font_size: Pixels,
    pub cached_font_family: String,
    /// Content-addressable line cache: stores pre-shaped lines indexed by their cell hash.
    /// Eliminates HarfBuzz font shaping when scrolling past previously seen lines.
    pub content_cache: HashMap<u64, ShapedLine>,
}

impl GridRenderCache {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            row_versions: Vec::new(),
            last_cursor_row: usize::MAX,
            last_cursor_col: usize::MAX,
            cached_font_size: px(0.0),
            cached_font_family: String::new(),
            content_cache: HashMap::with_capacity(1024),
        }
    }

    /// Ensures the cache has enough slots for the given grid height.
    fn ensure_capacity(&mut self, height: usize) {
        if self.rows.len() != height {
            self.rows.resize(height, None);
            self.row_versions.resize(height, 0);
        }
    }

    /// Checks if font configuration changed and resets cache if necessary.
    fn check_font(&mut self, font_family: &str, font_size: Pixels) {
        if self.cached_font_size != font_size || self.cached_font_family != font_family {
            self.rows.clear();
            self.row_versions.clear();
            self.content_cache.clear();
            self.cached_font_size = font_size;
            self.cached_font_family = font_family.to_string();
        }
    }

    /// Shifts cached rows in response to full-width or near full-width grid_scroll events,
    /// preserving pre-shaped GPU lines for surviving rows.
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

/// Builds pre-shaped row data from grid cells and highlight attributes,
/// ready for direct GPU canvas painting.
fn build_cached_row(
    row: &[crate::nvim::state::Cell],
    state: &NvimState,
    _default_style: &TextStyle,
    font_variants: &FontVariants,
    window: &mut Window,
    font_size: Pixels,
    content_cache: &mut HashMap<u64, ShapedLine>,
) -> CachedRow {
    let default_fg = state.default_fg;
    let default_bg = state.default_bg;

    // Find the rightmost cell that has content or non-default styling
    let last_content_col = row.iter().rposition(|cell| {
        let s = cell.text_str();
        if s != " " && !s.is_empty() {
            return true;
        }
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
            shaped_line: ShapedLine::default(),
            is_empty: true,
        };
    };

    let visible_cells = &row[..=last_col];

    // Check content-addressable line cache first
    let line_hash = hash_cells(visible_cells);
    if let Some(shaped) = content_cache.get(&line_hash) {
        return CachedRow {
            shaped_line: shaped.clone(),
            is_empty: false,
        };
    }

    let mut line_text = String::with_capacity(visible_cells.len() * 2);
    let mut runs: Vec<TextRun> = Vec::new();

    for cell in visible_cells {
        let text = cell.text_str();
        if cell.width == 0 && text.is_empty() {
            continue;
        }

        let char_str = if text.is_empty() { " " } else { text };
        let char_bytes = char_str.len();
        line_text.push_str(char_str);

        // Resolve style for this cell
        let mut font = font_variants.regular.clone();
        let mut fg = default_fg;
        let mut bg = default_bg;
        let mut underline = None;

        if cell.hl_id != 0 {
            if let Some(attr) = state.get_highlight(cell.hl_id) {
                fg = attr.foreground.unwrap_or(default_fg);
                bg = attr.background.unwrap_or(default_bg);
                if attr.reverse {
                    std::mem::swap(&mut fg, &mut bg);
                }
                font = match (attr.bold, attr.italic) {
                    (true, true) => font_variants.bold_italic.clone(),
                    (true, false) => font_variants.bold.clone(),
                    (false, true) => font_variants.italic.clone(),
                    (false, false) => font_variants.regular.clone(),
                };
                if attr.underline || attr.undercurl {
                    underline = Some(UnderlineStyle {
                        color: Some(rgb(fg).into()),
                        thickness: px(1.0),
                        wavy: attr.undercurl,
                    });
                }
            }
        }

        let color = rgb(fg).into();
        let background_color = if bg != default_bg {
            Some(rgb(bg).into())
        } else {
            None
        };

        // Try to coalesce with previous run if identical styling
        if let Some(last_run) = runs.last_mut() {
            if last_run.font == font
                && last_run.color == color
                && last_run.background_color == background_color
                && last_run.underline == underline
                && last_run.strikethrough.is_none()
            {
                last_run.len += char_bytes;
                continue;
            }
        }

        runs.push(TextRun {
            len: char_bytes,
            font,
            color,
            background_color,
            underline,
            strikethrough: None,
        });
    }

    let shaped_line = window.text_system().shape_line(
        SharedString::from(line_text),
        font_size,
        &runs,
        None,
    );

    if content_cache.len() >= 4096 {
        content_cache.clear();
    }
    content_cache.insert(line_hash, shaped_line.clone());

    CachedRow {
        shaped_line,
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
    smooth_cursor_pos: Option<(f32, f32)>,
    window: &mut Window,
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

    cache.check_font(font_family, font_size);
    cache.ensure_capacity(grid.height);

    let font_variants = FontVariants {
        regular: default_style.font(),
        bold: default_style.font().bold(),
        italic: default_style.font().italic(),
        bold_italic: default_style.font().bold().italic(),
    };

    // Drain pending scrolls and shift cached pre-shaped rows in-place
    let pending_scrolls = std::mem::take(&mut *grid.pending_scrolls.lock());
    for s in pending_scrolls {
        if s.left == 0 && s.right >= grid.width.saturating_sub(2) {
            cache.scroll(s.top, s.bot, s.rows);
        } else {
            // For other partial-width scrolls, invalidate affected cached rows
            for r in s.top..s.bot.min(cache.rows.len()) {
                cache.rows[r] = None;
                cache.row_versions[r] = 0;
            }
        }
    }

    // Re-shape only dirty rows
    for (row_idx, row) in grid.rows().enumerate() {
        let grid_ver = grid.row_versions.get(row_idx).copied().unwrap_or(0);
        let cache_ver = cache.row_versions.get(row_idx).copied().unwrap_or(0);
        let is_dirty = grid_ver != cache_ver || cache.rows.get(row_idx).map_or(true, |c| c.is_none());

        if is_dirty {
            let cached = build_cached_row(
                row,
                state,
                &default_style,
                &font_variants,
                window,
                font_size,
                &mut cache.content_cache,
            );
            if row_idx < cache.rows.len() {
                cache.rows[row_idx] = Some(cached);
                cache.row_versions[row_idx] = grid_ver;
            }
        }
    }

    // Floating Cursor Overlay
    let cursor_row = grid.cursor_row;
    let cursor_col = grid.cursor_col;
    let lh_f32: f32 = line_height.into();
    let (cursor_x, cursor_y) = smooth_cursor_pos.unwrap_or_else(|| {
        (cursor_col as f32 * char_width, cursor_row as f32 * lh_f32)
    });

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

    cache.last_cursor_row = cursor_row;
    cache.last_cursor_col = cursor_col;

    // Snapshot pre-shaped lines for GPU Canvas drawing
    let cached_rows: Vec<Option<ShapedLine>> = cache
        .rows
        .iter()
        .map(|r| {
            r.as_ref().and_then(|c| {
                if c.is_empty {
                    None
                } else {
                    Some(c.shaped_line.clone())
                }
            })
        })
        .collect();

    div()
        .relative()
        .size_full()
        .overflow_hidden()
        .font_family(font_family.to_string())
        .text_size(font_size)
        .line_height(line_height)
        .child(
            canvas(
                move |_bounds, _window, _cx| (),
                move |bounds, (), window, cx| {
                    // Fill grid background
                    window.paint_quad(fill(bounds, rgb(default_bg)));

                    // Direct GPU draw calls for all pre-shaped rows
                    for (r, maybe_shaped) in cached_rows.into_iter().enumerate() {
                        if let Some(shaped) = maybe_shaped {
                            let origin = Point::new(
                                bounds.left(),
                                bounds.top() + px(r as f32 * lh_f32),
                            );
                            let _ = shaped.paint_background(origin, line_height, window, cx);
                            let _ = shaped.paint(origin, line_height, window, cx);
                        }
                    }
                },
            )
            .size_full(),
        )
        .child(cursor_element)
}
