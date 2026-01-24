use gpui::App;

use crate::theme::Theme;

/// Extension trait for accessing and modifying the global theme.
pub trait ThemeExt {
    /// Changes the theme.
    fn set_theme<T: AsRef<Theme>>(&mut self, theme: T);

    /// Gets an immutable reference to the theme.
    fn get_theme(&self) -> &Theme;
}

impl ThemeExt for App {
    fn set_theme<T: AsRef<Theme>>(&mut self, theme: T) {
        self.set_global::<Theme>(theme.as_ref().clone())
    }

    fn get_theme(&self) -> &Theme {
        self.global()
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use crate::theme::schema::ActiveVariantId;
    use gpui::TestAppContext;

    #[gpui::test]
    fn test_set_and_get_theme(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);
            let theme = cx.get_theme();
            assert!(!theme.name.is_empty(), "Theme should have a name");
        });
    }

    #[gpui::test]
    fn test_theme_has_layout(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);
            let theme = cx.get_theme();

            assert!(
                theme.layout.size.sm > gpui::px(0.),
                "Size sm should be positive"
            );
            assert!(
                theme.layout.size.md > gpui::px(0.),
                "Size md should be positive"
            );
            assert!(
                theme.layout.size.lg > gpui::px(0.),
                "Size lg should be positive"
            );
            assert!(
                theme.layout.size.xl > gpui::px(0.),
                "Size xl should be positive"
            );
        });
    }

    #[gpui::test]
    fn test_theme_has_padding(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);
            let theme = cx.get_theme();

            assert!(
                theme.layout.padding.sm >= gpui::px(0.),
                "Padding sm should be non-negative"
            );
            assert!(
                theme.layout.padding.md >= gpui::px(0.),
                "Padding md should be non-negative"
            );
            assert!(
                theme.layout.padding.lg >= gpui::px(0.),
                "Padding lg should be non-negative"
            );
            assert!(
                theme.layout.padding.xl >= gpui::px(0.),
                "Padding xl should be non-negative"
            );
        });
    }

    #[gpui::test]
    fn test_theme_has_corner_radii(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);
            let theme = cx.get_theme();

            assert!(
                theme.layout.corner_radii.sm >= gpui::px(0.),
                "Corner radii sm should be non-negative"
            );
            assert!(
                theme.layout.corner_radii.md >= gpui::px(0.),
                "Corner radii md should be non-negative"
            );
            assert!(
                theme.layout.corner_radii.lg >= gpui::px(0.),
                "Corner radii lg should be non-negative"
            );
            assert!(
                theme.layout.corner_radii.xl >= gpui::px(0.),
                "Corner radii xl should be non-negative"
            );
        });
    }

    #[gpui::test]
    fn test_theme_has_variants(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);
            cx.set_global(ActiveVariantId(0));
            let theme = cx.get_theme();

            assert!(
                !theme.variants.variants.is_empty(),
                "Theme should have at least one variant"
            );

            let _active = theme.variants.active(cx);
        });
    }

    #[gpui::test]
    fn test_theme_variant_has_colors(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);
            cx.set_global(ActiveVariantId(0));
            let theme = cx.get_theme();
            let active = theme.variants.active(cx);

            let (primary, secondary) = active.colors.text.all();
            assert!(primary.a > 0.0, "Primary text color should be visible");
            assert!(secondary.a > 0.0, "Secondary text color should be visible");
        });
    }

    #[gpui::test]
    fn test_theme_variant_has_accent_colors(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);
            cx.set_global(ActiveVariantId(0));
            let theme = cx.get_theme();
            let active = theme.variants.active(cx);

            assert!(
                active.colors.accent.primary.a > 0.0,
                "Primary accent color should be visible"
            );
            assert!(
                active.colors.accent.destructive.a > 0.0,
                "Destructive accent color should be visible"
            );
            assert!(
                active.colors.accent.constructive.a > 0.0,
                "Constructive accent color should be visible"
            );
        });
    }

    #[gpui::test]
    fn test_theme_as_ref(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let theme = Theme::DEFAULT;
            let theme_ref: &Theme = theme.as_ref();
            assert!(!theme_ref.name.is_empty(), "Theme ref should have a name");

            cx.set_theme(Theme::DEFAULT);
            let retrieved = cx.get_theme();
            assert_eq!(retrieved.name, theme.name, "Theme names should match");
        });
    }
}
