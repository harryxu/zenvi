#![recursion_limit = "4096"]

mod actions;
mod assets;
mod cli;
mod input;
mod keymap;
mod menu;
mod nvim;
mod ui;
mod window;

pub use actions::*;

use gpui::*;

fn main() {
    env_logger::init();

    // Start a multi-threaded tokio runtime for background Neovim IPC
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");

    let _guard = rt.enter();

    let (open_urls_tx, mut open_urls_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<String>>();

    let app = Application::new().with_assets(assets::Assets);
    let tx = open_urls_tx.clone();
    app.on_open_urls(move |urls: Vec<String>| {
        let _ = tx.send(urls);
    });

    app.run(move |cx: &mut App| {
        // Register Global App Actions
        cx.on_action(|_: &Quit, cx: &mut App| {
            cx.quit();
        });

        cx.on_action(|_: &NewWindow, cx: &mut App| {
            window::open_zenvi_window(None, Vec::new(), false, cx);
        });

        cx.on_action(|_: &OpenConfig, cx: &mut App| {
            let (config_dir, target_file) = window::get_nvim_config_file();
            window::open_zenvi_window(Some(config_dir), vec![target_file], false, cx);
        });

        cx.on_action(|_: &InstallCli, _cx: &mut App| {
            match cli::install_shell_command() {
                Ok(symlink_path) => {
                    log::info!("Shell command successfully installed to {}", symlink_path.display());
                }
                Err(e) => {
                    log::error!("Failed to install shell command: {:?}", e);
                }
            }
        });

        // Initialize keyboard shortcuts & macOS application menus
        keymap::init_keymaps(cx);
        menu::init_menus(cx);

        // Handle external URLs/files dropped on Dock icon or opened via Finder
        cx.spawn(|cx: &mut AsyncApp| {
            let cx = cx.clone();
            async move {
                while let Some(urls) = open_urls_rx.recv().await {
                    let paths: Vec<std::path::PathBuf> = urls
                        .into_iter()
                        .filter_map(|u| window::url_to_path(&u))
                        .collect();

                    if !paths.is_empty() {
                        let _ = cx.update(|cx| {
                            let active_window = cx.active_window().or_else(|| cx.windows().first().copied());
                            if let Some(handle) = active_window {
                                let _ = handle.update(cx, |view: AnyView, _window, cx| {
                                    if let Ok(zenvi_view) = view.downcast::<ui::ZenviView>() {
                                        zenvi_view.update(cx, |this, _cx| {
                                            this.open_paths(&paths);
                                        });
                                    }
                                });
                            } else {
                                let cwd = if paths[0].is_dir() {
                                    Some(paths[0].clone())
                                } else {
                                    paths[0].parent().map(|p| p.to_path_buf())
                                };
                                window::open_zenvi_window(cwd, paths, false, cx);
                            }
                        });
                    }
                }
            }
        })
        .detach();

        // Open initial window with parsed CLI arguments
        let launch_config = window::resolve_cli_launch_config();
        window::open_zenvi_window(launch_config.cwd, launch_config.targets, launch_config.borderless, cx);
    });
}


