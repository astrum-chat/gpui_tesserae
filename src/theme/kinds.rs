use enum_assoc::Assoc;
use gpui::App;

use crate::theme::ThemeExt;

/// An enum containing all of the available text size options.
#[derive(Assoc)]
#[func(pub fn resolve(&self, cx: &App) -> gpui::AbsoluteLength)]
pub enum ThemeTextSizeKind {
    #[assoc(resolve = cx.get_theme().layout.text.default_font.sizes.heading_xl)]
    Xl,
    #[assoc(resolve = cx.get_theme().layout.text.default_font.sizes.heading_lg)]
    Lg,
    #[assoc(resolve = cx.get_theme().layout.text.default_font.sizes.heading_md)]
    Md,
    #[assoc(resolve = cx.get_theme().layout.text.default_font.sizes.heading_sm)]
    Sm,
    #[assoc(resolve = cx.get_theme().layout.text.default_font.sizes.body)]
    Body,
    #[assoc(resolve = cx.get_theme().layout.text.default_font.sizes.caption)]
    Caption,
}

/// An enum containing all of the available size options.
#[derive(Assoc)]
#[func(pub fn resolve(&self, cx: &App) -> gpui::Pixels)]
#[func(pub fn corner_radii(&self) -> ThemeLayoutCornerRadiiKind)]
pub enum ThemeLayoutSizeKind {
    #[assoc(resolve = cx.get_theme().layout.size.xl)]
    #[assoc(corner_radii = ThemeLayoutCornerRadiiKind::Xl)]
    Xl,
    #[assoc(resolve = cx.get_theme().layout.size.lg)]
    #[assoc(corner_radii = ThemeLayoutCornerRadiiKind::Lg)]
    Lg,
    #[assoc(resolve = cx.get_theme().layout.size.md)]
    #[assoc(corner_radii = ThemeLayoutCornerRadiiKind::Md)]
    Md,
    #[assoc(resolve = cx.get_theme().layout.size.sm)]
    #[assoc(corner_radii = ThemeLayoutCornerRadiiKind::Sm)]
    Sm,
}

/// An enum containing all of the available padding options.
#[derive(Assoc)]
#[func(pub fn resolve(&self, cx: &App) -> gpui::Pixels)]
pub enum ThemeLayoutPaddingKind {
    #[assoc(resolve = cx.get_theme().layout.padding.xl)]
    Xl,
    #[assoc(resolve = cx.get_theme().layout.padding.lg)]
    Lg,
    #[assoc(resolve = cx.get_theme().layout.padding.md)]
    Md,
    #[assoc(resolve = cx.get_theme().layout.padding.sm)]
    Sm,
}

/// An enum containing all of the available corner radius options.
#[derive(Assoc)]
#[func(pub fn resolve(&self, cx: &App) -> gpui::Pixels)]
pub enum ThemeLayoutCornerRadiiKind {
    #[assoc(resolve = cx.get_theme().layout.corner_radii.xl)]
    Xl,
    #[assoc(resolve = cx.get_theme().layout.corner_radii.lg)]
    Lg,
    #[assoc(resolve = cx.get_theme().layout.corner_radii.md)]
    Md,
    #[assoc(resolve = cx.get_theme().layout.corner_radii.sm)]
    Sm,
}

/// An enum containing all of the available background color options.
#[derive(Assoc)]
#[func(pub fn resolve(&self, cx: &App) -> gpui::Rgba)]
pub enum ThemeBackgroundKind {
    #[assoc(resolve = cx.get_theme().variants.active(cx).colors.background.primary)]
    Primary,
    #[assoc(resolve = cx.get_theme().variants.active(cx).colors.background.secondary)]
    Secondary,
    #[assoc(resolve = cx.get_theme().variants.active(cx).colors.background.tertiary)]
    Tertiary,
    #[assoc(resolve = cx.get_theme().variants.active(cx).colors.background.quaternary)]
    Quaternary,
    #[assoc(resolve = cx.get_theme().variants.active(cx).colors.background.quinary)]
    Quinary,
}

/// An enum containing all of the available background layers.
#[derive(Assoc)]
#[func(pub fn resolve(&self, cx: &App) -> gpui::Rgba)]
#[func(pub fn next(&self) -> ThemeBackgroundKind)]
pub enum ThemeLayerKind {
    #[assoc(resolve = cx.get_theme().variants.active(cx).colors.background.primary)]
    #[assoc(next = ThemeBackgroundKind::Secondary)]
    Primary,
    #[assoc(resolve = cx.get_theme().variants.active(cx).colors.background.secondary)]
    #[assoc(next = ThemeBackgroundKind::Tertiary)]
    Secondary,
    #[assoc(resolve = cx.get_theme().variants.active(cx).colors.background.tertiary)]
    #[assoc(next = ThemeBackgroundKind::Quaternary)]
    Tertiary,
    #[assoc(resolve = cx.get_theme().variants.active(cx).colors.background.quaternary)]
    #[assoc(next = ThemeBackgroundKind::Quinary)]
    Quaternary,
}

impl Into<ThemeBackgroundKind> for ThemeLayerKind {
    fn into(self) -> ThemeBackgroundKind {
        match self {
            Self::Primary => ThemeBackgroundKind::Primary,
            Self::Secondary => ThemeBackgroundKind::Secondary,
            Self::Tertiary => ThemeBackgroundKind::Tertiary,
            Self::Quaternary => ThemeBackgroundKind::Quaternary,
        }
    }
}
