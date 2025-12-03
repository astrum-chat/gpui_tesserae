use gpui::{AbsoluteLength, DefiniteLength, Pixels, Window, px};

pub trait PixelsExt {
    /// Calculates the top and bottom padding needed in order for
    /// the height of an element to reach this px value.
    fn padding_needed_for_height(
        &self,
        window: &Window,
        text_size: AbsoluteLength,
        line_height: DefiniteLength,
    ) -> Pixels;
}

impl PixelsExt for Pixels {
    fn padding_needed_for_height(
        &self,
        window: &Window,
        text_size: AbsoluteLength,
        line_height: DefiniteLength,
    ) -> Pixels {
        let text_size = match text_size {
            AbsoluteLength::Pixels(text_size) => text_size,
            AbsoluteLength::Rems(text_size) => text_size.to_pixels(window.rem_size()),
        }
        .to_f64() as f32;

        let line_height = match line_height {
            DefiniteLength::Absolute(line_height) => {
                line_height.to_pixels(window.rem_size()).to_f64() as f32
            }

            DefiniteLength::Fraction(frac) => text_size * frac,
        };

        return px((self.to_f64() as f32 - line_height) / 2.).into();
    }
}
