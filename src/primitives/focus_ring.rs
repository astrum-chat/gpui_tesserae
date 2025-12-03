use std::time::Duration;

use gpui::{
    CornersRefinement, ElementId, FocusHandle, IntoElement, Pixels, RenderOnce, ease_out_quint,
    prelude::*, px,
};
use gpui_squircle::{SquircleStyleRefinement, SquircleStyled, squircle};
use gpui_tesserae_theme::ThemeExt;
use gpui_transitions::{Transition, TransitionExt};

use crate::utils::RgbaExt;

const SIZE_SCALE_FACTOR: f32 = 8.;

#[derive(IntoElement)]
pub struct FocusRing {
    id: ElementId,
    focus_handle: FocusHandle,
    style: SquircleStyleRefinement,
}

impl FocusRing {
    pub fn new(id: impl Into<ElementId>, focus_handle: FocusHandle) -> Self {
        Self {
            id: id.into(),
            focus_handle: focus_handle,
            style: SquircleStyleRefinement::default(),
        }
    }

    pub fn focus_handle(mut self, focus_handle: FocusHandle) -> Self {
        self.focus_handle = focus_handle;
        self
    }
}

impl SquircleStyled for FocusRing {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style.inner
    }

    fn outer_style(&mut self) -> &mut SquircleStyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for FocusRing {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let border_focus_color = cx.get_theme().variants.active().colors.accent.primary;

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
                    .border_color(border_focus_color.alpha(delta * 0.3))
                    .map(|mut this| {
                        this.outer_style().corner_radii = add_to_corner_radii(
                            &self.style.corner_radii,
                            px(8.),
                            px(size_factor + 1.),
                        );
                        this
                    })
            })
    }
}

fn add_to_corner_radii(
    corner_radii: &CornersRefinement<Pixels>,
    default: Pixels,
    num: Pixels,
) -> CornersRefinement<Pixels> {
    CornersRefinement {
        top_left: Some(corner_radii.top_left.unwrap_or(default) + num),
        top_right: Some(corner_radii.top_right.unwrap_or(default) + num),
        bottom_right: Some(corner_radii.bottom_right.unwrap_or(default) + num),
        bottom_left: Some(corner_radii.bottom_left.unwrap_or(default) + num),
    }
}
