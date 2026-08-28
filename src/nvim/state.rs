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
        }
    }
}
