use crate::actions::*;
use gpui::{App, KeyBinding};

pub fn init_keymaps(cx: &mut App) {
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
}
