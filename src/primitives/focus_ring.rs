use std::time::Duration;

use gpui::{
    Corners, CornersRefinement, ElementId, FocusHandle, IntoElement, Pixels, RenderOnce, Rgba,
    ease_out_quint, prelude::*, px,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_tesserae_theme::ThemeExt;
use gpui_transitions::{Transition, TransitionExt};

use crate::utils::RgbaExt;

const SIZE_SCALE_FACTOR: f32 = 8.;

#[derive(IntoElement)]
pub struct FocusRing {
    id: ElementId,
    focus_handle: FocusHandle,
    corner_radii: Corners<Pixels>,
    border_color: Option<Rgba>,
}

impl FocusRing {
    pub fn new(id: impl Into<ElementId>, focus_handle: FocusHandle) -> Self {
        Self {
            id: id.into(),
            focus_handle: focus_handle,
            corner_radii: Corners::all(px(8.)),
            border_color: None,
        }
    }

    pub fn focus_handle(mut self, focus_handle: FocusHandle) -> Self {
        self.focus_handle = focus_handle;
        self
    }

    pub fn border_color(mut self, border_color: impl Into<Rgba>) -> Self {
        self.border_color = Some(border_color.into());
        self
    }

    pub fn rounded(mut self, rounded: impl Into<Pixels>) -> Self {
        self.corner_radii = Corners::all(rounded.into());
        self
    }
}

impl RenderOnce for FocusRing {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let border_color = self
            .border_color
            .unwrap_or_else(|| cx.get_theme().variants.active().colors.accent.primary);

        let is_focused = self.focus_handle.is_focused(window) as u8 as f32;

        let ring_progress_state = Transition::new(
            self.id.clone(),
            window,
            cx,
            Duration::from_millis(365),
            |_window, _cx| is_focused,
        )
        .with_easing(ease_out_quint());

        let changed = ring_progress_state.set(cx, is_focused);
        if changed {
            cx.notify(ring_progress_state.entity_id());
        }

        squircle()
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .border(px(3.))
            .border_outside()
            .with_transitions(ring_progress_state, move |_cx, this, delta| {
                let size_factor = (1. - delta) * SIZE_SCALE_FACTOR;

                this.inset(px(-size_factor))
                    .border_color(border_color.alpha(border_color.a * delta * 0.3))
                    .map(|mut this| {
                        this.outer_style().corner_radii =
                            add_to_corner_radii(&self.corner_radii, px(size_factor + 1.));
                        this
                    })
            })
    }
}

fn add_to_corner_radii(corner_radii: &Corners<Pixels>, num: Pixels) -> CornersRefinement<Pixels> {
    CornersRefinement {
        top_left: Some(corner_radii.top_left + num),
        top_right: Some(corner_radii.top_right + num),
        bottom_right: Some(corner_radii.bottom_right + num),
        bottom_left: Some(corner_radii.bottom_left + num),
    }
}
