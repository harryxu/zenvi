#![recursion_limit = "512"]

mod input;
mod nvim;
mod ui;

use gpui::*;
use ui::ZenviView;

actions!(zenvi, [Quit, OpenFile, OpenFolder, Escape, ReloadNvim]);

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

        // Keybindings
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
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
                    MenuItem::separator(),
                    MenuItem::action("Reload Neovim", ReloadNvim),
                ],
            },
        ]);

        let window_size = Size::new(px(1080.0), px(720.0));
        let window_bounds = if let Some(display) = cx.displays().first() {
            let screen = display.bounds();
            let origin = Point::new(
                screen.origin.x + ((screen.size.width - window_size.width) / 2.0).max(px(0.0)),
                screen.origin.y + ((screen.size.height - window_size.height) / 2.0).max(px(0.0)),
            );
            Bounds::new(origin, window_size)
        } else {
            Bounds::new(Point::new(px(100.0), px(100.0)), window_size)
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

            let view = cx.new(|cx| {
                let view = ZenviView::new(cx);
                window.focus(&view.focus_handle);
                view
            });

            view
        })
        .expect("Failed to open GPUI window");
    });
}
