use crate::actions::*;
use gpui::{App, Menu, MenuItem as GpuiMenuItem};

#[cfg(not(target_os = "macos"))]
use crate::nvim::state::NvimState;
#[cfg(not(target_os = "macos"))]
use crate::ui::components::menu::{render_cascading_menu, MenuItem, MenuOptions};
#[cfg(not(target_os = "macos"))]
use crate::ui::components::style::derive_titlebar_style;
#[cfg(not(target_os = "macos"))]
use crate::ui::ZenviView;
#[cfg(not(target_os = "macos"))]
use gpui::*;

pub fn init_menus(cx: &mut App) {
    #[cfg(target_os = "macos")]
    {
        cx.set_menus(vec![
            Menu {
                name: "Zenvi".into(),
                items: vec![
                    GpuiMenuItem::action("About Zenvi", About),
                    GpuiMenuItem::separator(),
                    GpuiMenuItem::action("Open Neovim Config", OpenConfig),
                    GpuiMenuItem::separator(),
                    GpuiMenuItem::action("Reload Neovim", ReloadNvim),
                    GpuiMenuItem::action("Install Shell Command", InstallCli),
                    GpuiMenuItem::separator(),
                    GpuiMenuItem::action("Quit Zenvi", Quit),
                ],
            },
            Menu {
                name: "File".into(),
                items: vec![
                    GpuiMenuItem::action("New Window", NewWindow),
                    GpuiMenuItem::separator(),
                    GpuiMenuItem::action("Open File...", OpenFile),
                    GpuiMenuItem::action("Open Folder...", OpenFolder),
                    GpuiMenuItem::separator(),
                    GpuiMenuItem::action("Close Window", CloseBuffer),
                ],
            },
            Menu {
                name: "Edit".into(),
                items: vec![
                    GpuiMenuItem::action("Undo", Undo),
                    GpuiMenuItem::action("Redo", Redo),
                    GpuiMenuItem::separator(),
                    GpuiMenuItem::action("Cut", Cut),
                    GpuiMenuItem::action("Copy", Copy),
                    GpuiMenuItem::action("Paste", Paste),
                    GpuiMenuItem::action("Select All", SelectAll),
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
                    GpuiMenuItem::action("New Window", NewWindow),
                    GpuiMenuItem::separator(),
                    GpuiMenuItem::action("Open File...", OpenFile),
                    GpuiMenuItem::action("Open Folder...", OpenFolder),
                    GpuiMenuItem::action("Open Neovim Config", OpenConfig),
                    GpuiMenuItem::separator(),
                    GpuiMenuItem::action("Reload Neovim", ReloadNvim),
                    GpuiMenuItem::action("Install Shell Command", InstallCli),
                    GpuiMenuItem::separator(),
                    GpuiMenuItem::action("Close Window", CloseBuffer),
                    GpuiMenuItem::separator(),
                    GpuiMenuItem::action("Exit", Quit),
                ],
            },
            Menu {
                name: "Edit".into(),
                items: vec![
                    GpuiMenuItem::action("Undo", Undo),
                    GpuiMenuItem::action("Redo", Redo),
                    GpuiMenuItem::separator(),
                    GpuiMenuItem::action("Cut", Cut),
                    GpuiMenuItem::action("Copy", Copy),
                    GpuiMenuItem::action("Paste", Paste),
                    GpuiMenuItem::action("Select All", SelectAll),
                ],
            },
        ]);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn file_menu_items() -> Vec<MenuItem> {
    vec![
        MenuItem::action("New Window", "Ctrl+Shift+N", |this, _window, cx| {
            crate::window::open_zenvi_window(this.cwd.clone(), Vec::new(), this.borderless, cx);
        }),
        MenuItem::separator(),
        MenuItem::action("Open File...", "Ctrl+Shift+O", |this, _window, cx| {
            this.open_file(cx);
        }),
        MenuItem::action("Open Folder...", "Ctrl+Alt+O", |this, _window, cx| {
            this.open_folder(cx);
        }),
        MenuItem::separator(),
        MenuItem::action("Close Buffer", "Ctrl+Shift+W", |this, _window, cx| {
            this.close_buffer(cx);
        }),
    ]
}

#[cfg(not(target_os = "macos"))]
pub fn edit_menu_items() -> Vec<MenuItem> {
    vec![
        MenuItem::action("Undo", "Ctrl+Shift+Z", |this, _window, cx| {
            this.undo(cx);
        }),
        MenuItem::action("Redo", "Ctrl+Shift+Y", |this, _window, cx| {
            this.redo(cx);
        }),
        MenuItem::separator(),
        MenuItem::action("Cut", "Ctrl+Shift+X", |this, _window, cx| {
            this.cut(cx);
        }),
        MenuItem::action("Copy", "Ctrl+Shift+C", |this, _window, cx| {
            this.copy(cx);
        }),
        MenuItem::action("Paste", "Ctrl+Shift+V", |this, _window, cx| {
            this.paste(cx);
        }),
        MenuItem::action("Select All", "Ctrl+Shift+A", |this, _window, cx| {
            this.select_all(cx);
        }),
    ]
}

/// Builds the hierarchical main menu data (First level: Zenvi actions + File / Edit submenus + Exit)
#[cfg(not(target_os = "macos"))]
pub fn build_main_menu() -> Vec<MenuItem> {
    vec![
        MenuItem::action("About Zenvi", "", |this, _window, cx| {
            this.show_about(cx);
        }),
        MenuItem::separator(),
        MenuItem::action("Open Neovim Config", "Ctrl+Shift+,", |this, _window, cx| {
            let (config_dir, target_file) = crate::window::get_nvim_config_file();
            crate::window::open_zenvi_window(Some(config_dir), vec![target_file], this.borderless, cx);
        }),
        MenuItem::action("Reload Neovim", "Ctrl+Shift+R", |this, _window, cx| {
            this.reload_nvim(cx);
        }),
        MenuItem::action("Install Shell Command", "", |this, _window, cx| {
            this.install_cli(cx);
        }),
        MenuItem::separator(),
        MenuItem::submenu("File", file_menu_items()),
        MenuItem::submenu("Edit", edit_menu_items()),
        MenuItem::separator(),
        MenuItem::action("Exit Zenvi", "Ctrl+Shift+Q", |_this, _window, cx| {
            cx.quit();
        }),
    ]
}

/// Renders the in-window cascading menu overlay for Linux / Windows.
#[cfg(not(target_os = "macos"))]
pub fn render_app_menu(
    state: &NvimState,
    active_submenu: Option<usize>,
    cx: &mut Context<ZenviView>,
) -> impl IntoElement {
    let style = derive_titlebar_style(state.default_bg, state.default_fg);
    let options = MenuOptions::top_left(px(8.0), px(crate::ui::TITLEBAR_HEIGHT));
    render_cascading_menu(build_main_menu(), options, &style, active_submenu, cx)
}
