use gpui::{Pixels, Window, px};

/// Extension trait for [`Window`] providing pixel-perfect rounding utilities.
pub trait WindowExt {
    /// Rounds a pixel value to the nearest display-aligned increment based on the
    /// window's scale factor. On 2x+ displays the increment is 0.5px; on 1x displays
    /// it is 1px. Use this to snap layout measurements (e.g. line heights) to values
    /// that land on exact device pixels and avoid sub-pixel blurriness.
    fn round(&self, px: impl Into<Pixels>) -> Pixels;
}

impl WindowExt for Window {
    fn round(&self, value: impl Into<Pixels>) -> Pixels {
        let scale_factor = self.scale_factor();

        let increment = if scale_factor >= 2.0 { 0.5 } else { 1.0 };
        let value = value.into().to_f64() as f32;
        px((value / increment).round() * increment)
    }
}
