use gpui::Rgba;

/// Creates an RGBA color from a hex value and alpha component.
pub fn rgb_a(hex: u32, a: f32) -> Rgba {
    let [_, r, g, b] = hex.to_be_bytes().map(|b| (b as f32) / 255.0);
    Rgba { r, g, b, a }
}

/// Extension trait for modifying RGBA colors.
pub trait RgbaExt {
    /// Returns a new color with the specified alpha value.
    fn alpha(self, alpha: f32) -> Self;
}

impl RgbaExt for Rgba {
    fn alpha(mut self, alpha: f32) -> Self {
        self.a = alpha;
        self
    }
}
