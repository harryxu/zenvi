#[derive(Debug, Clone, PartialEq)]
pub struct ParsedGuiFont {
    pub family: Option<String>,
    pub size: Option<f32>,
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
