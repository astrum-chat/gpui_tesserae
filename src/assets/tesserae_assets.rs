#![allow(missing_docs)] // Derive macros generate undocumented methods.

cfg_if::cfg_if!(
    if #[cfg(feature = "assets")] {
        use std::borrow::Cow;

        use gpui::{Result, SharedString};
        use rust_embed::RustEmbed;

        use crate::assets::assets::AssetProvider;

        /// Embedded assets bundled with the tesserae crate.
        #[derive(RustEmbed)]
        #[folder = "assets/"]
        #[include = "fonts/**/*.ttf"]
        #[include = "icons/**/*.svg"]
        #[exclude = "*.DS_Store"]
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

/// Built-in icon identifiers that map to bundled SVG assets.
#[derive(Assoc)]
#[func(pub fn path(&self) -> SharedString)]
pub enum TesseraeIconKind {
    /// Checkmark icon for confirmations and selections.
    #[assoc(path = "icons/checkmark.svg".into())]
    Checkmark,

    /// Downward arrow for dropdowns and expand indicators.
    #[assoc(path = "icons/arrow_down.svg".into())]
    ArrowDown,
}

impl Into<SharedString> for TesseraeIconKind {
    fn into(self) -> SharedString {
        self.path()
    }
}
