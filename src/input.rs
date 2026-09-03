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

    // Ignore standalone modifier key press/release events without allocating String
    let is_standalone_modifier = match raw_key.len() {
        2 => raw_key.eq_ignore_ascii_case("fn"),
        3 => raw_key.eq_ignore_ascii_case("alt") || raw_key.eq_ignore_ascii_case("cmd"),
        4 => raw_key.eq_ignore_ascii_case("ctrl") || raw_key.eq_ignore_ascii_case("meta"),
        5 => raw_key.eq_ignore_ascii_case("shift") || raw_key.eq_ignore_ascii_case("super"),
        _ => {
            raw_key.eq_ignore_ascii_case("shift_l")
                || raw_key.eq_ignore_ascii_case("shift_r")
                || raw_key.eq_ignore_ascii_case("control")
                || raw_key.eq_ignore_ascii_case("control_l")
                || raw_key.eq_ignore_ascii_case("control_r")
                || raw_key.eq_ignore_ascii_case("alt_l")
                || raw_key.eq_ignore_ascii_case("alt_r")
                || raw_key.eq_ignore_ascii_case("meta_l")
                || raw_key.eq_ignore_ascii_case("meta_r")
                || raw_key.eq_ignore_ascii_case("super_l")
                || raw_key.eq_ignore_ascii_case("super_r")
                || raw_key.eq_ignore_ascii_case("command")
                || raw_key.eq_ignore_ascii_case("platform")
                || raw_key.eq_ignore_ascii_case("capslock")
                || raw_key.eq_ignore_ascii_case("caps_lock")
                || raw_key.eq_ignore_ascii_case("numlock")
                || raw_key.eq_ignore_ascii_case("num_lock")
                || raw_key.eq_ignore_ascii_case("scrolllock")
                || raw_key.eq_ignore_ascii_case("scroll_lock")
                || raw_key.eq_ignore_ascii_case("function")
                || raw_key.eq_ignore_ascii_case("iso_level3_shift")
                || raw_key.eq_ignore_ascii_case("mode_switch")
        }
    };

    if is_standalone_modifier {
        return None;
    }

    // Map special key names to Neovim equivalents (without allocating a lowercased String)
    let special_name = if raw_key.eq_ignore_ascii_case("enter") || raw_key.eq_ignore_ascii_case("return") || raw_key == "\r" || raw_key == "\n" {
        Some("CR")
    } else if raw_key.eq_ignore_ascii_case("escape") || raw_key.eq_ignore_ascii_case("esc") || raw_key == "\x1b" {
        Some("Esc")
    } else if raw_key.eq_ignore_ascii_case("backspace") || raw_key.eq_ignore_ascii_case("bs") || raw_key == "\x08" || raw_key == "\u{7f}" {
        Some("BS")
    } else if raw_key.eq_ignore_ascii_case("tab") || raw_key == "\t" {
        Some("Tab")
    } else if raw_key.eq_ignore_ascii_case("space") {
        Some("Space")
    } else if raw_key.eq_ignore_ascii_case("up") {
        Some("Up")
    } else if raw_key.eq_ignore_ascii_case("down") {
        Some("Down")
    } else if raw_key.eq_ignore_ascii_case("left") {
        Some("Left")
    } else if raw_key.eq_ignore_ascii_case("right") {
        Some("Right")
    } else if raw_key.eq_ignore_ascii_case("pageup") || raw_key.eq_ignore_ascii_case("page_up") {
        Some("PageUp")
    } else if raw_key.eq_ignore_ascii_case("pagedown") || raw_key.eq_ignore_ascii_case("page_down") {
        Some("PageDown")
    } else if raw_key.eq_ignore_ascii_case("home") {
        Some("Home")
    } else if raw_key.eq_ignore_ascii_case("end") {
        Some("End")
    } else if raw_key.eq_ignore_ascii_case("insert") {
        Some("Insert")
    } else if raw_key.eq_ignore_ascii_case("delete") || raw_key.eq_ignore_ascii_case("del") {
        Some("Del")
    } else if raw_key.eq_ignore_ascii_case("f1") {
        Some("F1")
    } else if raw_key.eq_ignore_ascii_case("f2") {
        Some("F2")
    } else if raw_key.eq_ignore_ascii_case("f3") {
        Some("F3")
    } else if raw_key.eq_ignore_ascii_case("f4") {
        Some("F4")
    } else if raw_key.eq_ignore_ascii_case("f5") {
        Some("F5")
    } else if raw_key.eq_ignore_ascii_case("f6") {
        Some("F6")
    } else if raw_key.eq_ignore_ascii_case("f7") {
        Some("F7")
    } else if raw_key.eq_ignore_ascii_case("f8") {
        Some("F8")
    } else if raw_key.eq_ignore_ascii_case("f9") {
        Some("F9")
    } else if raw_key.eq_ignore_ascii_case("f10") {
        Some("F10")
    } else if raw_key.eq_ignore_ascii_case("f11") {
        Some("F11")
    } else if raw_key.eq_ignore_ascii_case("f12") {
        Some("F12")
    } else if raw_key == "<" {
        Some("lt")
    } else {
        None
    };

    if let Some(name) = special_name {
        let is_space_or_lt = raw_key.eq_ignore_ascii_case("space") || raw_key == "<";
        let has_shift = shift && !is_space_or_lt;
        if !ctrl && !alt && !cmd && !has_shift {
            return Some(match name {
                "CR" => "<CR>".to_string(),
                "Esc" => "<Esc>".to_string(),
                "BS" => "<BS>".to_string(),
                "Tab" => "<Tab>".to_string(),
                "Space" => "<Space>".to_string(),
                "Up" => "<Up>".to_string(),
                "Down" => "<Down>".to_string(),
                "Left" => "<Left>".to_string(),
                "Right" => "<Right>".to_string(),
                "PageUp" => "<PageUp>".to_string(),
                "PageDown" => "<PageDown>".to_string(),
                "Home" => "<Home>".to_string(),
                "End" => "<End>".to_string(),
                "Insert" => "<Insert>".to_string(),
                "Del" => "<Del>".to_string(),
                "lt" => "<lt>".to_string(),
                _ => format!("<{name}>"),
            });
        }

        let mut prefix = String::with_capacity(8);
        if ctrl {
            prefix.push_str("C-");
        }
        if alt {
            prefix.push_str("M-");
        }
        if cmd {
            prefix.push_str("D-");
        }
        if has_shift {
            prefix.push_str("S-");
        }
        return Some(format!("<{}{}>", prefix, name));
    }

    // Single character with modifiers (Ctrl, Alt, Cmd)
    if ctrl || alt || cmd {
        let mut prefix = String::with_capacity(8);
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
