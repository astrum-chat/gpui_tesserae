use gpui::{ColorSpace, linear_color_stop, linear_gradient};
use gpui_squircle::{Squircle, SquircleStyled};

use crate::utils::rgb_a;

pub trait SquircleExt {
    fn border_highlight_color(self, opacity: f32) -> Self;
}

impl SquircleExt for Squircle {
    fn border_highlight_color(self, opacity: f32) -> Self {
        self.border_color(
            linear_gradient(
                180.,
                linear_color_stop(rgb_a(0xE8E4FF, opacity), 0.),
                linear_color_stop(rgb_a(0x110F15, opacity), 1.),
            )
            .color_space(ColorSpace::Oklab),
        )
    }
}
