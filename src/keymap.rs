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
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-V", Paste, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-C", Copy, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-X", Cut, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-A", SelectAll, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("cmd-shift-Z", Redo, None),
        KeyBinding::new("escape", Escape, None),
        KeyBinding::new("ctrl-[", Escape, None),
    ]);

    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([
        KeyBinding::new("ctrl-shift-q", Quit, None),
        KeyBinding::new("alt-f4", Quit, None),
        KeyBinding::new("ctrl-shift-w", CloseBuffer, None),
        KeyBinding::new("ctrl-shift-W", CloseBuffer, None),
        KeyBinding::new("ctrl-shift-n", NewWindow, None),
        KeyBinding::new("ctrl-shift-N", NewWindow, None),
        KeyBinding::new("ctrl-shift-,", OpenConfig, None),
        KeyBinding::new("ctrl-shift-r", ReloadNvim, None),
        KeyBinding::new("ctrl-shift-R", ReloadNvim, None),
        KeyBinding::new("ctrl-shift-o", OpenFile, None),
        KeyBinding::new("ctrl-alt-o", OpenFolder, None),
        KeyBinding::new("ctrl-shift-v", Paste, None),
        KeyBinding::new("ctrl-shift-c", Copy, None),
        KeyBinding::new("ctrl-shift-x", Cut, None),
        KeyBinding::new("ctrl-shift-a", SelectAll, None),
        KeyBinding::new("ctrl-shift-z", Redo, None),
    ]);
}
