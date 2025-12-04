cfg_if::cfg_if!(
    if #[cfg(feature = "assets")] {
        use std::borrow::Cow;

        use gpui::{Result, SharedString};
        use rust_embed::RustEmbed;

        use crate::assets::assets::AssetProvider;

        #[derive(RustEmbed)]
        #[folder = "assets/"]
        pub struct TesseraeAssets;

        impl AssetProvider for TesseraeAssets {
            fn get(&self, path: &str) -> Option<Cow<'static, [u8]>> {
                <Self as RustEmbed>::get(path).map(|f| f.data)
            }

            fn list(&self, path: &str) -> Result<Vec<SharedString>> {
                Ok(TesseraeAssets::iter()
                    .filter_map(|p| p.starts_with(path).then(|| p.into()))
                    .collect())
            }
        }
    }
);

use enum_assoc::Assoc;

#[derive(Assoc)]
#[func(pub fn path(&self) -> SharedString)]
pub enum TesseraeIconKind {
    #[assoc(path = "icons/checkmark.svg".into())]
    Checkmark,
}

impl Into<SharedString> for TesseraeIconKind {
    fn into(self) -> SharedString {
        self.path()
    }
}
