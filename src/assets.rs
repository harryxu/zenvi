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
            "zenvi-icon.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/zenvi-icon.svg"
            )))),
            _ => Ok(None),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let all: &[&str] = &["icons/menu.svg", "zenvi-icon.svg"];
        Ok(all
            .iter()
            .filter(|p| p.starts_with(path))
            .map(|p| SharedString::from(*p))
            .collect())
    }
}
