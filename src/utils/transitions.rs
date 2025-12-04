use std::time::Duration;

use gpui::{App, ElementId, Window, ease_out_quint};
use gpui_transitions::Transition;

use crate::ElementIdExt;

#[macro_export]
macro_rules! conitional_transition {
    (
        $id:expr, $window:expr, $cx:expr, $duration:expr, $($rest:tt)+
    ) => {{
        let value = conitional_transition!(@condition [ $($rest)+ ]);

        let transition = gpui_transitions::Transition::new(
            $id,
            $window,
            $cx,
            $duration,
            |_window, _cx| value,
        )
        .with_easing(gpui::ease_out_quint());

        if transition.read($cx) != &value {
            transition.update($cx, |this, _cx| *this = value);
            $cx.notify(transition.entity_id());
        }

        transition
    }};

    // Match-esque block:

    // Default branch wasn't last.
    (@branch_list [ _ => $value:expr, $($rest:tt)+ ]) => {{
        compile_error!("`_ => value` is only allowed on the last branch.");
    }};

    // Entry point.
    (@condition [ { $cond:expr => $value:expr, $($rest:tt)+ } ]) => {{
        if $cond { $value } else { conitional_transition!(@branch_list [ $($rest)+ ])  }
    }};

    (@branch_list [ $cond:expr => $value:expr, $($rest:tt)+ ]) => {{
        if $cond { $value } else { conitional_transition!(@branch_list [ $($rest)+ ])  }
    }};

    // Last branch.
    (@branch_list [ _ => $value:expr ]) => {{
        $value
    }};

    // Last branch wasn't default.
    (@branch_list [ $cond:expr => $value:expr ]) => {{
        compile_error!("The last branch must be `_ => value`");
    }};


    // Other
    (@condition [ $($rest:tt)+ ]) => {{
        $($rest)+
    }};
}

pub fn disabled_transition(
    base_id: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
    is_disabled: bool,
) -> Transition<f32> {
    conitional_transition!(
        base_id.into().with_suffix("state:transition:disabled"),
        window,
        cx,
        Duration::from_millis(365),
        {
            is_disabled => 0.45,
            _ => 1.
        }
    )
    .with_easing(ease_out_quint())
}

pub fn checked_transition(
    base_id: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
    duration: Duration,
    is_checked: bool,
) -> Transition<f32> {
    let is_checked_float = is_checked as u8 as f32;

    let checked_state = Transition::new(
        base_id.into().with_suffix("state:checked"),
        window,
        cx,
        duration,
        |_cx, _window| is_checked_float,
    )
    .with_easing(ease_out_quint());

    let checked_changed = checked_state.set(cx, is_checked_float);
    if checked_changed {
        cx.notify(checked_state.entity_id());
    }

    checked_state
}
