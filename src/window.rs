use crate::ui::ZenviView;
use gpui::*;
use std::path::PathBuf;

pub fn get_nvim_config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local_app_data).join("nvim");
            if p.exists() {
                return p;
            }
        }
        if let Ok(app_data) = std::env::var("APPDATA") {
            return PathBuf::from(app_data).join("nvim");
        }
    }
    #[cfg(not(windows))]
    {
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
    }
    PathBuf::from(".config/nvim")
}

pub fn get_nvim_config_file() -> (PathBuf, PathBuf) {
    let config_dir = get_nvim_config_dir();
    if !config_dir.exists() {
        let _ = std::fs::create_dir_all(&config_dir);
    }
    let init_lua = config_dir.join("init.lua");
    let init_vim = config_dir.join("init.vim");
    let target_file = if init_lua.exists() {
        init_lua
    } else if init_vim.exists() {
        init_vim
    } else {
        init_lua
    };
    (config_dir, target_file)
}

pub fn get_safe_default_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let state_dir = PathBuf::from(local_app_data).join("zenvi");
            let _ = std::fs::create_dir_all(&state_dir);
            return Some(state_dir);
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let state_dir = PathBuf::from(home).join(".local").join("state").join("zenvi");
            let _ = std::fs::create_dir_all(&state_dir);
            return Some(state_dir);
        }
    }
    None
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliLaunchConfig {
    pub cwd: Option<PathBuf>,
    pub targets: Vec<PathBuf>,
    /// Remove OS-provided window decorations (border + titlebar). Linux only.
    pub borderless: bool,
}

pub fn parse_cli_args<I, S>(args: I, current_dir: Option<PathBuf>) -> CliLaunchConfig
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut targets = Vec::new();
    let mut borderless = false;

    for arg in args {
        let s = arg.as_ref();
        if s.starts_with("-psn_") || s.is_empty() {
            // Ignore macOS Finder process serial number
            continue;
        }
        if s == "--no-titlebar" || s == "-B" {
            borderless = true;
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

    CliLaunchConfig { cwd, targets, borderless }
}

pub fn resolve_cli_launch_config() -> CliLaunchConfig {
    let current_dir = std::env::current_dir().ok().filter(|c| {
        c.as_os_str() != "/" && !c.to_string_lossy().contains(".app/Contents")
    });
    parse_cli_args(std::env::args().skip(1), current_dir)
}

pub fn decode_percent(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

pub fn url_to_path(url_str: &str) -> Option<PathBuf> {
    let s = if let Some(stripped) = url_str.strip_prefix("file://") {
        stripped
    } else {
        url_str
    };
    let decoded = decode_percent(s);
    if decoded.is_empty() {
        None
    } else {
        Some(PathBuf::from(decoded))
    }
}

pub fn open_zenvi_window(cwd: Option<PathBuf>, targets: Vec<PathBuf>, borderless: bool, cx: &mut App) {
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

    #[cfg(target_os = "macos")]
    {
        window_options.titlebar = Some(TitlebarOptions {
            title: Some("Zenvi".into()),
            appears_transparent: true,
            traffic_light_position: Some(Point::new(px(12.0), px(12.0))),
        });
    }
    #[cfg(not(target_os = "macos"))]
    {
        if borderless {
            // Client-side decorations: remove OS-provided border and titlebar.
            // The custom-drawn titlebar in ui/components/titlebar.rs is retained for
            // window title, menu button, window controls, and drag-to-move support.
            window_options.titlebar = None;
            window_options.window_decorations = Some(WindowDecorations::Client);
            window_options.window_background = WindowBackgroundAppearance::Transparent;
        } else {
            window_options.titlebar = Some(TitlebarOptions {
                title: Some("Zenvi".into()),
                appears_transparent: false,
                traffic_light_position: None,
            });
            window_options.window_background = WindowBackgroundAppearance::Opaque;
        }
    }

    cx.open_window(window_options, |window, cx| {
        cx.activate(true);

        let window_handle = window.window_handle();
        let view = cx.new(|cx| {
            cx.observe_window_appearance(window, |_, window, _| {
                window.refresh();
            })
            .detach();

            let view = ZenviView::with_cwd_and_targets(window_handle, cwd, targets, borderless, cx);
            window.focus(&view.focus_handle);
            view
        });

        view
    })
    .expect("Failed to open GPUI window");
}

#[cfg(test)]
mod tests {
    use super::{get_nvim_config_file, parse_cli_args, url_to_path};
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
        assert!(!config.borderless);
    }

    #[test]
    fn test_parse_cli_args_borderless_flags() {
        let cwd = PathBuf::from("/Users/test/workspace");
        let config1 = parse_cli_args(vec!["--no-titlebar", "main.rs"], Some(cwd.clone()));
        assert!(config1.borderless);
        assert_eq!(config1.targets.len(), 1);

        let config2 = parse_cli_args(vec!["-B", "src/lib.rs"], Some(cwd.clone()));
        assert!(config2.borderless);
        assert_eq!(config2.targets.len(), 1);
    }

    #[test]
    fn test_get_nvim_config_file_returns_valid_path() {
        let (config_dir, target_file) = get_nvim_config_file();
        assert!(config_dir.is_absolute() || !config_dir.as_os_str().is_empty());
        assert!(target_file.starts_with(&config_dir));
        assert!(target_file.ends_with("init.lua") || target_file.ends_with("init.vim"));
    }

    #[test]
    fn test_url_to_path_decoding() {
        let u1 = "file:///Users/harry/project/src/main.rs";
        assert_eq!(
            url_to_path(u1),
            Some(PathBuf::from("/Users/harry/project/src/main.rs"))
        );

        let u2 = "file:///Users/harry/my%20folder/my%20file.txt";
        assert_eq!(
            url_to_path(u2),
            Some(PathBuf::from("/Users/harry/my folder/my file.txt"))
        );

        let u3 = "file:///Users/harry/%E4%B8%AD%E6%96%87%E7%9B%AE%E5%BD%95";
        assert_eq!(
            url_to_path(u3),
            Some(PathBuf::from("/Users/harry/中文目录"))
        );
    }
}
