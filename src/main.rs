#![recursion_limit = "512"]

mod actions;
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

    Application::new().run(|cx: &mut App| {
        // Register Global App Actions
        cx.on_action(|_: &Quit, cx: &mut App| {
            cx.quit();
        });

        cx.on_action(|_: &NewWindow, cx: &mut App| {
            window::open_zenvi_window(None, Vec::new(), cx);
        });

        cx.on_action(|_: &OpenConfig, cx: &mut App| {
            let config_dir = window::get_nvim_config_dir();
            if !config_dir.exists() {
                let _ = std::fs::create_dir_all(&config_dir);
            }
            window::open_zenvi_window(Some(config_dir), Vec::new(), cx);
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

        // Open initial window with parsed CLI arguments
        let launch_config = window::resolve_cli_launch_config();
        window::open_zenvi_window(launch_config.cwd, launch_config.targets, cx);
    });
}


