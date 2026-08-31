use gpui::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedGuiFont {
    pub family: Option<String>,
    pub size: Option<f32>,
}

pub fn default_font_family() -> &'static str {
    #[cfg(target_os = "macos")]
    return "Menlo";
    #[cfg(target_os = "windows")]
    return "Consolas";
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return "monospace";
}

pub fn parse_guifont(guifont: &str) -> ParsedGuiFont {
    let mut result = ParsedGuiFont {
        family: None,
        size: None,
    };

    let trimmed = guifont.trim();
    if trimmed.is_empty() {
        return result;
    }

    // In Vim/Neovim, guifont can be a comma-separated list of fallbacks (e.g. "JetBrainsMono Nerd Font:h14,Menlo:h14")
    let primary = trimmed.split(",").next().unwrap_or(trimmed).trim();
    if primary.is_empty() {
        return result;
    }

    let parts: Vec<&str> = primary.split(":").collect();
    if parts.is_empty() {
        return result;
    }

    // First part is font name (if non-empty)
    let font_name = parts[0].trim();
    if !font_name.is_empty() {
        // Vim replaces underscores with spaces and handles backslash escapes
        let unescaped = font_name.replace("\\", "");
        let with_spaces = unescaped.replace("_", " ");
        let cleaned = with_spaces.trim().to_string();
        if !cleaned.is_empty() {
            result.family = Some(cleaned);
        }
    }

    // Subsequent parts are attributes like "h14", "h14.5", "w8", etc.
    for part in &parts[1..] {
        let part = part.trim();
        if part.starts_with("h") || part.starts_with("H") {
            if let Ok(size) = part[1..].parse::<f32>() {
                if size > 4.0 && size < 120.0 {
                    result.size = Some(size);
                }
            }
        }
    }

    result
}

use super::ZenviView;

impl ZenviView {
    /// Updates font family, size, char width and line height from Neovim's guifont/linespace options.
    pub fn update_font(&mut self, guifont: &str, linespace: i64, cx: &App) {
        let parsed = parse_guifont(guifont);

        if let Some(family) = parsed.family {
            self.font_family = family;
        } else if guifont.is_empty() {
            self.font_family = default_font_family().to_string();
        }

        let size: f32 = if let Some(s) = parsed.size {
            s
        } else if guifont.is_empty() {
            14.0
        } else {
            self.font_size.into()
        };

        self.font_size = px(size);

        // Measure actual monospace advance width using GPUI text system
        let font_id = cx.text_system().resolve_font(&font(&self.font_family));
        let advance: f32 = cx
            .text_system()
            .advance(font_id, self.font_size, '0')
            .or_else(|_| cx.text_system().advance(font_id, self.font_size, 'm'))
            .map(|s| s.width.into())
            .unwrap_or(size * 0.6015);
        self.char_width = advance;

        // Line height calculation: terminal monospace 1.2x ratio + linespace pixels
        let base_lh = (size * 1.2).round();
        let final_lh = (base_lh + linespace as f32).max(8.0);
        self.line_height = px(final_lh);
    }

    /// Checks if Neovim's guifont or linespace has changed and updates font metrics accordingly.
    pub fn sync_font_if_changed(&mut self, cx: &App) {
        let (guifont_changed, new_guifont, new_linespace) = {
            let state = self.session.state.read();
            if state.guifont != self.last_guifont || state.linespace != self.last_linespace {
                (true, state.guifont.clone(), state.linespace)
            } else {
                (false, String::new(), 0)
            }
        };

        if guifont_changed {
            self.last_guifont = new_guifont.clone();
            self.last_linespace = new_linespace;
            self.update_font(&new_guifont, new_linespace, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_guifont_full() {
        let parsed = parse_guifont("JetBrainsMono_Nerd_Font:h15.5");
        assert_eq!(parsed.family, Some("JetBrainsMono Nerd Font".to_string()));
        assert_eq!(parsed.size, Some(15.5));
    }

    #[test]
    fn test_parse_guifont_spaces_and_attributes() {
        let parsed = parse_guifont("Fira Code:h16:b:w8");
        assert_eq!(parsed.family, Some("Fira Code".to_string()));
        assert_eq!(parsed.size, Some(16.0));
    }

    #[test]
    fn test_parse_guifont_fallback_list() {
        let parsed = parse_guifont("Source Code Pro:h14,Menlo:h14");
        assert_eq!(parsed.family, Some("Source Code Pro".to_string()));
        assert_eq!(parsed.size, Some(14.0));
    }

    #[test]
    fn test_parse_guifont_size_only() {
        let parsed = parse_guifont(":h18");
        assert_eq!(parsed.family, None);
        assert_eq!(parsed.size, Some(18.0));
    }

    #[test]
    fn test_parse_guifont_font_only() {
        let parsed = parse_guifont("Hack");
        assert_eq!(parsed.family, Some("Hack".to_string()));
        assert_eq!(parsed.size, None);
    }

    #[test]
    fn test_parse_guifont_empty() {
        let parsed = parse_guifont("");
        assert_eq!(parsed.family, None);
        assert_eq!(parsed.size, None);
    }

    #[test]
    fn test_parse_guifont_escaped_spaces() {
        let parsed = parse_guifont("Fira\\ Code\\ Nerd\\ Font:h13");
        assert_eq!(parsed.family, Some("Fira Code Nerd Font".to_string()));
        assert_eq!(parsed.size, Some(13.0));
    }
}
