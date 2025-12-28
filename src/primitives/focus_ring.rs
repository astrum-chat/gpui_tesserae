use std::time::Duration;

use gpui::{
    CornersRefinement, ElementId, FocusHandle, IntoElement, Pixels, RenderOnce, Rgba,
    ease_out_quint, prelude::*, px,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::WindowUseTransition;

use crate::{theme::ThemeExt, utils::RgbaExt};

const SIZE_SCALE_FACTOR: f32 = 8.;

#[derive(IntoElement)]
pub struct FocusRing {
    id: ElementId,
    focus_handle: FocusHandle,
    pub corner_radii: CornersRefinement<Pixels>,
    border_color: Option<Rgba>,
}

impl FocusRing {
    pub fn new(id: impl Into<ElementId>, focus_handle: FocusHandle) -> Self {
        Self {
            id: id.into(),
            focus_handle: focus_handle,
            corner_radii: CornersRefinement {
                top_left: None,
                top_right: None,
                bottom_left: None,
                bottom_right: None,
            },
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
        let rounded = rounded.into();
        self.corner_radii = CornersRefinement {
            top_left: Some(rounded),
            top_right: Some(rounded),
            bottom_left: Some(rounded),
            bottom_right: Some(rounded),
        };
        self
    }

    pub fn rounded_tl(mut self, rounded: impl Into<Pixels>) -> Self {
        self.corner_radii.top_left = Some(rounded.into());
        self
    }

    pub fn rounded_tr(mut self, rounded: impl Into<Pixels>) -> Self {
        self.corner_radii.top_right = Some(rounded.into());
        self
    }

    pub fn rounded_bl(mut self, rounded: impl Into<Pixels>) -> Self {
        self.corner_radii.bottom_left = Some(rounded.into());
        self
    }

    pub fn rounded_br(mut self, rounded: impl Into<Pixels>) -> Self {
        self.corner_radii.bottom_right = Some(rounded.into());
        self
    }
}

impl RenderOnce for FocusRing {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let border_color = self
            .border_color
            .unwrap_or_else(|| cx.get_theme().variants.active(cx).colors.accent.primary);

        let is_focused = self.focus_handle.is_focused(window) as u8 as f32;

        let ring_progress_state = window
            .use_keyed_transition(
                self.id.clone(),
                cx,
                Duration::from_millis(365),
                |_window, _cx| is_focused,
            )
            .with_easing(ease_out_quint());

        ring_progress_state.update(cx, |this, cx| {
            if *this != is_focused {
                *this = is_focused;
                cx.notify();
            }
        });

        squircle()
            .absolute()
            .border(px(3.))
            .border_outside()
            .map(|this| {
                let ring_progress_delta = *ring_progress_state.evaluate(window, cx);
                let size_factor = ((1. - ring_progress_delta) * SIZE_SCALE_FACTOR).floor();

                this.inset(px(-size_factor))
                    .border_color(border_color.alpha(border_color.a * ring_progress_delta * 0.3))
                    .map(|mut this| {
                        this.outer_style().corner_radii =
                            add_to_corner_radii(&self.corner_radii, px(size_factor + 1.));
                        this
                    })
            })
    }
}

fn add_to_corner_radii(
    corner_radii: &CornersRefinement<Pixels>,
    num: Pixels,
) -> CornersRefinement<Pixels> {
    CornersRefinement {
        top_left: Some(corner_radii.top_left.unwrap_or_default() + num),
        top_right: Some(corner_radii.top_right.unwrap_or_default() + num),
        bottom_right: Some(corner_radii.bottom_right.unwrap_or_default() + num),
        bottom_left: Some(corner_radii.bottom_left.unwrap_or_default() + num),
    }
}
