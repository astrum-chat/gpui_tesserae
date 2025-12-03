use std::time::Duration;

use gpui::{App, ElementId, Rgba, Window, ease_out_quint};
use gpui_transitions::Transition;

pub fn hover_and_focus_border_color_transition(
    id: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
    is_hover: bool,
    is_focus: bool,
    default_color: Rgba,
    hover_color: Rgba,
    focus_color: Rgba,
) -> Transition<Rgba> {
    let border_color_transition_state = Transition::new(
        id,
        window,
        cx,
        Duration::from_millis(300),
        |_window, _cx| default_color,
    )
    .with_easing(ease_out_quint());

    border_color_transition_state.update(cx, |this, _cx| {
        *this = if is_focus {
            focus_color
        } else if is_hover {
            hover_color
        } else {
            default_color
        }
    });

    border_color_transition_state
}

pub fn hover_border_color_transition(
    id: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
    is_hover: bool,
    default_color: Rgba,
    hover_color: Rgba,
) -> Transition<Rgba> {
    let border_color_transition_state = Transition::new(
        id,
        window,
        cx,
        Duration::from_millis(300),
        |_window, _cx| default_color,
    )
    .with_easing(ease_out_quint());

    border_color_transition_state.update(cx, |this, _cx| {
        *this = if is_hover { hover_color } else { default_color }
    });

    border_color_transition_state
}
