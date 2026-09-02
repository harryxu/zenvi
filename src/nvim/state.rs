use std::collections::HashMap;

/// Inline compact UTF-8 text storage for terminal cells (up to 15 bytes),
/// eliminating all heap allocations per cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmallText {
    buf: [u8; 15],
    len: u8,
}

impl SmallText {
    #[allow(dead_code)]
    pub const fn empty() -> Self {
        Self {
            buf: [0; 15],
            len: 0,
        }
    }

    pub fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let len = bytes.len().min(15);
        let mut buf = [0u8; 15];
        buf[..len].copy_from_slice(&bytes[..len]);
        Self {
            buf,
            len: len as u8,
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        match std::str::from_utf8(&self.buf[..self.len as usize]) {
            Ok(s) => s,
            Err(_) => "",
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for SmallText {
    fn default() -> Self {
        Self::from_str(" ")
    }
}

/// A lightweight, copyable grid cell stored completely inline on the stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub text: SmallText,
    pub hl_id: u32,
    pub width: usize,
}

impl Cell {
    #[inline]
    pub fn new(text: &str, hl_id: u32, width: usize) -> Self {
        Self {
            text: SmallText::from_str(text),
            hl_id,
            width,
        }
    }

    #[inline]
    pub fn text_str(&self) -> &str {
        self.text.as_str()
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            text: SmallText::from_str(" "),
            hl_id: 0,
            width: 1,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HighlightAttr {
    pub foreground: Option<u32>,
    pub background: Option<u32>,
    pub special: Option<u32>,
    pub reverse: bool,
    pub italic: bool,
    pub bold: bool,
    pub underline: bool,
    pub undercurl: bool,
    pub strikethrough: bool,
    pub blend: u8,
}

/// Contiguous flat grid storage enabling zero-copy SIMD memmove row scrolling.
#[derive(Debug, Clone)]
pub struct Grid {
    #[allow(dead_code)]
    pub id: u64,
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Cell>,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

impl Grid {
    pub fn new(id: u64, width: usize, height: usize) -> Self {
        let count = width * height;
        let cells = vec![Cell::default(); count];
        Self {
            id,
            width,
            height,
            cells,
            cursor_row: 0,
            cursor_col: 0,
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn row(&self, r: usize) -> &[Cell] {
        if r < self.height && self.width > 0 {
            let start = r * self.width;
            let end = start + self.width;
            &self.cells[start..end]
        } else {
            &[]
        }
    }

    #[inline]
    pub fn row_mut(&mut self, r: usize) -> &mut [Cell] {
        if r < self.height && self.width > 0 {
            let start = r * self.width;
            let end = start + self.width;
            &mut self.cells[start..end]
        } else {
            &mut []
        }
    }

    #[inline]
    pub fn rows(&self) -> impl Iterator<Item = &[Cell]> {
        if self.width == 0 {
            self.cells.chunks(1)
        } else {
            self.cells.chunks(self.width)
        }
    }

    #[inline]
    pub fn get_cell(&self, r: usize, c: usize) -> Option<&Cell> {
        if r < self.height && c < self.width {
            Some(&self.cells[r * self.width + c])
        } else {
            None
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn set_cell(&mut self, r: usize, c: usize, cell: Cell) {
        if r < self.height && c < self.width {
            self.cells[r * self.width + c] = cell;
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        if self.width == width && self.height == height {
            return;
        }
        let mut new_cells = vec![Cell::default(); width * height];
        let copy_rows = self.height.min(height);
        let copy_cols = self.width.min(width);

        for r in 0..copy_rows {
            let src_start = r * self.width;
            let dst_start = r * width;
            new_cells[dst_start..dst_start + copy_cols]
                .copy_from_slice(&self.cells[src_start..src_start + copy_cols]);
        }

        self.width = width;
        self.height = height;
        self.cells = new_cells;
    }

    pub fn clear(&mut self) {
        self.cells.fill(Cell::default());
    }

    pub fn scroll(&mut self, top: usize, bot: usize, left: usize, right: usize, rows: i64) {
        let top = top.min(self.height);
        let bot = bot.min(self.height);
        let left = left.min(self.width);
        let right = right.min(self.width);

        if top >= bot || left >= right || rows == 0 || self.width == 0 {
            return;
        }

        let is_full_width = left == 0 && right == self.width;

        if rows > 0 {
            let count = rows as usize;
            if count >= bot - top {
                // Clear entire region
                if is_full_width {
                    self.cells[top * self.width..bot * self.width].fill(Cell::default());
                } else {
                    for r in top..bot {
                        let start = r * self.width + left;
                        self.cells[start..start + (right - left)].fill(Cell::default());
                    }
                }
                return;
            }

            let valid_rows = bot - top - count;
            if is_full_width {
                let src_start = (top + count) * self.width;
                let src_end = bot * self.width;
                let dst_start = top * self.width;
                self.cells.copy_within(src_start..src_end, dst_start);
                // Clear bottom vacated rows
                let clear_start = (bot - count) * self.width;
                let clear_end = bot * self.width;
                self.cells[clear_start..clear_end].fill(Cell::default());
            } else {
                let len = right - left;
                for r in top..(top + valid_rows) {
                    let src_r = r + count;
                    let src_start = src_r * self.width + left;
                    let dst_start = r * self.width + left;
                    self.cells.copy_within(src_start..src_start + len, dst_start);
                }
                for r in (bot - count)..bot {
                    let start = r * self.width + left;
                    self.cells[start..start + len].fill(Cell::default());
                }
            }
        } else {
            let count = (-rows) as usize;
            if count >= bot - top {
                // Clear entire region
                if is_full_width {
                    self.cells[top * self.width..bot * self.width].fill(Cell::default());
                } else {
                    for r in top..bot {
                        let start = r * self.width + left;
                        self.cells[start..start + (right - left)].fill(Cell::default());
                    }
                }
                return;
            }

            let valid_rows = bot - top - count;
            if is_full_width {
                let src_start = top * self.width;
                let src_end = (top + valid_rows) * self.width;
                let dst_start = (top + count) * self.width;
                self.cells.copy_within(src_start..src_end, dst_start);
                // Clear top vacated rows
                let clear_start = top * self.width;
                let clear_end = (top + count) * self.width;
                self.cells[clear_start..clear_end].fill(Cell::default());
            } else {
                let len = right - left;
                for r in (top..top + valid_rows).rev() {
                    let src_r = r;
                    let dst_r = r + count;
                    let src_start = src_r * self.width + left;
                    let dst_start = dst_r * self.width + left;
                    self.cells.copy_within(src_start..src_start + len, dst_start);
                }
                for r in top..(top + count) {
                    let start = r * self.width + left;
                    self.cells[start..start + len].fill(Cell::default());
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModeInfo {
    pub name: String,
    pub cursor_shape: String, // "block", "horizontal", "vertical"
    pub cell_percentage: u64,
    pub blinkwait: u64,
    pub blinkon: u64,
    pub blinkoff: u64,
    pub hl_id: u32,
}

impl Default for ModeInfo {
    fn default() -> Self {
        Self {
            name: "normal".to_string(),
            cursor_shape: "block".to_string(),
            cell_percentage: 100,
            blinkwait: 0,
            blinkon: 0,
            blinkoff: 0,
            hl_id: 0,
        }
    }
}

#[derive(Debug)]
pub struct NvimState {
    pub default_fg: u32,
    pub default_bg: u32,
    pub default_sp: u32,
    pub highlights: HashMap<u32, HighlightAttr>,
    pub grids: HashMap<u64, Grid>,
    pub active_grid: u64,
    pub current_mode: String,
    pub mode_info: Vec<ModeInfo>,
    pub current_mode_idx: usize,
    pub title: String,
    pub guifont: String,
    pub linespace: i64,
}

impl Default for NvimState {
    fn default() -> Self {
        let mut grids = HashMap::new();
        grids.insert(1, Grid::new(1, 80, 24));
        Self {
            default_fg: 0xcccccc,
            default_bg: 0x1e1e1e,
            default_sp: 0xff0000,
            highlights: HashMap::new(),
            grids,
            active_grid: 1,
            current_mode: "normal".to_string(),
            mode_info: Vec::new(),
            current_mode_idx: 0,
            title: "Zenvi".to_string(),
            guifont: String::new(),
            linespace: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_default() {
        let cell = Cell::default();
        assert_eq!(cell.text_str(), " ");
        assert_eq!(cell.hl_id, 0);
        assert_eq!(cell.width, 1);
    }

    #[test]
    fn test_highlight_attr_default() {
        let hl = HighlightAttr::default();
        assert_eq!(hl.foreground, None);
        assert_eq!(hl.background, None);
        assert_eq!(hl.special, None);
        assert!(!hl.bold);
        assert!(!hl.italic);
        assert!(!hl.reverse);
        assert!(!hl.underline);
        assert!(!hl.undercurl);
        assert!(!hl.strikethrough);
        assert_eq!(hl.blend, 0);
    }

    #[test]
    fn test_mode_info_default() {
        let mode = ModeInfo::default();
        assert_eq!(mode.name, "normal");
        assert_eq!(mode.cursor_shape, "block");
        assert_eq!(mode.cell_percentage, 100);
        assert_eq!(mode.hl_id, 0);
    }

    #[test]
    fn test_nvim_state_default() {
        let state = NvimState::default();
        assert_eq!(state.default_bg, 0x1e1e1e);
        assert_eq!(state.default_fg, 0xcccccc);
        assert_eq!(state.current_mode, "normal");
        assert_eq!(state.active_grid, 1);
        assert!(state.grids.contains_key(&1));
        let grid = state.grids.get(&1).unwrap();
        assert_eq!(grid.width, 80);
        assert_eq!(grid.height, 24);
    }

    #[test]
    fn test_grid_new() {
        let grid = Grid::new(2, 40, 10);
        assert_eq!(grid.id, 2);
        assert_eq!(grid.width, 40);
        assert_eq!(grid.height, 10);
        assert_eq!(grid.cells.len(), 400);
        assert_eq!(grid.row(0).len(), 40);
        assert_eq!(grid.cursor_row, 0);
        assert_eq!(grid.cursor_col, 0);
    }

    #[test]
    fn test_grid_resize_expand_and_shrink() {
        let mut grid = Grid::new(1, 3, 2);
        grid.set_cell(0, 0, Cell::new("A", 1, 1));
        grid.set_cell(1, 2, Cell::new("B", 2, 1));

        // Expand
        grid.resize(5, 4);
        assert_eq!(grid.width, 5);
        assert_eq!(grid.height, 4);
        assert_eq!(grid.get_cell(0, 0).unwrap().text_str(), "A");
        assert_eq!(grid.get_cell(1, 2).unwrap().text_str(), "B");
        assert_eq!(grid.get_cell(3, 4).unwrap(), &Cell::default());

        // Shrink
        grid.resize(2, 1);
        assert_eq!(grid.width, 2);
        assert_eq!(grid.height, 1);
        assert_eq!(grid.cells.len(), 2);
        assert_eq!(grid.get_cell(0, 0).unwrap().text_str(), "A");
    }

    #[test]
    fn test_grid_clear() {
        let mut grid = Grid::new(1, 2, 2);
        grid.set_cell(0, 0, Cell::new("X", 10, 1));
        grid.set_cell(1, 1, Cell::new("Y", 20, 1));

        grid.clear();
        assert_eq!(grid.get_cell(0, 0).unwrap(), &Cell::default());
        assert_eq!(grid.get_cell(1, 1).unwrap(), &Cell::default());
    }

    #[test]
    fn test_grid_scroll_up() {
        let mut grid = Grid::new(1, 4, 4);
        // Fill row 0 with '0', row 1 with '1', row 2 with '2', row 3 with '3'
        for r in 0..4 {
            for c in 0..4 {
                grid.set_cell(r, c, Cell::new(&r.to_string(), 0, 1));
            }
        }

        // Scroll up by 1 row in region top=0, bot=4, left=0, right=4
        grid.scroll(0, 4, 0, 4, 1);

        // Row 0 should now have '1', Row 1 should have '2', Row 2 should have '3', Row 3 should be default
        assert_eq!(grid.get_cell(0, 0).unwrap().text_str(), "1");
        assert_eq!(grid.get_cell(1, 0).unwrap().text_str(), "2");
        assert_eq!(grid.get_cell(2, 0).unwrap().text_str(), "3");
        assert_eq!(grid.get_cell(3, 0).unwrap(), &Cell::default());
    }

    #[test]
    fn test_grid_scroll_down() {
        let mut grid = Grid::new(1, 4, 4);
        for r in 0..4 {
            for c in 0..4 {
                grid.set_cell(r, c, Cell::new(&r.to_string(), 0, 1));
            }
        }

        // Scroll down by 1 row (rows = -1) in region top=0, bot=4, left=0, right=4
        grid.scroll(0, 4, 0, 4, -1);

        // Row 0 should be default, Row 1 should have '0', Row 2 should have '1', Row 3 should have '2'
        assert_eq!(grid.get_cell(0, 0).unwrap(), &Cell::default());
        assert_eq!(grid.get_cell(1, 0).unwrap().text_str(), "0");
        assert_eq!(grid.get_cell(2, 0).unwrap().text_str(), "1");
        assert_eq!(grid.get_cell(3, 0).unwrap().text_str(), "2");
    }

    #[test]
    fn test_grid_scroll_partial_region() {
        let mut grid = Grid::new(1, 4, 4);
        for r in 0..4 {
            for c in 0..4 {
                grid.set_cell(r, c, Cell::new(&format!("{}{}", r, c), 0, 1));
            }
        }

        // Scroll up by 1 row in sub-box: top=1, bot=3, left=1, right=3
        grid.scroll(1, 3, 1, 3, 1);

        // Outside area should remain untouched
        assert_eq!(grid.get_cell(0, 0).unwrap().text_str(), "00");
        assert_eq!(grid.get_cell(3, 3).unwrap().text_str(), "33");
        assert_eq!(grid.get_cell(1, 0).unwrap().text_str(), "10");
        assert_eq!(grid.get_cell(1, 3).unwrap().text_str(), "13");

        // Sub-box row 1 (cols 1..3) should now have previous row 2 content
        assert_eq!(grid.get_cell(1, 1).unwrap().text_str(), "21");
        assert_eq!(grid.get_cell(1, 2).unwrap().text_str(), "22");

        // Sub-box row 2 (cols 1..3) should be default (cleared)
        assert_eq!(grid.get_cell(2, 1).unwrap(), &Cell::default());
        assert_eq!(grid.get_cell(2, 2).unwrap(), &Cell::default());
    }

    #[test]
    fn test_grid_scroll_boundary_noop() {
        let mut grid = Grid::new(1, 4, 4);
        grid.set_cell(0, 0, Cell::new("A", 0, 1));

        // rows == 0
        grid.scroll(0, 4, 0, 4, 0);
        assert_eq!(grid.get_cell(0, 0).unwrap().text_str(), "A");

        // top >= bot
        grid.scroll(3, 2, 0, 4, 1);
        assert_eq!(grid.get_cell(0, 0).unwrap().text_str(), "A");

        // left >= right
        grid.scroll(0, 4, 3, 2, 1);
        assert_eq!(grid.get_cell(0, 0).unwrap().text_str(), "A");

        // Out-of-bounds coordinates handled gracefully without panic
        grid.scroll(0, 100, 0, 100, 1);
        // After scrolling 1 row up on 4x4, row 0 becomes row 1 (which was default)
        assert_eq!(grid.get_cell(0, 0).unwrap(), &Cell::default());
    }
}
