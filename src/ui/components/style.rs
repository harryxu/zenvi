use gpui::*;

/// Pre-computed titlebar and menu colors derived from the Neovim theme.
pub struct TitlebarStyle {
    pub title_color: Rgba,
    pub border_color: Rgba,
    #[cfg(not(target_os = "macos"))]
    pub menu_hover_bg: Rgba,
    #[cfg(not(target_os = "macos"))]
    pub menu_active_bg: Rgba,
    #[cfg(not(target_os = "macos"))]
    pub dropdown_bg: Rgba,
}

/// Packs three floating-point channel values into a single `u32` RGB color.
fn pack_rgb(r: f32, g: f32, b: f32) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Derives harmonious titlebar and menu colors from Neovim's active theme colors.
pub fn derive_titlebar_style(default_bg: u32, default_fg: u32) -> TitlebarStyle {
    let (bg_r, bg_g, bg_b) = (
        ((default_bg >> 16) & 0xff) as f32,
        ((default_bg >> 8) & 0xff) as f32,
        (default_bg & 0xff) as f32,
    );
    let luminance = 0.299 * bg_r + 0.587 * bg_g + 0.114 * bg_b;
    let is_dark = luminance < 128.0;

    let title_color = rgb(default_fg);

    let border_color = if is_dark {
        rgb(pack_rgb(
            (bg_r + 18.0).min(255.0),
            (bg_g + 18.0).min(255.0),
            (bg_b + 18.0).min(255.0),
        ))
    } else {
        rgb(pack_rgb(
            (bg_r - 25.0).max(0.0),
            (bg_g - 25.0).max(0.0),
            (bg_b - 25.0).max(0.0),
        ))
    };

    #[cfg(not(target_os = "macos"))]
    let menu_hover_bg = if is_dark {
        rgb(pack_rgb(
            (bg_r + 20.0).min(255.0),
            (bg_g + 20.0).min(255.0),
            (bg_b + 20.0).min(255.0),
        ))
    } else {
        rgb(pack_rgb(
            (bg_r - 20.0).max(0.0),
            (bg_g - 20.0).max(0.0),
            (bg_b - 20.0).max(0.0),
        ))
    };

    #[cfg(not(target_os = "macos"))]
    let menu_active_bg = if is_dark {
        rgb(pack_rgb(
            (bg_r + 35.0).min(255.0),
            (bg_g + 35.0).min(255.0),
            (bg_b + 35.0).min(255.0),
        ))
    } else {
        rgb(pack_rgb(
            (bg_r - 35.0).max(0.0),
            (bg_g - 35.0).max(0.0),
            (bg_b - 35.0).max(0.0),
        ))
    };

    #[cfg(not(target_os = "macos"))]
    let dropdown_bg = if is_dark {
        rgb(pack_rgb(
            (bg_r + 14.0).min(255.0),
            (bg_g + 14.0).min(255.0),
            (bg_b + 14.0).min(255.0),
        ))
    } else {
        rgb(pack_rgb(
            (bg_r - 12.0).max(0.0),
            (bg_g - 12.0).max(0.0),
            (bg_b - 12.0).max(0.0),
        ))
    };

    TitlebarStyle {
        title_color,
        border_color,
        #[cfg(not(target_os = "macos"))]
        menu_hover_bg,
        #[cfg(not(target_os = "macos"))]
        menu_active_bg,
        #[cfg(not(target_os = "macos"))]
        dropdown_bg,
    }
}
