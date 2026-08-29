#![recursion_limit = "512"]

mod input;
mod nvim;
mod ui;

use gpui::*;
use std::path::PathBuf;
use ui::ZenviView;

actions!(zenvi, [Quit, OpenFile, OpenFolder, OpenConfig, Escape, ReloadNvim]);

fn get_nvim_config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(xdg).join("nvim");
        if p.exists() {
            return p;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".config").join("nvim");
        return p;
    }
    PathBuf::from(".config/nvim")
}

fn open_zenvi_window(cwd: Option<PathBuf>, cx: &mut App) {
    let window_size = Size::new(px(1080.0), px(720.0));
    let window_count = cx.windows().len();
    let offset = px((window_count as f32 % 10.0) * 28.0);

    let window_bounds = if let Some(display) = cx.displays().first() {
        let screen = display.bounds();
        let origin = Point::new(
            (screen.origin.x + ((screen.size.width - window_size.width) / 2.0).max(px(0.0))) + offset,
            (screen.origin.y + ((screen.size.height - window_size.height) / 2.0).max(px(0.0))) + offset,
        );
        Bounds::new(origin, window_size)
    } else {
        Bounds::new(Point::new(px(100.0) + offset, px(100.0) + offset), window_size)
    };

    let mut window_options = WindowOptions::default();
    window_options.window_bounds = Some(WindowBounds::Windowed(window_bounds));
    window_options.focus = true;
    window_options.show = true;
    window_options.titlebar = Some(TitlebarOptions {
        title: Some("Zenvi".into()),
        appears_transparent: true,
        traffic_light_position: Some(Point::new(px(12.0), px(10.0))),
    });

    cx.open_window(window_options, |window, cx| {
        cx.activate(true);

        let window_handle = window.window_handle();
        let view = cx.new(|cx| {
            let view = match cwd {
                Some(dir) => ZenviView::with_cwd(window_handle, Some(dir), cx),
                None => ZenviView::new(window_handle, cx),
            };
            window.focus(&view.focus_handle);
            view
        });

        view
    })
    .expect("Failed to open GPUI window");
}

fn main() {
    env_logger::init();

    // Start a multi-threaded tokio runtime for background Neovim IPC
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");

    let _guard = rt.enter();

    Application::new().run(|cx: &mut App| {
        // Register Global App Actions
        cx.on_action(|_: &Quit, cx: &mut App| {
            cx.quit();
        });

        cx.on_action(|_: &OpenConfig, cx: &mut App| {
            let config_dir = get_nvim_config_dir();
            if !config_dir.exists() {
                let _ = std::fs::create_dir_all(&config_dir);
            }
            open_zenvi_window(Some(config_dir), cx);
        });

        // Keybindings
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-,", OpenConfig, None),
            KeyBinding::new("cmd-shift-r", ReloadNvim, None),
            KeyBinding::new("cmd-shift-R", ReloadNvim, None),
            KeyBinding::new("cmd-o", OpenFile, None),
            KeyBinding::new("cmd-alt-o", OpenFolder, None),
            KeyBinding::new("cmd-shift-o", OpenFolder, None),
            KeyBinding::new("escape", Escape, None),
            KeyBinding::new("ctrl-[", Escape, None),
        ]);

        // macOS Application Menus
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
                    MenuItem::action("Open File...", OpenFile),
                    MenuItem::action("Open Folder...", OpenFolder),
                    MenuItem::action("Open Neovim Config", OpenConfig),
                    MenuItem::separator(),
                    MenuItem::action("Reload Neovim", ReloadNvim),
                ],
            },
        ]);

        open_zenvi_window(None, cx);
    });
}
