mod assets;
pub use assets::*;
use cfg_if::cfg_if;

cfg_if!(
    if #[cfg(feature = "assets")] {
        mod tesserae_assets;
        pub use tesserae_assets::*;
    }
);
