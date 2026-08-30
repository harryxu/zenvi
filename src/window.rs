use crate::ui::ZenviView;
use gpui::*;
use std::path::PathBuf;

pub fn get_nvim_config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(xdg).join("nvim");
        if p.exists() {
            return p;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".config").join("nvim");
        return p;
    }
    PathBuf::from(".config/nvim")
}

pub fn get_safe_default_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let state_dir = PathBuf::from(home).join(".local").join("state").join("zenvi");
        let _ = std::fs::create_dir_all(&state_dir);
        return Some(state_dir);
    }
    None
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliLaunchConfig {
    pub cwd: Option<PathBuf>,
    pub targets: Vec<PathBuf>,
}

pub fn parse_cli_args<I, S>(args: I, current_dir: Option<PathBuf>) -> CliLaunchConfig
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut targets = Vec::new();

    for arg in args {
        let s = arg.as_ref();
        if s.starts_with("-psn_") || s.is_empty() {
            // Ignore macOS Finder process serial number
            continue;
        }
        let p = PathBuf::from(s);
        let absolute_path = if p.is_absolute() {
            p
        } else if let Some(ref cwd) = current_dir {
            cwd.join(&p)
        } else {
            p
        };
        targets.push(absolute_path);
    }

    let cwd = if let Some(ref cwd) = current_dir {
        Some(cwd.clone())
    } else if let Some(first) = targets.first() {
        if first.is_dir() {
            Some(first.clone())
        } else if let Some(parent) = first.parent() {
            if parent.exists() && parent.as_os_str() != "" {
                Some(parent.to_path_buf())
            } else {
                get_safe_default_dir()
            }
        } else {
            get_safe_default_dir()
        }
    } else {
        get_safe_default_dir()
    };

    CliLaunchConfig { cwd, targets }
}

pub fn resolve_cli_launch_config() -> CliLaunchConfig {
    let current_dir = std::env::current_dir().ok().filter(|c| {
        c.as_os_str() != "/" && !c.to_string_lossy().contains(".app/Contents")
    });
    parse_cli_args(std::env::args().skip(1), current_dir)
}

pub fn open_zenvi_window(cwd: Option<PathBuf>, targets: Vec<PathBuf>, cx: &mut App) {
    let window_size = Size::new(px(1080.0), px(720.0));
    let window_count = cx.windows().len();
    let offset = px((window_count as f32 % 10.0) * 28.0);

    let window_bounds = if let Some(display) = cx.displays().first() {
        let screen = display.bounds();
        let origin = Point::new(
            (screen.origin.x + ((screen.size.width - window_size.width) / 2.0).max(px(0.0))) + offset,
            (screen.origin.y + ((screen.size.height - window_size.height) / 2.0).max(px(0.0))) + offset,
        );
        Bounds::new(origin, window_size)
    } else {
        Bounds::new(Point::new(px(100.0) + offset, px(100.0) + offset), window_size)
    };

    let mut window_options = WindowOptions::default();
    window_options.window_bounds = Some(WindowBounds::Windowed(window_bounds));
    window_options.focus = true;
    window_options.show = true;
    window_options.titlebar = Some(TitlebarOptions {
        title: Some("Zenvi".into()),
        appears_transparent: true,
        traffic_light_position: Some(Point::new(px(12.0), px(10.0))),
    });

    cx.open_window(window_options, |window, cx| {
        cx.activate(true);

        let window_handle = window.window_handle();
        let view = cx.new(|cx| {
            let view = ZenviView::with_cwd_and_targets(window_handle, cwd, targets, cx);
            window.focus(&view.focus_handle);
            view
        });

        view
    })
    .expect("Failed to open GPUI window");
}

#[cfg(test)]
mod tests {
    use super::parse_cli_args;
    use std::path::PathBuf;

    #[test]
    fn test_parse_cli_args_files_and_directories() {
        let cwd = PathBuf::from("/Users/test/workspace");
        let args = vec!["src/main.rs", "README.md"];
        let config = parse_cli_args(args, Some(cwd.clone()));

        assert_eq!(config.cwd, Some(cwd.clone()));
        assert_eq!(config.targets.len(), 2);
        assert_eq!(config.targets[0], cwd.join("src/main.rs"));
        assert_eq!(config.targets[1], cwd.join("README.md"));
    }

    #[test]
    fn test_parse_cli_args_dot_and_psn() {
        let cwd = PathBuf::from("/Users/test/workspace");
        let args = vec!["-psn_0_123456", "."];
        let config = parse_cli_args(args, Some(cwd.clone()));

        assert_eq!(config.cwd, Some(cwd.clone()));
        assert_eq!(config.targets.len(), 1);
        assert_eq!(config.targets[0], cwd.join("."));
    }
}
