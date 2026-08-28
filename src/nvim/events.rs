use crate::nvim::state::{Cell, HighlightAttr, ModeInfo, NvimState};
use rmpv::Value;

pub fn handle_redraw_event(state: &mut NvimState, event: &[Value]) {
    if event.is_empty() {
        return;
    }

    let name = match event[0].as_str() {
        Some(n) => n,
        None => return,
    };

    let calls = &event[1..];
    for call_val in calls {
        let args = match call_val.as_array() {
            Some(a) => a,
            None => continue,
        };

        match name {
            "grid_resize" => {
                if args.len() >= 3 {
                    let grid_id = args[0].as_u64().unwrap_or(1);
                    let width = args[1].as_u64().unwrap_or(80) as usize;
                    let height = args[2].as_u64().unwrap_or(24) as usize;

                    state
                        .grids
                        .entry(grid_id)
                        .and_modify(|g| g.resize(width, height))
                        .or_insert_with(|| crate::nvim::state::Grid::new(grid_id, width, height));
                }
            }
            "default_colors_set" => {
                if args.len() >= 3 {
                    if let Some(fg) = args[0].as_i64() {
                        if fg >= 0 {
                            state.default_fg = fg as u32;
                        }
                    }
                    if let Some(bg) = args[1].as_i64() {
                        if bg >= 0 {
                            state.default_bg = bg as u32;
                        }
                    }
                    if let Some(sp) = args[2].as_i64() {
                        if sp >= 0 {
                            state.default_sp = sp as u32;
                        }
                    }
                }
            }
            "hl_attr_define" => {
                if args.len() >= 2 {
                    let id = args[0].as_u64().unwrap_or(0) as u32;
                    let mut attr = HighlightAttr::default();

                    if let Some(map) = args[1].as_map() {
                        for (k, v) in map {
                            if let Some(key) = k.as_str() {
                                match key {
                                    "foreground" => {
                                        if let Some(c) = v.as_i64() {
                                            attr.foreground = Some(c as u32);
                                        }
                                    }
                                    "background" => {
                                        if let Some(c) = v.as_i64() {
                                            attr.background = Some(c as u32);
                                        }
                                    }
                                    "special" => {
                                        if let Some(c) = v.as_i64() {
                                            attr.special = Some(c as u32);
                                        }
                                    }
                                    "reverse" => attr.reverse = v.as_bool().unwrap_or(false),
                                    "italic" => attr.italic = v.as_bool().unwrap_or(false),
                                    "bold" => attr.bold = v.as_bool().unwrap_or(false),
                                    "underline" => attr.underline = v.as_bool().unwrap_or(false),
                                    "undercurl" => attr.undercurl = v.as_bool().unwrap_or(false),
                                    "strikethrough" => {
                                        attr.strikethrough = v.as_bool().unwrap_or(false)
                                    }
                                    "blend" => {
                                        attr.blend = v.as_u64().unwrap_or(0) as u8;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    state.highlights.insert(id, attr);
                }
            }
            "grid_line" => {
                if args.len() >= 4 {
                    let grid_id = args[0].as_u64().unwrap_or(1);
                    let row = args[1].as_u64().unwrap_or(0) as usize;
                    let mut col_start = args[2].as_u64().unwrap_or(0) as usize;
                    let cells = match args[3].as_array() {
                        Some(c) => c,
                        None => continue,
                    };

                    if let Some(grid) = state.grids.get_mut(&grid_id) {
                        if row >= grid.height {
                            continue;
                        }

                        let mut current_hl = 0;
                        for cell_val in cells {
                            if let Some(cell_info) = cell_val.as_array() {
                                if cell_info.is_empty() {
                                    continue;
                                }

                                let text = cell_info[0].as_str().unwrap_or(" ").to_string();
                                if cell_info.len() >= 2 {
                                    current_hl = cell_info[1].as_u64().unwrap_or(0) as u32;
                                }
                                let repeat = if cell_info.len() >= 3 {
                                    cell_info[2].as_u64().unwrap_or(1) as usize
                                } else {
                                    1
                                };

                                let char_width = unicode_width::UnicodeWidthStr::width(text.as_str());
                                let cell = Cell {
                                    text: text.clone(),
                                    hl_id: current_hl,
                                    width: if char_width == 0 { 1 } else { char_width },
                                };

                                for _ in 0..repeat {
                                    if col_start < grid.width {
                                        grid.cells[row][col_start] = cell.clone();
                                        col_start += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "grid_cursor_goto" => {
                if args.len() >= 3 {
                    let grid_id = args[0].as_u64().unwrap_or(1);
                    let row = args[1].as_u64().unwrap_or(0) as usize;
                    let col = args[2].as_u64().unwrap_or(0) as usize;

                    state.active_grid = grid_id;
                    if let Some(grid) = state.grids.get_mut(&grid_id) {
                        grid.cursor_row = row;
                        grid.cursor_col = col;
                    }
                }
            }
            "grid_scroll" => {
                if args.len() >= 7 {
                    let grid_id = args[0].as_u64().unwrap_or(1);
                    let top = args[1].as_u64().unwrap_or(0) as usize;
                    let bot = args[2].as_u64().unwrap_or(0) as usize;
                    let left = args[3].as_u64().unwrap_or(0) as usize;
                    let right = args[4].as_u64().unwrap_or(0) as usize;
                    let rows = args[5].as_i64().unwrap_or(0);

                    if let Some(grid) = state.grids.get_mut(&grid_id) {
                        grid.scroll(top, bot, left, right, rows);
                    }
                }
            }
            "grid_clear" => {
                let grid_id = args.get(0).and_then(|v| v.as_u64()).unwrap_or(1);
                if let Some(grid) = state.grids.get_mut(&grid_id) {
                    grid.clear();
                }
            }
            "grid_destroy" => {
                let grid_id = args.get(0).and_then(|v| v.as_u64()).unwrap_or(1);
                state.grids.remove(&grid_id);
            }
            "mode_info_set" => {
                if args.len() >= 2 {
                    if let Some(modes) = args[1].as_array() {
                        let mut info_list = Vec::new();
                        for m in modes {
                            if let Some(map) = m.as_map() {
                                let mut info = ModeInfo::default();
                                for (k, v) in map {
                                    if let Some(key) = k.as_str() {
                                        match key {
                                            "name" => {
                                                if let Some(s) = v.as_str() {
                                                    info.name = s.to_string();
                                                }
                                            }
                                            "cursor_shape" => {
                                                if let Some(s) = v.as_str() {
                                                    info.cursor_shape = s.to_string();
                                                }
                                            }
                                            "cell_percentage" => {
                                                info.cell_percentage = v.as_u64().unwrap_or(100);
                                            }
                                            "blinkwait" => {
                                                info.blinkwait = v.as_u64().unwrap_or(0);
                                            }
                                            "blinkon" => {
                                                info.blinkon = v.as_u64().unwrap_or(0);
                                            }
                                            "blinkoff" => {
                                                info.blinkoff = v.as_u64().unwrap_or(0);
                                            }
                                            "hl_id" => {
                                                info.hl_id = v.as_u64().unwrap_or(0) as u32;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                info_list.push(info);
                            }
                        }
                        state.mode_info = info_list;
                    }
                }
            }
            "mode_change" => {
                if args.len() >= 2 {
                    if let Some(name) = args[0].as_str() {
                        state.current_mode = name.to_string();
                    }
                    state.current_mode_idx = args[1].as_u64().unwrap_or(0) as usize;
                }
            }
            "set_title" => {
                if let Some(title) = args.get(0).and_then(|v| v.as_str()) {
                    state.title = title.to_string();
                }
            }
            "flush" => {
                // Redraw batch completed
            }
            _ => {}
        }
    }
}
