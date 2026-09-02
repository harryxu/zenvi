use crate::nvim::state::{Cell, HighlightAttr, ModeInfo, NvimState};
use rmpv::Value;

pub fn handle_redraw_event(state: &mut NvimState, event: &[Value]) -> bool {
    if event.is_empty() {
        return false;
    }

    let name = match event[0].as_str() {
        Some(n) => n,
        None => return false,
    };
    // println!("REDRAW: {}", name);

    let mut is_flush = false;
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

                                let text = cell_info[0].as_str().unwrap_or("");
                                if cell_info.len() >= 2 {
                                    current_hl = cell_info[1].as_u64().unwrap_or(0) as u32;
                                }
                                let repeat = if cell_info.len() >= 3 {
                                    cell_info[2].as_u64().unwrap_or(1) as usize
                                } else {
                                    1
                                };

                                let char_width = unicode_width::UnicodeWidthStr::width(text);
                                let cell = Cell::new(text, current_hl, char_width);

                                let row_slice = grid.row_mut(row);
                                for _ in 0..repeat {
                                    if col_start < row_slice.len() {
                                        row_slice[col_start] = cell;
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
                if args.len() >= 6 {
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
            "option_set" => {
                if args.len() >= 2 {
                    if let Some(opt_name) = args[0].as_str() {
                        match opt_name {
                            "guifont" => {
                                if let Some(val) = args[1].as_str() {
                                    state.guifont = val.to_string();
                                }
                            }
                            "linespace" => {
                                if let Some(val) = args[1].as_i64() {
                                    state.linespace = val;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            "flush" => {
                is_flush = true;
            }
            _ => {}
        }
    }
    is_flush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_redraw_empty() {
        let mut state = NvimState::default();
        handle_redraw_event(&mut state, &[]);
        assert_eq!(state.current_mode, "normal");

        handle_redraw_event(&mut state, &[Value::from(123)]);
        assert_eq!(state.current_mode, "normal");
    }

    #[test]
    fn test_handle_grid_resize() {
        let mut state = NvimState::default();

        // Resize existing grid 1 to 100x40
        let event = vec![
            Value::from("grid_resize"),
            Value::Array(vec![
                Value::from(1u64),
                Value::from(100u64),
                Value::from(40u64),
            ]),
        ];
        handle_redraw_event(&mut state, &event);

        let grid1 = state.grids.get(&1).expect("Grid 1 should exist");
        assert_eq!(grid1.width, 100);
        assert_eq!(grid1.height, 40);

        // Create new grid 2 of size 50x20
        let event2 = vec![
            Value::from("grid_resize"),
            Value::Array(vec![
                Value::from(2u64),
                Value::from(50u64),
                Value::from(20u64),
            ]),
        ];
        handle_redraw_event(&mut state, &event2);

        let grid2 = state.grids.get(&2).expect("Grid 2 should exist");
        assert_eq!(grid2.width, 50);
        assert_eq!(grid2.height, 20);
    }

    #[test]
    fn test_handle_default_colors_set() {
        let mut state = NvimState::default();
        let event = vec![
            Value::from("default_colors_set"),
            Value::Array(vec![
                Value::from(0xffffffi64),
                Value::from(0x000000i64),
                Value::from(0xff0000i64),
            ]),
        ];
        handle_redraw_event(&mut state, &event);

        assert_eq!(state.default_fg, 0xffffff);
        assert_eq!(state.default_bg, 0x000000);
        assert_eq!(state.default_sp, 0xff0000);
    }

    #[test]
    fn test_handle_hl_attr_define() {
        let mut state = NvimState::default();
        let mut map = Vec::new();
        map.push((Value::from("foreground"), Value::from(0x112233i64)));
        map.push((Value::from("background"), Value::from(0x445566i64)));
        map.push((Value::from("special"), Value::from(0x778899i64)));
        map.push((Value::from("bold"), Value::from(true)));
        map.push((Value::from("italic"), Value::from(true)));
        map.push((Value::from("underline"), Value::from(true)));
        map.push((Value::from("undercurl"), Value::from(true)));
        map.push((Value::from("reverse"), Value::from(true)));
        map.push((Value::from("strikethrough"), Value::from(true)));
        map.push((Value::from("blend"), Value::from(50u64)));

        let event = vec![
            Value::from("hl_attr_define"),
            Value::Array(vec![
                Value::from(10u64),
                Value::Map(map),
                Value::Map(vec![]),
                Value::Array(vec![]),
            ]),
        ];
        handle_redraw_event(&mut state, &event);

        let hl = state.highlights.get(&10).expect("Highlight 10 should exist");
        assert_eq!(hl.foreground, Some(0x112233));
        assert_eq!(hl.background, Some(0x445566));
        assert_eq!(hl.special, Some(0x778899));
        assert!(hl.bold);
        assert!(hl.italic);
        assert!(hl.underline);
        assert!(hl.undercurl);
        assert!(hl.reverse);
        assert!(hl.strikethrough);
        assert_eq!(hl.blend, 50);
    }

    #[test]
    fn test_handle_grid_line_ascii_and_repeat() {
        let mut state = NvimState::default();

        let cells = vec![
            // text, hl_id, repeat
            Value::Array(vec![Value::from("A"), Value::from(5u64), Value::from(3u64)]),
            Value::Array(vec![Value::from("B"), Value::from(6u64)]),
        ];

        let event = vec![
            Value::from("grid_line"),
            Value::Array(vec![
                Value::from(1u64), // grid id
                Value::from(0u64), // row
                Value::from(0u64), // col_start
                Value::Array(cells),
            ]),
        ];
        handle_redraw_event(&mut state, &event);

        let grid = state.grids.get(&1).unwrap();
        // Cols 0, 1, 2 should be "A" with hl_id 5
        for col in 0..3 {
            assert_eq!(grid.row(0)[col].text_str(), "A");
            assert_eq!(grid.row(0)[col].hl_id, 5);
            assert_eq!(grid.row(0)[col].width, 1);
        }
        // Col 3 should be "B" with hl_id 6
        assert_eq!(grid.row(0)[3].text_str(), "B");
        assert_eq!(grid.row(0)[3].hl_id, 6);
        assert_eq!(grid.row(0)[3].width, 1);
    }

    #[test]
    fn test_handle_grid_line_cjk_double_width() {
        let mut state = NvimState::default();

        let cells = vec![
            Value::Array(vec![Value::from("中"), Value::from(1u64)]),
            Value::Array(vec![Value::from(""), Value::from(1u64)]),
        ];

        let event = vec![
            Value::from("grid_line"),
            Value::Array(vec![
                Value::from(1u64),
                Value::from(1u64), // row 1
                Value::from(0u64), // col 0
                Value::Array(cells),
            ]),
        ];
        handle_redraw_event(&mut state, &event);

        let grid = state.grids.get(&1).unwrap();
        assert_eq!(grid.row(1)[0].text_str(), "中");
        assert_eq!(grid.row(1)[0].width, 2);
        assert_eq!(grid.row(1)[1].text_str(), "");
        assert_eq!(grid.row(1)[1].width, 0);
    }

    #[test]
    fn test_handle_grid_cursor_goto() {
        let mut state = NvimState::default();
        let event = vec![
            Value::from("grid_cursor_goto"),
            Value::Array(vec![
                Value::from(1u64),
                Value::from(15u64), // row
                Value::from(30u64), // col
            ]),
        ];
        handle_redraw_event(&mut state, &event);

        assert_eq!(state.active_grid, 1);
        let grid = state.grids.get(&1).unwrap();
        assert_eq!(grid.cursor_row, 15);
        assert_eq!(grid.cursor_col, 30);
    }

    #[test]
    fn test_handle_grid_scroll() {
        let mut state = NvimState::default();
        if let Some(grid) = state.grids.get_mut(&1) {
            grid.set_cell(0, 0, Cell::new("Top", 0, 1));
            grid.set_cell(1, 0, Cell::new("Second", 0, 1));
        }

        let event = vec![
            Value::from("grid_scroll"),
            Value::Array(vec![
                Value::from(1u64),
                Value::from(0u64),  // top
                Value::from(24u64), // bot
                Value::from(0u64),  // left
                Value::from(80u64), // right
                Value::from(1i64),  // rows
                Value::from(0i64),  // cols
            ]),
        ];
        handle_redraw_event(&mut state, &event);

        let grid = state.grids.get(&1).unwrap();
        assert_eq!(grid.get_cell(0, 0).unwrap().text_str(), "Second");
    }

    #[test]
    fn test_handle_grid_clear_and_destroy() {
        let mut state = NvimState::default();
        if let Some(grid) = state.grids.get_mut(&1) {
            grid.set_cell(0, 0, Cell::new("Hello", 0, 1));
        }

        // grid_clear
        let clear_event = vec![
            Value::from("grid_clear"),
            Value::Array(vec![Value::from(1u64)]),
        ];
        handle_redraw_event(&mut state, &clear_event);
        assert_eq!(state.grids.get(&1).unwrap().get_cell(0, 0).unwrap().text_str(), " ");

        // grid_destroy
        let destroy_event = vec![
            Value::from("grid_destroy"),
            Value::Array(vec![Value::from(1u64)]),
        ];
        handle_redraw_event(&mut state, &destroy_event);
        assert!(!state.grids.contains_key(&1));
    }

    #[test]
    fn test_handle_mode_info_set_and_change() {
        let mut state = NvimState::default();

        let mut mode_normal = Vec::new();
        mode_normal.push((Value::from("name"), Value::from("normal")));
        mode_normal.push((Value::from("cursor_shape"), Value::from("block")));
        mode_normal.push((Value::from("cell_percentage"), Value::from(100u64)));

        let mut mode_insert = Vec::new();
        mode_insert.push((Value::from("name"), Value::from("insert")));
        mode_insert.push((Value::from("cursor_shape"), Value::from("vertical")));
        mode_insert.push((Value::from("cell_percentage"), Value::from(25u64)));
        mode_insert.push((Value::from("blinkon"), Value::from(500u64)));

        let mode_info_event = vec![
            Value::from("mode_info_set"),
            Value::Array(vec![
                Value::from(true),
                Value::Array(vec![Value::Map(mode_normal), Value::Map(mode_insert)]),
            ]),
        ];
        handle_redraw_event(&mut state, &mode_info_event);

        assert_eq!(state.mode_info.len(), 2);
        assert_eq!(state.mode_info[1].name, "insert");
        assert_eq!(state.mode_info[1].cursor_shape, "vertical");
        assert_eq!(state.mode_info[1].blinkon, 500);

        // mode_change
        let mode_change_event = vec![
            Value::from("mode_change"),
            Value::Array(vec![Value::from("insert"), Value::from(1u64)]),
        ];
        handle_redraw_event(&mut state, &mode_change_event);

        assert_eq!(state.current_mode, "insert");
        assert_eq!(state.current_mode_idx, 1);
    }

    #[test]
    fn test_handle_title_and_options() {
        let mut state = NvimState::default();

        // set_title
        let title_event = vec![
            Value::from("set_title"),
            Value::Array(vec![Value::from("Zenvi - main.rs")]),
        ];
        handle_redraw_event(&mut state, &title_event);
        assert_eq!(state.title, "Zenvi - main.rs");

        // option_set: guifont
        let font_event = vec![
            Value::from("option_set"),
            Value::Array(vec![
                Value::from("guifont"),
                Value::from("JetBrainsMono Nerd Font:h15"),
            ]),
        ];
        handle_redraw_event(&mut state, &font_event);
        assert_eq!(state.guifont, "JetBrainsMono Nerd Font:h15");

        // option_set: linespace
        let linespace_event = vec![
            Value::from("option_set"),
            Value::Array(vec![Value::from("linespace"), Value::from(4i64)]),
        ];
        handle_redraw_event(&mut state, &linespace_event);
        assert_eq!(state.linespace, 4);
    }
}
