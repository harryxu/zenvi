use gpui::*;

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
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
    return "DejaVu Sans Mono";
}

/// Resolves the best available monospace font family using GPUI's own font database.
/// This ensures the returned name matches what cosmic-text/fontdb actually uses,
/// avoiding mismatches between fontconfig and GPUI's font naming.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn resolve_default_font_family(cx: &App) -> String {
    use std::sync::OnceLock;
    static RESOLVED: OnceLock<String> = OnceLock::new();
    RESOLVED
        .get_or_init(|| {
            let all_fonts = cx.text_system().all_font_names();

            // 1. Try fontconfig-configured monospace font with fuzzy matching
            if let Some(fc_name) = fc_match_monospace() {
                // Exact match
                if all_fonts.iter().any(|f| f == &fc_name) {
                    eprintln!("[zenvi] Using monospace font (fontconfig exact): {fc_name}");
                    return fc_name;
                }
                // Fuzzy match: strip spaces and compare case-insensitively
                let fc_normalized = fc_name.to_lowercase().replace(' ', "");
                if let Some(found) = all_fonts
                    .iter()
                    .find(|f| f.to_lowercase().replace(' ', "") == fc_normalized)
                {
                    eprintln!("[zenvi] Using monospace font (fontconfig fuzzy): {found}");
                    return found.clone();
                }
            }

            // 2. Try well-known monospace font families in preference order
            let preferred = [
                "JetBrains Mono",
                "JetBrainsMono Nerd Font",
                "JetBrainsMono NF",
                "Fira Code",
                "Source Code Pro",
                "Cascadia Code",
                "Cascadia Mono",
                "DejaVu Sans Mono",
                "Liberation Mono",
                "Ubuntu Mono",
                "Noto Sans Mono",
                "Hack",
                "Inconsolata",
                "Adwaita Mono",
                "Nimbus Mono PS",
                "Droid Sans Mono",
                "Courier New",
            ];
            for name in &preferred {
                if all_fonts.iter().any(|f| f == name) {
                    eprintln!("[zenvi] Using monospace font (preferred list): {name}");
                    return name.to_string();
                }
            }

            // 3. Search for any font with "Mono" in the name (likely monospace)
            if let Some(mono) = all_fonts.iter().find(|f| {
                let lower = f.to_lowercase();
                (lower.contains("mono") || lower.contains("courier"))
                    && !lower.contains("emoji")
            }) {
                eprintln!("[zenvi] Using monospace font (name heuristic): {mono}");
                return mono.clone();
            }

            // 4. Last resort fallback
            let fallback = default_font_family().to_string();
            eprintln!("[zenvi] No monospace font found, falling back to: {fallback}");
            fallback
        })
        .clone()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn fc_match_monospace() -> Option<String> {
    let output = std::process::Command::new("fc-match")
        .args(["monospace", "-f", "%{family}"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let family = String::from_utf8_lossy(&output.stdout);
    let primary = family.split(',').next()?.trim().to_string();
    if primary.is_empty() {
        None
    } else {
        Some(primary)
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn resolve_default_font_family(_cx: &App) -> String {
    default_font_family().to_string()
}

pub fn parse_single_guifont(spec: &str) -> (Option<String>, Option<f32>) {
    let mut family = None;
    let mut size = None;

    let parts: Vec<&str> = spec.split(':').collect();
    if !parts.is_empty() {
        let font_name = parts[0].trim();
        if !font_name.is_empty() {
            let unescaped = font_name.replace('\\', "");
            let with_spaces = unescaped.replace('_', " ");
            let cleaned = with_spaces.trim().to_string();
            if !cleaned.is_empty() {
                family = Some(cleaned);
            }
        }

        for part in &parts[1..] {
            let part = part.trim();
            if part.starts_with('h') || part.starts_with('H') {
                if let Ok(s) = part[1..].parse::<f32>() {
                    if s > 4.0 && s < 120.0 {
                        size = Some(s);
                    }
                }
            }
        }
    }

    (family, size)
}

#[allow(dead_code)]
pub fn parse_guifont(guifont: &str) -> ParsedGuiFont {
    let trimmed = guifont.trim();
    if trimmed.is_empty() {
        return ParsedGuiFont {
            family: None,
            size: None,
        };
    }

    let primary = trimmed.split(',').next().unwrap_or(trimmed).trim();
    let (family, size) = parse_single_guifont(primary);
    ParsedGuiFont { family, size }
}

/// Parses Neovim's guifont string (which may be a comma-separated fallback list)
/// and resolves the best available font against the system font database.
pub fn resolve_guifont(guifont: &str, cx: &App) -> (String, Option<f32>) {
    let trimmed = guifont.trim();
    if trimmed.is_empty() {
        return (resolve_default_font_family(cx), None);
    }

    let all_fonts = cx.text_system().all_font_names();
    let candidates: Vec<&str> = trimmed.split(',').collect();

    let mut first_size = None;
    let mut resolved_family = None;

    for candidate in candidates {
        let (family, size) = parse_single_guifont(candidate.trim());
        if first_size.is_none() && size.is_some() {
            first_size = size;
        }

        if resolved_family.is_none() {
            if let Some(ref name) = family {
                if name.eq_ignore_ascii_case("monospace") {
                    resolved_family = Some(resolve_default_font_family(cx));
                } else if all_fonts.iter().any(|f| f == name) {
                    resolved_family = Some(name.clone());
                } else {
                    // Try case-insensitive / normalized match
                    let normalized = name.to_lowercase().replace(' ', "");
                    if let Some(found) = all_fonts
                        .iter()
                        .find(|f| f.to_lowercase().replace(' ', "") == normalized)
                    {
                        resolved_family = Some(found.clone());
                    }
                }
            }
        }
    }

    let final_family = resolved_family.unwrap_or_else(|| resolve_default_font_family(cx));
    (final_family, first_size)
}

use super::ZenviView;

impl ZenviView {
    /// Updates font family, size, char width and line height from Neovim's guifont/linespace options.
    pub fn update_font(&mut self, guifont: &str, linespace: i64, cx: &App) {
        let (family, size_opt) = resolve_guifont(guifont, cx);
        self.font_family = family;

        let size: f32 = if let Some(s) = size_opt {
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
    use super::parse_guifont;

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
