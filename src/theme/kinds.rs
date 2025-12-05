use gpui_tesserae_macros::IntoThemeField;

/// An enum containing all of the available text size options.
#[derive(IntoThemeField)]
#[field(gpui::AbsoluteLength)]
pub enum ThemeTextSizeKind {
    #[theme(layout.text.default_font.sizes.heading_xl)]
    Xl,
    #[theme(layout.text.default_font.sizes.heading_lg)]
    Lg,
    #[theme(layout.text.default_font.sizes.heading_md)]
    Md,
    #[theme(layout.text.default_font.sizes.heading_sm)]
    Sm,
    #[theme(layout.text.default_font.sizes.body)]
    Body,
    #[theme(layout.text.default_font.sizes.caption)]
    Caption,
}

/// An enum containing all of the available size options.
#[derive(IntoThemeField)]
#[field(gpui::Pixels)]
pub enum ThemeLayoutSizeKind {
    #[theme(layout.size.xl)]
    Xl,
    #[theme(layout.size.lg)]
    Lg,
    #[theme(layout.size.md)]
    Md,
    #[theme(layout.size.sm)]
    Sm,
}

impl ThemeLayoutSizeKind {
    pub fn corner_radii(&self) -> ThemeLayoutCornerRadiiKind {
        match self {
            Self::Xl => ThemeLayoutCornerRadiiKind::Xl,
            Self::Lg => ThemeLayoutCornerRadiiKind::Lg,
            Self::Md => ThemeLayoutCornerRadiiKind::Md,
            Self::Sm => ThemeLayoutCornerRadiiKind::Sm,
        }
    }
}

/// An enum containing all of the available padding options.
#[derive(IntoThemeField)]
#[field(gpui::Pixels)]
pub enum ThemeLayoutPaddingKind {
    #[theme(layout.padding.xl)]
    Xl,
    #[theme(layout.padding.lg)]
    Lg,
    #[theme(layout.padding.md)]
    Md,
    #[theme(layout.padding.sm)]
    Sm,
}

/// An enum containing all of the available corner radius options.
#[derive(IntoThemeField)]
#[field(gpui::Pixels)]
pub enum ThemeLayoutCornerRadiiKind {
    #[theme(layout.corner_radii.xl)]
    Xl,
    #[theme(layout.corner_radii.lg)]
    Lg,
    #[theme(layout.corner_radii.md)]
    Md,
    #[theme(layout.corner_radii.sm)]
    Sm,
}

/// An enum containing all of the available background color options.
#[derive(IntoThemeField)]
#[field(gpui::Rgba)]
pub enum ThemeBackgroundKind {
    #[theme(variants.active().colors.background.primary)]
    Primary,
    #[theme(variants.active().colors.background.secondary)]
    Secondary,
    #[theme(variants.active().colors.background.tertiary)]
    Tertiary,
    #[theme(variants.active().colors.background.quaternary)]
    Quaternary,
    #[theme(variants.active().colors.background.quinary)]
    Quinary,
}

/// An enum containing all of the available background layers.
#[derive(IntoThemeField)]
#[field(gpui::Rgba)]
pub enum ThemeLayerKind {
    #[theme(variants.active().colors.background.primary)]
    Primary,
    #[theme(variants.active().colors.background.secondary)]
    Secondary,
    #[theme(variants.active().colors.background.tertiary)]
    Tertiary,
    #[theme(variants.active().colors.background.quaternary)]
    Quaternary,
}

impl ThemeLayerKind {
    pub fn next(&self) -> ThemeBackgroundKind {
        match self {
            Self::Primary => ThemeBackgroundKind::Secondary,
            Self::Secondary => ThemeBackgroundKind::Tertiary,
            Self::Tertiary => ThemeBackgroundKind::Quaternary,
            Self::Quaternary => ThemeBackgroundKind::Quinary,
        }
    }
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
