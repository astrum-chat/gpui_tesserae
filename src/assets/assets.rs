use std::borrow::Cow;

use anyhow::anyhow;
use gpui::{App, AssetSource, Result, SharedString};
use smallvec::SmallVec;

/// Composite asset source that queries multiple providers in order.
///
/// The first provider to return an asset for a given path wins.
pub struct Assets<const N: usize> {
    providers: SmallVec<[Box<dyn AssetProvider>; N]>,
}

impl<const N: usize> Assets<N> {
    /// Creates a new asset source from an array of providers.
    pub fn new(providers: [Box<dyn AssetProvider>; N]) -> Assets<N> {
        Self {
            providers: SmallVec::from(providers),
        }
    }
}

impl Assets<0> {
    /// Initializes font assets.
    pub fn init_fonts(cx: &mut App) -> gpui::Result<()> {
        let font_paths = cx.asset_source().list("fonts")?;
        let mut embedded_fonts = Vec::new();

        for font_path in font_paths {
            if !font_path.ends_with(".ttf") {
                continue;
            }

            let Some(font_bytes) = cx.asset_source().load(&font_path)? else {
                continue;
            };

            embedded_fonts.push(font_bytes);
        }

        cx.text_system().add_fonts(embedded_fonts)
    }
}

/// Creates an `Assets` instance from a list of asset providers.
#[macro_export]
macro_rules! assets {
    ( $( $item:expr ),* $(,)? ) => {
        $crate::Assets::new([
            $( Box::new($item) ),*
        ])
    };
}

impl<const N: usize> AssetSource for Assets<N> {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        for provider in &self.providers {
            let asset = provider.get(path);

            if asset.is_some() {
                return Ok(asset);
            }
        }

        Err(anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(self
            .providers
            .iter()
            .flat_map(|assets| assets.list(path).into_iter())
            .flatten()
            .collect())
    }
}

/// Trait for types that can provide asset data.
pub trait AssetProvider: Send + Sync {
    /// Returns the asset data at the given path, if it exists.
    fn get(&self, path: &str) -> Option<Cow<'static, [u8]>>;

    /// Lists all assets under the given path prefix.
    fn list(&self, path: &str) -> Result<Vec<SharedString>>;
}
