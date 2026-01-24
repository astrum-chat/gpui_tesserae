use gpui::{ColorSpace, linear_color_stop, linear_gradient};
use gpui_squircle::{Squircle, SquircleStyled};

use crate::utils::rgb_a;

/// Extension trait for squircle styling.
pub trait SquircleExt {
    /// Applies a gradient border highlight effect with the given alpha.
    fn border_highlight(self, alpha: f32) -> Self;
}

impl SquircleExt for Squircle {
    fn border_highlight(self, alpha: f32) -> Self {
        self.border_color(
            linear_gradient(
                180.,
                linear_color_stop(rgb_a(0xE8E4FF, alpha), 0.),
                linear_color_stop(rgb_a(0x110F15, alpha), 1.),
            )
            .color_space(ColorSpace::Oklab),
        )
    }
}
