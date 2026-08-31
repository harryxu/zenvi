use crate::actions::*;
use gpui::{App, Menu, MenuItem};

#[cfg(not(target_os = "macos"))]
use crate::nvim::state::NvimState;
#[cfg(not(target_os = "macos"))]
use crate::ui::components::dropdown::{render_dropdown, DropdownItem};
#[cfg(not(target_os = "macos"))]
use crate::ui::components::style::derive_titlebar_style;
#[cfg(not(target_os = "macos"))]
use crate::ui::ZenviView;
#[cfg(not(target_os = "macos"))]
use gpui::*;

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveMenu {
    File,
    Edit,
}

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
            Menu {
                name: "Edit".into(),
                items: vec![
                    MenuItem::action("Undo", Undo),
                    MenuItem::action("Redo", Redo),
                    MenuItem::separator(),
                    MenuItem::action("Cut", Cut),
                    MenuItem::action("Copy", Copy),
                    MenuItem::action("Paste", Paste),
                    MenuItem::action("Select All", SelectAll),
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
            Menu {
                name: "Edit".into(),
                items: vec![
                    MenuItem::action("Undo", Undo),
                    MenuItem::action("Redo", Redo),
                    MenuItem::separator(),
                    MenuItem::action("Cut", Cut),
                    MenuItem::action("Copy", Copy),
                    MenuItem::action("Paste", Paste),
                    MenuItem::action("Select All", SelectAll),
                ],
            },
        ]);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn file_menu_items() -> Vec<DropdownItem> {
    vec![
        DropdownItem::action("New Window", "Ctrl+Shift+N", |this, _window, cx| {
            crate::window::open_zenvi_window(this.cwd.clone(), Vec::new(), cx);
        }),
        DropdownItem::separator(),
        DropdownItem::action("Open File...", "Ctrl+Shift+O", |this, _window, cx| {
            this.open_file(cx);
        }),
        DropdownItem::action("Open Folder...", "Ctrl+Alt+O", |this, _window, cx| {
            this.open_folder(cx);
        }),
        DropdownItem::action("Open Neovim Config", "Ctrl+Shift+,", |_this, _window, cx| {
            let config_dir = crate::window::get_nvim_config_dir();
            if !config_dir.exists() {
                let _ = std::fs::create_dir_all(&config_dir);
            }
            crate::window::open_zenvi_window(Some(config_dir), Vec::new(), cx);
        }),
        DropdownItem::separator(),
        DropdownItem::action("Reload Neovim", "Ctrl+Shift+R", |this, _window, cx| {
            this.reload_nvim(cx);
        }),
        DropdownItem::action("Install Shell Command", "", |this, _window, cx| {
            this.install_cli(cx);
        }),
        DropdownItem::separator(),
        DropdownItem::action("Close Buffer", "Ctrl+Shift+W", |this, _window, cx| {
            this.close_buffer(cx);
        }),
        DropdownItem::separator(),
        DropdownItem::action("Exit Zenvi", "Ctrl+Shift+Q", |_this, _window, cx| {
            cx.quit();
        }),
    ]
}

#[cfg(not(target_os = "macos"))]
pub fn edit_menu_items() -> Vec<DropdownItem> {
    vec![
        DropdownItem::action("Undo", "Ctrl+Shift+Z", |this, _window, cx| {
            this.undo(cx);
        }),
        DropdownItem::action("Redo", "Ctrl+Shift+Y", |this, _window, cx| {
            this.redo(cx);
        }),
        DropdownItem::separator(),
        DropdownItem::action("Cut", "Ctrl+Shift+X", |this, _window, cx| {
            this.cut(cx);
        }),
        DropdownItem::action("Copy", "Ctrl+Shift+C", |this, _window, cx| {
            this.copy(cx);
        }),
        DropdownItem::action("Paste", "Ctrl+Shift+V", |this, _window, cx| {
            this.paste(cx);
        }),
        DropdownItem::action("Select All", "Ctrl+Shift+A", |this, _window, cx| {
            this.select_all(cx);
        }),
    ]
}

/// Renders the in-window dropdown menu overlay for Linux / Windows.
#[cfg(not(target_os = "macos"))]
pub fn render_menu_dropdown(
    active: ActiveMenu,
    state: &NvimState,
    cx: &mut Context<ZenviView>,
) -> impl IntoElement {
    let style = derive_titlebar_style(state.default_bg, state.default_fg);
    match active {
        ActiveMenu::File => render_dropdown(file_menu_items(), px(12.0), &style, cx),
        ActiveMenu::Edit => render_dropdown(edit_menu_items(), px(56.0), &style, cx),
    }
}
