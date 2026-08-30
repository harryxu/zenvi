use crate::actions::*;
use gpui::{App, Menu, MenuItem};

pub fn init_menus(cx: &mut App) {
    #[cfg(target_os = "macos")]
    {
        cx.set_menus(vec![
            Menu {
                name: "Zenvi".into(),
                items: vec![
                    MenuItem::action("Open Neovim Config", OpenConfig),
                    MenuItem::separator(),
                    MenuItem::action("Reload Neovim", ReloadNvim),
                    MenuItem::action("Install Shell Command", InstallCli),
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
                    MenuItem::separator(),
                    MenuItem::action("Close Window", CloseBuffer),
                ],
            },
        ]);
    }

    #[cfg(not(target_os = "macos"))]
    {
        cx.set_menus(vec![
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
                    MenuItem::action("Install Shell Command", InstallCli),
                    MenuItem::separator(),
                    MenuItem::action("Close Window", CloseBuffer),
                    MenuItem::separator(),
                    MenuItem::action("Exit", Quit),
                ],
            },
        ]);
    }
}
