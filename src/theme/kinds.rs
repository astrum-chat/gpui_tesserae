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

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use crate::theme::{Theme, ThemeExt};
    use gpui::TestAppContext;

    #[gpui::test]
    fn test_theme_text_size_kind_variants(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            let _ = ThemeTextSizeKind::Xl.resolve(cx);
            let _ = ThemeTextSizeKind::Lg.resolve(cx);
            let _ = ThemeTextSizeKind::Md.resolve(cx);
            let _ = ThemeTextSizeKind::Sm.resolve(cx);
            let _ = ThemeTextSizeKind::Body.resolve(cx);
            let _ = ThemeTextSizeKind::Caption.resolve(cx);
        });
    }

    #[gpui::test]
    fn test_theme_layout_size_kind_variants(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            let _ = ThemeLayoutSizeKind::Xl.resolve(cx);
            let _ = ThemeLayoutSizeKind::Lg.resolve(cx);
            let _ = ThemeLayoutSizeKind::Md.resolve(cx);
            let _ = ThemeLayoutSizeKind::Sm.resolve(cx);
        });
    }

    #[gpui::test]
    fn test_theme_layout_size_kind_corner_radii(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            assert!(matches!(
                ThemeLayoutSizeKind::Xl.corner_radii(),
                ThemeLayoutCornerRadiiKind::Xl
            ));
            assert!(matches!(
                ThemeLayoutSizeKind::Lg.corner_radii(),
                ThemeLayoutCornerRadiiKind::Lg
            ));
            assert!(matches!(
                ThemeLayoutSizeKind::Md.corner_radii(),
                ThemeLayoutCornerRadiiKind::Md
            ));
            assert!(matches!(
                ThemeLayoutSizeKind::Sm.corner_radii(),
                ThemeLayoutCornerRadiiKind::Sm
            ));
        });
    }

    #[gpui::test]
    fn test_theme_layout_padding_kind_variants(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            let _ = ThemeLayoutPaddingKind::Xl.resolve(cx);
            let _ = ThemeLayoutPaddingKind::Lg.resolve(cx);
            let _ = ThemeLayoutPaddingKind::Md.resolve(cx);
            let _ = ThemeLayoutPaddingKind::Sm.resolve(cx);
        });
    }

    #[gpui::test]
    fn test_theme_layout_corner_radii_kind_variants(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            let _ = ThemeLayoutCornerRadiiKind::Xl.resolve(cx);
            let _ = ThemeLayoutCornerRadiiKind::Lg.resolve(cx);
            let _ = ThemeLayoutCornerRadiiKind::Md.resolve(cx);
            let _ = ThemeLayoutCornerRadiiKind::Sm.resolve(cx);
        });
    }

    #[gpui::test]
    fn test_theme_background_kind_variants(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            let _ = ThemeBackgroundKind::Primary.resolve(cx);
            let _ = ThemeBackgroundKind::Secondary.resolve(cx);
            let _ = ThemeBackgroundKind::Tertiary.resolve(cx);
            let _ = ThemeBackgroundKind::Quaternary.resolve(cx);
            let _ = ThemeBackgroundKind::Quinary.resolve(cx);
        });
    }

    #[gpui::test]
    fn test_theme_layer_kind_variants(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            let _ = ThemeLayerKind::Primary.resolve(cx);
            let _ = ThemeLayerKind::Secondary.resolve(cx);
            let _ = ThemeLayerKind::Tertiary.resolve(cx);
            let _ = ThemeLayerKind::Quaternary.resolve(cx);
        });
    }

    #[gpui::test]
    fn test_theme_layer_kind_next(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            assert!(matches!(
                ThemeLayerKind::Primary.next(),
                ThemeBackgroundKind::Secondary
            ));
            assert!(matches!(
                ThemeLayerKind::Secondary.next(),
                ThemeBackgroundKind::Tertiary
            ));
            assert!(matches!(
                ThemeLayerKind::Tertiary.next(),
                ThemeBackgroundKind::Quaternary
            ));
            assert!(matches!(
                ThemeLayerKind::Quaternary.next(),
                ThemeBackgroundKind::Quinary
            ));
        });
    }

    #[gpui::test]
    fn test_theme_layer_kind_into_background_kind(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let bg: ThemeBackgroundKind = ThemeLayerKind::Primary.into();
            assert!(matches!(bg, ThemeBackgroundKind::Primary));

            let bg: ThemeBackgroundKind = ThemeLayerKind::Secondary.into();
            assert!(matches!(bg, ThemeBackgroundKind::Secondary));

            let bg: ThemeBackgroundKind = ThemeLayerKind::Tertiary.into();
            assert!(matches!(bg, ThemeBackgroundKind::Tertiary));

            let bg: ThemeBackgroundKind = ThemeLayerKind::Quaternary.into();
            assert!(matches!(bg, ThemeBackgroundKind::Quaternary));
        });
    }

    #[gpui::test]
    fn test_size_ordering(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            let sm = ThemeLayoutSizeKind::Sm.resolve(cx);
            let md = ThemeLayoutSizeKind::Md.resolve(cx);
            let lg = ThemeLayoutSizeKind::Lg.resolve(cx);
            let xl = ThemeLayoutSizeKind::Xl.resolve(cx);

            assert!(sm <= md, "Sm should be <= Md");
            assert!(md <= lg, "Md should be <= Lg");
            assert!(lg <= xl, "Lg should be <= Xl");
        });
    }

    #[gpui::test]
    fn test_padding_ordering(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            let sm = ThemeLayoutPaddingKind::Sm.resolve(cx);
            let md = ThemeLayoutPaddingKind::Md.resolve(cx);
            let lg = ThemeLayoutPaddingKind::Lg.resolve(cx);
            let xl = ThemeLayoutPaddingKind::Xl.resolve(cx);

            assert!(sm <= md, "Sm should be <= Md");
            assert!(md <= lg, "Md should be <= Lg");
            assert!(lg <= xl, "Lg should be <= Xl");
        });
    }

    #[gpui::test]
    fn test_corner_radii_ordering(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            let sm = ThemeLayoutCornerRadiiKind::Sm.resolve(cx);
            let md = ThemeLayoutCornerRadiiKind::Md.resolve(cx);
            let lg = ThemeLayoutCornerRadiiKind::Lg.resolve(cx);
            let xl = ThemeLayoutCornerRadiiKind::Xl.resolve(cx);

            assert!(sm <= md, "Sm should be <= Md");
            assert!(md <= lg, "Md should be <= Lg");
            assert!(lg <= xl, "Lg should be <= Xl");
        });
    }
}
