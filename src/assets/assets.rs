use std::borrow::Cow;

use anyhow::anyhow;
use gpui::{AssetSource, Result, SharedString};
use smallvec::SmallVec;

pub struct Assets<const N: usize> {
    providers: SmallVec<[Box<dyn AssetProvider>; N]>,
}

impl<const N: usize> Assets<N> {
    pub fn new(providers: [Box<dyn AssetProvider>; N]) -> Assets<N> {
        Self {
            providers: SmallVec::from(providers),
        }
    }
}

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

pub trait AssetProvider: Send + Sync {
    fn get(&self, path: &str) -> Option<Cow<'static, [u8]>>;
    fn list(&self, path: &str) -> Result<Vec<SharedString>>;
}
