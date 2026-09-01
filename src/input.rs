use gpui::KeyDownEvent;

pub fn key_event_to_nvim(event: &KeyDownEvent) -> Option<String> {
    let raw_key = event.keystroke.key.as_str();
    let mods = &event.keystroke.modifiers;

    let ctrl = mods.control;
    let alt = mods.alt;
    let shift = mods.shift;
    let cmd = mods.platform;

    // Do not forward macOS system menu shortcuts (Cmd+Key) to Neovim
    #[cfg(target_os = "macos")]
    if cmd {
        match raw_key {
            "q" | "Q" | "w" | "W" | "o" | "O" | "r" | "R" | "n" | "N" | "," | "v" | "V"
            | "c" | "C" | "x" | "X" | "a" | "A" | "z" | "Z" => return None,
            _ => {}
        }
    }

    // Do not forward Linux/Windows system shortcuts (Ctrl+Shift+Key, Ctrl+Alt+O, Alt+F4) to Neovim
    #[cfg(not(target_os = "macos"))]
    {
        if alt && (raw_key == "F4" || raw_key == "f4") {
            return None;
        }
        if ctrl && alt && (raw_key == "o" || raw_key == "O") {
            return None;
        }
        if ctrl && shift {
            match raw_key {
                "q" | "Q" | "w" | "W" | "o" | "O" | "r" | "R" | "n" | "N" | "," | "v" | "V"
                | "c" | "C" | "x" | "X" | "a" | "A" | "z" | "Z" => return None,
                _ => {}
            }
        }
    }

    let key_lower = raw_key.to_lowercase();
    let key = key_lower.as_str();

    // Ignore standalone modifier key press/release events
    match key {
        "shift"
        | "shift_l"
        | "shift_r"
        | "control"
        | "ctrl"
        | "control_l"
        | "control_r"
        | "alt"
        | "alt_l"
        | "alt_r"
        | "meta"
        | "meta_l"
        | "meta_r"
        | "super"
        | "super_l"
        | "super_r"
        | "command"
        | "cmd"
        | "platform"
        | "capslock"
        | "caps_lock"
        | "numlock"
        | "num_lock"
        | "scrolllock"
        | "scroll_lock"
        | "fn"
        | "function"
        | "iso_level3_shift"
        | "mode_switch" => return None,
        _ => {}
    }

    // Map special key names to Neovim equivalents
    let special_name = match key {
        "enter" | "return" | "\r" | "\n" => Some("CR"),
        "escape" | "esc" | "\x1b" => Some("Esc"),
        "backspace" | "bs" | "\x08" | "\u{7f}" => Some("BS"),
        "tab" | "\t" => Some("Tab"),
        "space" => Some("Space"),
        "up" => Some("Up"),
        "down" => Some("Down"),
        "left" => Some("Left"),
        "right" => Some("Right"),
        "pageup" | "page_up" => Some("PageUp"),
        "pagedown" | "page_down" => Some("PageDown"),
        "home" => Some("Home"),
        "end" => Some("End"),
        "insert" => Some("Insert"),
        "delete" | "del" => Some("Del"),
        "f1" => Some("F1"),
        "f2" => Some("F2"),
        "f3" => Some("F3"),
        "f4" => Some("F4"),
        "f5" => Some("F5"),
        "f6" => Some("F6"),
        "f7" => Some("F7"),
        "f8" => Some("F8"),
        "f9" => Some("F9"),
        "f10" => Some("F10"),
        "f11" => Some("F11"),
        "f12" => Some("F12"),
        "<" => Some("lt"),
        _ => None,
    };

    if let Some(name) = special_name {
        let mut prefix = String::new();
        if ctrl {
            prefix.push_str("C-");
        }
        if alt {
            prefix.push_str("M-");
        }
        if cmd {
            prefix.push_str("D-");
        }
        if shift && (key != "space" && key != "<") {
            prefix.push_str("S-");
        }
        return Some(format!("<{}{}>", prefix, name));
    }

    // Single character with modifiers (Ctrl, Alt, Cmd)
    if ctrl || alt || cmd {
        let mut prefix = String::new();
        if ctrl {
            prefix.push_str("C-");
        }
        if alt {
            prefix.push_str("M-");
        }
        if cmd {
            prefix.push_str("D-");
        }
        if shift {
            prefix.push_str("S-");
        }
        return Some(format!("<{}{}>", prefix, raw_key));
    }

    // Plain characters without modifiers (e.g. 'a', 'A', '1', ':', '/', '<', etc.)
    if raw_key == "<" {
        Some("<lt>".to_string())
    } else if shift && raw_key.len() == 1 && raw_key.chars().all(|c| c.is_ascii_lowercase()) {
        Some(raw_key.to_uppercase())
    } else {
        Some(raw_key.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Keystroke, Modifiers};

    fn make_event(key: &str, ctrl: bool, alt: bool, shift: bool, cmd: bool) -> KeyDownEvent {
        KeyDownEvent {
            keystroke: Keystroke {
                modifiers: Modifiers {
                    control: ctrl,
                    alt,
                    shift,
                    platform: cmd,
                    function: false,
                },
                key: key.to_string(),
                key_char: None,
            },
            is_held: false,
        }
    }

    #[test]
    fn test_modifier_keys_ignored() {
        assert_eq!(key_event_to_nvim(&make_event("shift", false, false, true, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("Shift", false, false, true, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("control", true, false, false, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("ctrl", true, false, false, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("alt", false, true, false, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("super", false, false, false, true)), None);
        assert_eq!(key_event_to_nvim(&make_event("capslock", false, false, false, false)), None);
    }

    #[test]
    fn test_shift_g_and_uppercase() {
        assert_eq!(key_event_to_nvim(&make_event("G", false, false, true, false)), Some("G".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("g", false, false, true, false)), Some("G".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("$", false, false, true, false)), Some("$".to_string()));
    }

    #[test]
    fn test_escape_variants() {
        assert_eq!(key_event_to_nvim(&make_event("escape", false, false, false, false)), Some("<Esc>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("Escape", false, false, false, false)), Some("<Esc>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("esc", false, false, false, false)), Some("<Esc>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("\u{1b}", false, false, false, false)), Some("<Esc>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("\x1b", false, false, false, false)), Some("<Esc>".to_string()));
    }

    #[test]
    fn test_enter_variants() {
        assert_eq!(key_event_to_nvim(&make_event("enter", false, false, false, false)), Some("<CR>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("Return", false, false, false, false)), Some("<CR>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("\r", false, false, false, false)), Some("<CR>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("\n", false, false, false, false)), Some("<CR>".to_string()));
    }

    #[test]
    fn test_backspace_and_tab() {
        assert_eq!(key_event_to_nvim(&make_event("Backspace", false, false, false, false)), Some("<BS>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("\x08", false, false, false, false)), Some("<BS>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("\u{7f}", false, false, false, false)), Some("<BS>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("Tab", false, false, false, false)), Some("<Tab>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("\t", false, false, false, false)), Some("<Tab>".to_string()));
    }

    #[test]
    fn test_arrows_and_modifiers() {
        assert_eq!(key_event_to_nvim(&make_event("Up", false, false, false, false)), Some("<Up>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("c", true, false, false, false)), Some("<C-c>".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("s", false, false, false, true)), Some("<D-s>".to_string()));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_system_shortcuts_ignored() {
        assert_eq!(key_event_to_nvim(&make_event("q", false, false, false, true)), None);
        assert_eq!(key_event_to_nvim(&make_event("o", false, false, false, true)), None);
        assert_eq!(key_event_to_nvim(&make_event("r", false, false, false, true)), None);
        assert_eq!(key_event_to_nvim(&make_event("R", false, false, true, true)), None);
        assert_eq!(key_event_to_nvim(&make_event("r", false, false, true, true)), None);
        assert_eq!(key_event_to_nvim(&make_event("v", false, false, false, true)), None);
        assert_eq!(key_event_to_nvim(&make_event("c", false, false, false, true)), None);
        assert_eq!(key_event_to_nvim(&make_event("x", false, false, false, true)), None);
        assert!(key_event_to_nvim(&make_event("z", false, false, false, true)).is_none());
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_system_shortcuts_ignored() {
        assert_eq!(key_event_to_nvim(&make_event("q", true, false, true, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("o", true, false, true, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("o", true, true, false, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("r", true, false, true, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("R", true, false, true, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("v", true, false, true, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("c", true, false, true, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("x", true, false, true, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("z", true, false, true, false)), None);
        assert_eq!(key_event_to_nvim(&make_event("F4", false, true, false, false)), None);
    }

    #[test]
    fn test_plain_characters_route_to_nvim() {
        assert_eq!(key_event_to_nvim(&make_event("a", false, false, false, false)), Some("a".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("c", false, false, false, false)), Some("c".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("e", false, false, false, false)), Some("e".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("1", false, false, false, false)), Some("1".to_string()));
        assert_eq!(key_event_to_nvim(&make_event(":", false, false, false, false)), Some(":".to_string()));
        assert_eq!(key_event_to_nvim(&make_event("<", false, false, false, false)), Some("<lt>".to_string()));
    }
}
