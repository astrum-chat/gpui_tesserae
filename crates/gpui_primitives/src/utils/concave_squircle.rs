//! Concave squircle corner path generation.
//!
//! Produces smooth concave (interior) corner patches. The curve scoops inward
//! from one axis-aligned edge to the other, filling the triangular region between
//! the two straight edges and the concave curve.
//!
//! `corner_smoothing` controls how "squircle-like" the concave curve is:
//! - 0.0 = quarter-circle arc (standard rounded corner, gentle inward curve)
//! - 1.0 = maximally squircle - the curve scoops inward more aggressively,
//!   approaching a right-angle crease toward the origin
//!
//! The shape for a TopRight corner (sx=+1, sy=-1) looks like:
//!
//! ```text
//!        (0, -r)
//!           |  .
//!           | .   ← concave curve (scoops toward origin)
//!           |.
//!   (0,0) ------ (r, 0)
//! ```

use gpui::{Path, PathBuilder, Pixels, Point, point, px};

/// Which corner orientation to draw.
#[derive(Clone, Copy)]
pub enum ConcaveCorner {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

/// Builds a concave squircle corner path.
///
/// `rx` is the horizontal extent, `ry` is the vertical extent. When the step between
/// adjacent lines is smaller than the full patch size, `rx` is clamped so the patch
/// doesn't extend past the adjacent line's selection edge.
///
/// Returns `None` if the path can't be built (e.g. zero radius).
pub fn build_concave_squircle_path(
    cx: Pixels,
    cy: Pixels,
    rx: Pixels,
    ry: Pixels,
    corner: ConcaveCorner,
    corner_smoothing: f32,
) -> Option<Path<Pixels>> {
    let rx_f = rx.to_f64() as f32;
    let ry_f = ry.to_f64() as f32;
    if rx_f <= 0.0 || ry_f <= 0.0 {
        return None;
    }

    let mut builder = PathBuilder::fill();

    let (sx, sy) = match corner {
        ConcaveCorner::TopRight => (1.0_f32, -1.0_f32),
        ConcaveCorner::TopLeft => (-1.0, -1.0),
        ConcaveCorner::BottomRight => (1.0, 1.0),
        ConcaveCorner::BottomLeft => (-1.0, 1.0),
    };

    build_corner(&mut builder, cx, cy, rx_f, ry_f, sx, sy, corner_smoothing);

    builder.build().ok()
}

fn pt(cx: Pixels, cy: Pixels, dx: f32, dy: f32) -> Point<Pixels> {
    point(cx + px(dx), cy + px(dy))
}

/// Builds a concave corner path.
///
/// The filled region is the triangle `(0,0) → (rx,0) → (0,ry)` with the hypotenuse
/// replaced by a concave curve that scoops toward the origin. `rx` controls the
/// horizontal extent, `ry` controls the vertical extent.
///
/// `sx`/`sy` flip the orientation:
///   TopRight:    sx=+1, sy=-1  (extends right and up)
///   TopLeft:     sx=-1, sy=-1  (extends left and up)
///   BottomRight: sx=+1, sy=+1  (extends right and down)
///   BottomLeft:  sx=-1, sy=+1  (extends left and down)
///
/// The cubic bezier goes from `(rx, 0)` to `(0, ry)` with control points on the
/// axes between the endpoints and the origin. The control distance `kx`/`ky` from
/// each endpoint determines the curvature:
///
/// - `k = r * KAPPA` → quarter-circle arc (smoothing = 0)
/// - `k > r * KAPPA` → more aggressive inward scoop (smoothing → 1)
///
/// As smoothing increases, `k` grows from `r * KAPPA` toward `r`, pulling the
/// control points closer to the origin and making the curve scoop inward more.
/// At `k = r`, the controls are at the origin and the curve forms a sharp crease.
fn build_corner(
    builder: &mut PathBuilder,
    cx: Pixels,
    cy: Pixels,
    rx: f32,
    ry: f32,
    sx: f32,
    sy: f32,
    corner_smoothing: f32,
) {
    let smoothing = corner_smoothing.clamp(0.0, 1.0);

    // KAPPA ≈ 0.5523 gives a quarter-circle. At smoothing=1.0 we want k close to r
    // (sharp crease), but not fully r (which would be a cusp). Lerp from KAPPA to
    // a max of ~0.9 for a strong but smooth scoop.
    const KAPPA: f32 = 0.5522847498;
    const MAX_K_FACTOR: f32 = 0.92;
    let k_factor = KAPPA + (MAX_K_FACTOR - KAPPA) * smoothing;
    let kx = rx * k_factor;
    let ky = ry * k_factor;

    // Path: origin → horizontal edge point → concave curve → vertical edge point → close
    builder.move_to(pt(cx, cy, 0.0, 0.0));
    builder.line_to(pt(cx, cy, sx * rx, 0.0));

    // Cubic bezier from (rx, 0) to (0, ry), curving toward origin.
    // ctrl1 at (rx-kx, 0): on horizontal axis, between endpoint and origin
    // ctrl2 at (0, ry-ky): on vertical axis, between endpoint and origin
    // Larger k → controls closer to origin → deeper inward scoop
    builder.cubic_bezier_to(
        pt(cx, cy, 0.0, sy * ry),        // endpoint: vertical edge
        pt(cx, cy, sx * (rx - kx), 0.0), // ctrl1: on horizontal axis
        pt(cx, cy, 0.0, sy * (ry - ky)), // ctrl2: on vertical axis
    );

    builder.close();
}
