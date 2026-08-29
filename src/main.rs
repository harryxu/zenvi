#![recursion_limit = "512"]

mod actions;
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

        cx.on_action(|_: &OpenConfig, cx: &mut App| {
            let config_dir = window::get_nvim_config_dir();
            if !config_dir.exists() {
                let _ = std::fs::create_dir_all(&config_dir);
            }
            window::open_zenvi_window(Some(config_dir), cx);
        });

        // Initialize keyboard shortcuts & macOS application menus
        keymap::init_keymaps(cx);
        menu::init_menus(cx);

        // Open initial window
        window::open_zenvi_window(None, cx);
    });
}

