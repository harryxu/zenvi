#![recursion_limit = "512"]

mod input;
mod nvim;
mod ui;

use gpui::*;
use nvim::process::{NvimEvent, NvimSession};
use std::sync::Arc;
use tokio::sync::mpsc;
use ui::ZenviView;

actions!(zenvi, [Quit, OpenFile, OpenFolder, Escape]);

fn main() {
    env_logger::init();

    // Start a multi-threaded tokio runtime for background Neovim IPC
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");

    let _guard = rt.enter();

    Application::new().run(|cx: &mut App| {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<NvimEvent>();

        let session = match NvimSession::spawn(event_tx) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to spawn Neovim process: {:?}", e);
                std::process::exit(1);
            }
        };

        let session_clone = Arc::clone(&session);

        // Register Actions
        cx.on_action(|_: &Quit, cx: &mut App| {
            cx.quit();
        });

        let session_escape = Arc::clone(&session);
        cx.on_action(move |_: &Escape, _cx: &mut App| {
            session_escape.send_input("<Esc>");
        });

        let session_open_file = Arc::clone(&session);
        cx.on_action(move |_: &OpenFile, cx: &mut App| {
            let session = Arc::clone(&session_open_file);
            let receiver = cx.prompt_for_paths(PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some("Open File".into()),
            });
            cx.spawn(|_cx: &mut AsyncApp| async move {
                if let Ok(Ok(Some(paths))) = receiver.await {
                    for path in paths {
                        if let Some(parent) = path.parent() {
                            session.send_command(&format!("cd {}", parent.display()));
                        }
                        session.send_command(&format!("edit {}", path.display()));
                    }
                }
            })
            .detach();
        });

        let session_open_folder = Arc::clone(&session);
        cx.on_action(move |_: &OpenFolder, cx: &mut App| {
            let session = Arc::clone(&session_open_folder);
            let receiver = cx.prompt_for_paths(PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: Some("Open Folder".into()),
            });
            cx.spawn(|_cx: &mut AsyncApp| async move {
                if let Ok(Ok(Some(paths))) = receiver.await {
                    for path in paths {
                        session.send_command(&format!("cd {}", path.display()));
                        session.send_command(&format!("edit {}", path.display()));
                    }
                }
            })
            .detach();
        });

        // Keybindings
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
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
                items: vec![MenuItem::action("Quit Zenvi", Quit)],
            },
            Menu {
                name: "File".into(),
                items: vec![
                    MenuItem::action("Open File...", OpenFile),
                    MenuItem::action("Open Folder...", OpenFolder),
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
                let view = ZenviView::new(session_clone, cx);
                window.focus(&view.focus_handle);
                view
            });

            // Listen for events (redraw / exit) from Neovim and notify the view or quit
            let view_weak = view.downgrade();
            cx.spawn(|cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    while let Some(event) = event_rx.recv().await {
                        match event {
                            NvimEvent::Redraw => {
                                let _ = view_weak.update(&mut cx, |_this, cx| {
                                    cx.notify();
                                });
                            }
                            NvimEvent::Exit => {
                                let _ = cx.update(|cx| {
                                    cx.quit();
                                });
                            }
                        }
                    }
                }
            })
            .detach();

            view
        })
        .expect("Failed to open GPUI window");
    });
}
