use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub text: String,
    pub hl_id: u32,
    pub width: usize,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            text: " ".to_string(),
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

#[derive(Debug, Clone)]
pub struct Grid {
    #[allow(dead_code)]
    pub id: u64,
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

impl Grid {
    pub fn new(id: u64, width: usize, height: usize) -> Self {
        let cells = vec![vec![Cell::default(); width]; height];
        Self {
            id,
            width,
            height,
            cells,
            cursor_row: 0,
            cursor_col: 0,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.cells.resize(height, vec![Cell::default(); width]);
        for row in self.cells.iter_mut() {
            row.resize(width, Cell::default());
        }
    }

    pub fn clear(&mut self) {
        for row in self.cells.iter_mut() {
            for cell in row.iter_mut() {
                *cell = Cell::default();
            }
        }
    }

    pub fn scroll(&mut self, top: usize, bot: usize, left: usize, right: usize, rows: i64) {
        let top = top.min(self.height);
        let bot = bot.min(self.height);
        let left = left.min(self.width);
        let right = right.min(self.width);

        if top >= bot || left >= right || rows == 0 {
            return;
        }

        if rows > 0 {
            let count = rows as usize;
            for r in top..(bot.saturating_sub(count)) {
                let src_r = r + count;
                for c in left..right {
                    self.cells[r][c] = self.cells[src_r][c].clone();
                }
            }
            for r in (bot.saturating_sub(count))..bot {
                for c in left..right {
                    self.cells[r][c] = Cell::default();
                }
            }
        } else {
            let count = (-rows) as usize;
            for r in ((top + count)..bot).rev() {
                let src_r = r - count;
                for c in left..right {
                    self.cells[r][c] = self.cells[src_r][c].clone();
                }
            }
            for r in top..(top + count).min(bot) {
                for c in left..right {
                    self.cells[r][c] = Cell::default();
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
        assert_eq!(cell.text, " ");
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
        assert_eq!(grid.cells.len(), 10);
        assert_eq!(grid.cells[0].len(), 40);
        assert_eq!(grid.cursor_row, 0);
        assert_eq!(grid.cursor_col, 0);
    }

    #[test]
    fn test_grid_resize_expand_and_shrink() {
        let mut grid = Grid::new(1, 3, 2);
        grid.cells[0][0] = Cell {
            text: "A".to_string(),
            hl_id: 1,
            width: 1,
        };
        grid.cells[1][2] = Cell {
            text: "B".to_string(),
            hl_id: 2,
            width: 1,
        };

        // Expand
        grid.resize(5, 4);
        assert_eq!(grid.width, 5);
        assert_eq!(grid.height, 4);
        assert_eq!(grid.cells[0][0].text, "A");
        assert_eq!(grid.cells[1][2].text, "B");
        assert_eq!(grid.cells[3][4], Cell::default());

        // Shrink
        grid.resize(2, 1);
        assert_eq!(grid.width, 2);
        assert_eq!(grid.height, 1);
        assert_eq!(grid.cells.len(), 1);
        assert_eq!(grid.cells[0].len(), 2);
        assert_eq!(grid.cells[0][0].text, "A");
    }

    #[test]
    fn test_grid_clear() {
        let mut grid = Grid::new(1, 2, 2);
        grid.cells[0][0] = Cell {
            text: "X".to_string(),
            hl_id: 10,
            width: 1,
        };
        grid.cells[1][1] = Cell {
            text: "Y".to_string(),
            hl_id: 20,
            width: 1,
        };

        grid.clear();
        assert_eq!(grid.cells[0][0], Cell::default());
        assert_eq!(grid.cells[1][1], Cell::default());
    }

    #[test]
    fn test_grid_scroll_up() {
        let mut grid = Grid::new(1, 4, 4);
        // Fill row 0 with '0', row 1 with '1', row 2 with '2', row 3 with '3'
        for r in 0..4 {
            for c in 0..4 {
                grid.cells[r][c] = Cell {
                    text: r.to_string(),
                    hl_id: 0,
                    width: 1,
                };
            }
        }

        // Scroll up by 1 row in region top=0, bot=4, left=0, right=4
        grid.scroll(0, 4, 0, 4, 1);

        // Row 0 should now have '1', Row 1 should have '2', Row 2 should have '3', Row 3 should be default
        assert_eq!(grid.cells[0][0].text, "1");
        assert_eq!(grid.cells[1][0].text, "2");
        assert_eq!(grid.cells[2][0].text, "3");
        assert_eq!(grid.cells[3][0], Cell::default());
    }

    #[test]
    fn test_grid_scroll_down() {
        let mut grid = Grid::new(1, 4, 4);
        for r in 0..4 {
            for c in 0..4 {
                grid.cells[r][c] = Cell {
                    text: r.to_string(),
                    hl_id: 0,
                    width: 1,
                };
            }
        }

        // Scroll down by 1 row (rows = -1) in region top=0, bot=4, left=0, right=4
        grid.scroll(0, 4, 0, 4, -1);

        // Row 0 should be default, Row 1 should have '0', Row 2 should have '1', Row 3 should have '2'
        assert_eq!(grid.cells[0][0], Cell::default());
        assert_eq!(grid.cells[1][0].text, "0");
        assert_eq!(grid.cells[2][0].text, "1");
        assert_eq!(grid.cells[3][0].text, "2");
    }

    #[test]
    fn test_grid_scroll_partial_region() {
        let mut grid = Grid::new(1, 4, 4);
        for r in 0..4 {
            for c in 0..4 {
                grid.cells[r][c] = Cell {
                    text: format!("{}{}", r, c),
                    hl_id: 0,
                    width: 1,
                };
            }
        }

        // Scroll up by 1 row in sub-box: top=1, bot=3, left=1, right=3
        grid.scroll(1, 3, 1, 3, 1);

        // Outside area should remain untouched
        assert_eq!(grid.cells[0][0].text, "00");
        assert_eq!(grid.cells[3][3].text, "33");
        assert_eq!(grid.cells[1][0].text, "10");
        assert_eq!(grid.cells[1][3].text, "13");

        // Sub-box row 1 (cols 1..3) should now have previous row 2 content
        assert_eq!(grid.cells[1][1].text, "21");
        assert_eq!(grid.cells[1][2].text, "22");

        // Sub-box row 2 (cols 1..3) should be default (cleared)
        assert_eq!(grid.cells[2][1], Cell::default());
        assert_eq!(grid.cells[2][2], Cell::default());
    }

    #[test]
    fn test_grid_scroll_boundary_noop() {
        let mut grid = Grid::new(1, 4, 4);
        grid.cells[0][0] = Cell {
            text: "A".to_string(),
            hl_id: 0,
            width: 1,
        };

        // rows == 0
        grid.scroll(0, 4, 0, 4, 0);
        assert_eq!(grid.cells[0][0].text, "A");

        // top >= bot
        grid.scroll(3, 2, 0, 4, 1);
        assert_eq!(grid.cells[0][0].text, "A");

        // left >= right
        grid.scroll(0, 4, 3, 2, 1);
        assert_eq!(grid.cells[0][0].text, "A");

        // Out-of-bounds coordinates handled gracefully without panic
        grid.scroll(0, 100, 0, 100, 1);
        // After scrolling 1 row up on 4x4, row 0 becomes row 1 (which was default)
        assert_eq!(grid.cells[0][0], Cell::default());
    }
}
