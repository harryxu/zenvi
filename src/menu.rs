use crate::actions::*;
use gpui::{App, Menu, MenuItem};

pub fn init_menus(cx: &mut App) {
    cx.set_menus(vec![
        Menu {
            name: "Zenvi".into(),
            items: vec![
                MenuItem::action("Open Neovim Config", OpenConfig),
                MenuItem::separator(),
                MenuItem::action("Reload Neovim", ReloadNvim),
                MenuItem::separator(),
                MenuItem::action("Quit Zenvi", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Window", NewWindow),
                MenuItem::separator(),
                MenuItem::action("Open File...", OpenFile),
                MenuItem::action("Open Folder...", OpenFolder),
                MenuItem::action("Open Neovim Config", OpenConfig),
                MenuItem::separator(),
                MenuItem::action("Reload Neovim", ReloadNvim),
            ],
        },
    ]);
}
