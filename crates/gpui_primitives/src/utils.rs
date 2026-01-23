use gpui::Rgba;

pub fn rgb_a(hex: u32, a: f32) -> Rgba {
    let [_, r, g, b] = hex.to_be_bytes().map(|b| (b as f32) / 255.0);
    Rgba { r, g, b, a }
}
