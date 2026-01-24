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

/// Complete theme configuration including layout dimensions and color variants.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Theme {
    /// Display name of the theme.
    pub name: SharedString,
    /// Layout dimensions: sizes, spacing, and typography.
    pub layout: ThemeLayout,
    /// Color variants, typically dark and light modes.
    pub variants: ThemeVariants,
}

macro_rules! generate_builtin_themes {
    ( $( [$path:literal, $name:ident] ),+ ) => {
        $(
            #[doc = concat!("Built-in theme loaded from `", $path, "`.")]
            pub const $name: LazyLockTheme = LazyLockTheme::new(|| Theme::from_string(include_str!($path)).unwrap());
        )+
    };
}

/// A lazily-initialized theme that defers parsing until first access.
pub struct LazyLockTheme(LazyLock<Theme>);

impl LazyLockTheme {
    /// Creates a new lazy theme with the given initialization function.
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

/// Layout configuration containing dimensions and typography settings.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeLayout {
    /// Typography settings.
    pub text: ThemeText,
    /// Border radius values for different size scales.
    pub corner_radii: ThemeCornerRadii,
    /// Component size values.
    pub size: ThemeSize,
    /// Spacing values.
    pub padding: ThemePadding,
}

/// Typography configuration including fonts and sizing.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeText {
    /// Base font size that other sizes are relative to.
    #[serde(deserialize_with = "de_pixels")]
    pub base_size: Pixels,
    /// Default font for body text.
    pub default_font: ThemeFont,
    /// Monospace font for code.
    pub mono_font: ThemeFont,
}

/// Font family and sizing configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeFont {
    /// Font family stack, first available is used.
    #[serde(deserialize_with = "de_string_or_non_empty_list")]
    pub family: SmallVec<[SharedString; 1]>,
    /// Line height multiplier.
    #[serde(deserialize_with = "de_def_length")]
    pub line_height: DefiniteLength,
    /// Text sizes for different contexts.
    pub sizes: ThemeTextSizes,
    /// Font weights for different contexts.
    pub weights: ThemeTextWeights,
}

/// Text sizes for headings, body, and caption text.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeTextSizes {
    /// Extra large heading size.
    #[serde(deserialize_with = "de_abs_length")]
    pub heading_xl: AbsoluteLength,
    /// Large heading size.
    #[serde(deserialize_with = "de_abs_length")]
    pub heading_lg: AbsoluteLength,
    /// Medium heading size.
    #[serde(deserialize_with = "de_abs_length")]
    pub heading_md: AbsoluteLength,
    /// Small heading size.
    #[serde(deserialize_with = "de_abs_length")]
    pub heading_sm: AbsoluteLength,
    /// Body text size.
    #[serde(deserialize_with = "de_abs_length")]
    pub body: AbsoluteLength,
    /// Caption text size.
    #[serde(deserialize_with = "de_abs_length")]
    pub caption: AbsoluteLength,
}

/// Font weights for different text contexts.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeTextWeights {
    /// Extra large heading weight.
    pub heading_xl: f32,
    /// Large heading weight.
    pub heading_lg: f32,
    /// Medium heading weight.
    pub heading_md: f32,
    /// Small heading weight.
    pub heading_sm: f32,
    /// Body text weight.
    pub body: f32,
    /// Caption text weight.
    pub caption: f32,
}

/// Border radius values for different size scales.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeCornerRadii {
    /// Extra large corner radius.
    #[serde(deserialize_with = "de_pixels")]
    pub xl: Pixels,
    /// Large corner radius.
    #[serde(deserialize_with = "de_pixels")]
    pub lg: Pixels,
    /// Medium corner radius.
    #[serde(deserialize_with = "de_pixels")]
    pub md: Pixels,
    /// Small corner radius.
    #[serde(deserialize_with = "de_pixels")]
    pub sm: Pixels,
}

/// Component sizes for different scale options.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeSize {
    /// Extra large component size.
    #[serde(deserialize_with = "de_pixels")]
    pub xl: Pixels,
    /// Large component size.
    #[serde(deserialize_with = "de_pixels")]
    pub lg: Pixels,
    /// Medium component size.
    #[serde(deserialize_with = "de_pixels")]
    pub md: Pixels,
    /// Small component size.
    #[serde(deserialize_with = "de_pixels")]
    pub sm: Pixels,
}

/// Padding values for different spacing scales.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemePadding {
    /// Extra large padding.
    #[serde(deserialize_with = "de_pixels")]
    pub xl: Pixels,
    /// Large padding.
    #[serde(deserialize_with = "de_pixels")]
    pub lg: Pixels,
    /// Medium padding.
    #[serde(deserialize_with = "de_pixels")]
    pub md: Pixels,
    /// Small padding.
    #[serde(deserialize_with = "de_pixels")]
    pub sm: Pixels,
}

/// Container for theme color variants (e.g., dark and light modes).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub struct ThemeVariants {
    /// List of available variants.
    #[serde(deserialize_with = "de_variants")]
    pub variants: SmallVec<[ThemeVariant; 2]>,
}

impl ThemeVariants {
    /// Returns the currently active variant based on global state.
    ///
    /// Falls back to the first variant (index 0) if no active variant is set.
    pub fn active(&self, cx: &App) -> &ThemeVariant {
        &self.variants[cx
            .try_global::<ActiveVariantId>()
            .unwrap_or(&ActiveVariantId(0))
            .0]
    }
}

/// Global state tracking which theme variant is currently active.
///
/// The inner value is an index into `ThemeVariants::variants`.
pub struct ActiveVariantId(pub usize);

impl gpui::Global for ActiveVariantId {}

/// A single theme variant containing its kind and color palette.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeVariant {
    /// Whether this is a dark or light variant.
    pub kind: ThemeVariantKind,
    /// Color palette for this variant.
    pub colors: ThemeColors,
}

/// Indicates whether a theme variant uses dark or light colors.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ThemeVariantKind {
    /// Dark mode with light text on dark backgrounds.
    Dark,
    /// Light mode with dark text on light backgrounds.
    Light,
}

/// Complete color palette for a theme variant.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeColors {
    /// Background colors for surfaces and containers.
    pub background: ThemeBackgroundColors,
    /// Accent colors for interactive elements and status.
    pub accent: ThemeAccentColors,
    /// Text colors for content.
    pub text: ThemeTextColors,
}

/// Background colors for surfaces at different elevation levels.
///
/// Colors progress from primary (lowest) to quinary (highest emphasis).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeBackgroundColors {
    /// Base background for main surfaces.
    pub primary: Rgba,
    /// Slightly elevated or grouped content.
    pub secondary: Rgba,
    /// Further elevated elements.
    pub tertiary: Rgba,
    /// High emphasis backgrounds.
    pub quaternary: Rgba,
    /// Highest emphasis backgrounds.
    pub quinary: Rgba,
}

/// Accent colors for interactive elements and semantic meaning.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeAccentColors {
    /// Default accent for buttons and links.
    pub primary: Rgba,
    /// Positive actions like save or confirm.
    pub constructive: Rgba,
    /// Negative actions like delete or error states.
    pub destructive: Rgba,
}

/// Text colors for different content hierarchies.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeTextColors {
    /// Main body text color.
    pub primary: Rgba,
    /// De-emphasized or supplementary text.
    pub secondary: Rgba,
}

impl ThemeTextColors {
    /// Returns both text colors as a tuple for convenience.
    pub fn all(&self) -> (Rgba, Rgba) {
        (self.primary, self.secondary)
    }
}
