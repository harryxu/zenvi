use crate::actions::*;
use gpui::{App, KeyBinding};

pub fn init_keymaps(cx: &mut App) {
    #[cfg(target_os = "macos")]
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-w", CloseBuffer, None),
        KeyBinding::new("cmd-W", CloseBuffer, None),
        KeyBinding::new("cmd-shift-n", NewWindow, None),
        KeyBinding::new("cmd-shift-N", NewWindow, None),
        KeyBinding::new("cmd-,", OpenConfig, None),
        KeyBinding::new("cmd-shift-r", ReloadNvim, None),
        KeyBinding::new("cmd-shift-R", ReloadNvim, None),
        KeyBinding::new("cmd-o", OpenFile, None),
        KeyBinding::new("cmd-alt-o", OpenFolder, None),
        KeyBinding::new("cmd-shift-o", OpenFolder, None),
        KeyBinding::new("escape", Escape, None),
        KeyBinding::new("ctrl-[", Escape, None),
    ]);

    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([
        KeyBinding::new("ctrl-q", Quit, None),
        KeyBinding::new("ctrl-w", CloseBuffer, None),
        KeyBinding::new("ctrl-W", CloseBuffer, None),
        KeyBinding::new("ctrl-shift-n", NewWindow, None),
        KeyBinding::new("ctrl-shift-N", NewWindow, None),
        KeyBinding::new("ctrl-,", OpenConfig, None),
        KeyBinding::new("ctrl-shift-r", ReloadNvim, None),
        KeyBinding::new("ctrl-shift-R", ReloadNvim, None),
        KeyBinding::new("ctrl-o", OpenFile, None),
        KeyBinding::new("ctrl-alt-o", OpenFolder, None),
        KeyBinding::new("ctrl-shift-o", OpenFolder, None),
        KeyBinding::new("escape", Escape, None),
        KeyBinding::new("ctrl-[", Escape, None),
    ]);
}
