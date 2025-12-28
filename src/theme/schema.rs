use std::{
    ops::{Deref, DerefMut},
    sync::LazyLock,
};

use gpui::{AbsoluteLength, App, DefiniteLength, Global, Pixels, Rgba, SharedString};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use super::deserializers::{
    de_abs_length, de_def_length, de_pixels, de_string_or_non_empty_list, de_variants,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Theme {
    pub name: SharedString,
    pub layout: ThemeLayout,
    pub variants: ThemeVariants,
}

macro_rules! generate_builtin_themes {
    ( $( [$path:literal, $name:ident] ),+ ) => {
        $(
            pub const $name: LazyLockTheme = LazyLockTheme::new(|| Theme::from_string(include_str!($path)).unwrap());
        )+
    };
}

pub struct LazyLockTheme(LazyLock<Theme>);

impl LazyLockTheme {
    #[inline(always)]
    const fn new(f: fn() -> Theme) -> Self {
        Self(LazyLock::new(f))
    }
}

impl Deref for LazyLockTheme {
    type Target = Theme;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LazyLockTheme {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Theme> for LazyLockTheme {
    fn as_ref(&self) -> &Theme {
        &self.0
    }
}

impl Theme {
    generate_builtin_themes!(["../../themes/default.json", DEFAULT]);

    fn from_string<S: AsRef<str>>(str: S) -> Result<Theme, serde_json::Error> {
        serde_json::from_str(str.as_ref())
    }
}

impl Global for Theme {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeLayout {
    pub text: ThemeText,
    pub corner_radii: ThemeCornerRadii,
    pub size: ThemeSize,
    pub padding: ThemePadding,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeText {
    #[serde(deserialize_with = "de_pixels")]
    pub base_size: Pixels,
    pub default_font: ThemeFont,
    pub mono_font: ThemeFont,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeFont {
    #[serde(deserialize_with = "de_string_or_non_empty_list")]
    pub family: SmallVec<[SharedString; 1]>,
    #[serde(deserialize_with = "de_def_length")]
    pub line_height: DefiniteLength,
    pub sizes: ThemeTextSizes,
    pub weights: ThemeTextWeights,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeTextSizes {
    #[serde(deserialize_with = "de_abs_length")]
    pub heading_xl: AbsoluteLength,
    #[serde(deserialize_with = "de_abs_length")]
    pub heading_lg: AbsoluteLength,
    #[serde(deserialize_with = "de_abs_length")]
    pub heading_md: AbsoluteLength,
    #[serde(deserialize_with = "de_abs_length")]
    pub heading_sm: AbsoluteLength,
    #[serde(deserialize_with = "de_abs_length")]
    pub body: AbsoluteLength,
    #[serde(deserialize_with = "de_abs_length")]
    pub caption: AbsoluteLength,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeTextWeights {
    pub heading_xl: f32,
    pub heading_lg: f32,
    pub heading_md: f32,
    pub heading_sm: f32,
    pub body: f32,
    pub caption: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeCornerRadii {
    #[serde(deserialize_with = "de_pixels")]
    pub xl: Pixels,
    #[serde(deserialize_with = "de_pixels")]
    pub lg: Pixels,
    #[serde(deserialize_with = "de_pixels")]
    pub md: Pixels,
    #[serde(deserialize_with = "de_pixels")]
    pub sm: Pixels,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeSize {
    #[serde(deserialize_with = "de_pixels")]
    pub xl: Pixels,
    #[serde(deserialize_with = "de_pixels")]
    pub lg: Pixels,
    #[serde(deserialize_with = "de_pixels")]
    pub md: Pixels,
    #[serde(deserialize_with = "de_pixels")]
    pub sm: Pixels,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemePadding {
    #[serde(deserialize_with = "de_pixels")]
    pub xl: Pixels,
    #[serde(deserialize_with = "de_pixels")]
    pub lg: Pixels,
    #[serde(deserialize_with = "de_pixels")]
    pub md: Pixels,
    #[serde(deserialize_with = "de_pixels")]
    pub sm: Pixels,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub struct ThemeVariants {
    #[serde(deserialize_with = "de_variants")]
    pub variants: SmallVec<[ThemeVariant; 2]>,
}

impl ThemeVariants {
    pub fn active(&self, cx: &App) -> &ThemeVariant {
        &self.variants[cx.global::<ActiveVariantId>().0]
    }
}

pub struct ActiveVariantId(pub usize);

impl gpui::Global for ActiveVariantId {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeVariant {
    pub kind: ThemeVariantKind,
    pub colors: ThemeColors,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ThemeVariantKind {
    Dark,
    Light,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeColors {
    pub background: ThemeBackgroundColors,
    pub accent: ThemeAccentColors,
    pub text: ThemeTextColors,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeBackgroundColors {
    pub primary: Rgba,
    pub secondary: Rgba,
    pub tertiary: Rgba,
    pub quaternary: Rgba,
    pub quinary: Rgba,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeAccentColors {
    pub primary: Rgba,
    pub constructive: Rgba,
    pub destructive: Rgba,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeTextColors {
    pub primary: Rgba,
    pub secondary: Rgba,
}

impl ThemeTextColors {
    pub fn all(&self) -> (Rgba, Rgba) {
        (self.primary, self.secondary)
    }
}
