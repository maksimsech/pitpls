//! Embedded assets used by both gpui-components and pitpls branding.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match path {
            "images/app-icon.png" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icon.png")))),
            _ => gpui_component_assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut assets = gpui_component_assets::Assets.list(path)?;
        if "images/app-icon.png".starts_with(path) {
            assets.push("images/app-icon.png".into());
        }
        Ok(assets)
    }
}
