use gpui::KeyDownEvent;

pub fn key_event_to_nvim(event: &KeyDownEvent) -> Option<String> {
    let key = event.keystroke.key.as_str();
    let mods = &event.keystroke.modifiers;

    let ctrl = mods.control;
    let alt = mods.alt;
    let shift = mods.shift;
    let cmd = mods.platform;

    // Do not forward system menu shortcuts to Neovim
    if cmd {
        match key {
            "q" | "Q" | "o" | "O" => return None,
            _ => {}
        }
    }

    // Map special key names to Neovim equivalents
    let special_name = match key {
        "enter" | "return" => Some("CR"),
        "escape" | "esc" => Some("Esc"),
        "backspace" => Some("BS"),
        "tab" => Some("Tab"),
        "space" => Some("Space"),
        "up" => Some("Up"),
        "down" => Some("Down"),
        "left" => Some("Left"),
        "right" => Some("Right"),
        "pageup" => Some("PageUp"),
        "pagedown" => Some("PageDown"),
        "home" => Some("Home"),
        "end" => Some("End"),
        "insert" => Some("Insert"),
        "delete" => Some("Del"),
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

    // Single character with modifiers or raw
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
        return Some(format!("<{}{}>", prefix, key));
    }

    // Normal text input
    if let Some(ref ch) = event.keystroke.key_char {
        if ch == "<" {
            return Some("<lt>".to_string());
        }
        return Some(ch.to_string());
    }

    if key.chars().count() == 1 {
        if key == "<" {
            return Some("<lt>".to_string());
        }
        return Some(key.to_string());
    }

    None
}
