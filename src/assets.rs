use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let clean_path = path.strip_prefix("assets/").unwrap_or(path);
        match clean_path {
            "icons/menu.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icons/menu.svg"
            )))),
            "icons/panel-left.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icons/panel-left.svg"
            )))),
            "icons/panel-left-open.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icons/panel-left-open.svg"
            )))),
            "zenvi-icon.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/zenvi-icon.svg"
            )))),
            _ => Ok(None),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let all: &[&str] = &[
            "icons/menu.svg",
            "icons/panel-left.svg",
            "icons/panel-left-open.svg",
            "zenvi-icon.svg",
        ];
        Ok(all
            .iter()
            .filter(|p| p.starts_with(path))
            .map(|p| SharedString::from(*p))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assets_load_panel_icons() {
        let assets = Assets;
        assert!(assets.load("icons/panel-left.svg").unwrap().is_some());
        assert!(assets.load("assets/icons/panel-left.svg").unwrap().is_some());
        assert!(assets.load("icons/panel-left-open.svg").unwrap().is_some());
        assert!(assets.load("assets/icons/panel-left-open.svg").unwrap().is_some());
    }

    #[test]
    fn test_assets_list_includes_panel_icons() {
        let assets = Assets;
        let list = assets.list("icons/panel-left").unwrap();
        assert_eq!(list.len(), 2);
    }
}
