use std::time::Duration;

use gpui::{App, ElementId, Window, ease_out_quint};
use gpui_transitions::{Transition, WindowUseTransition};

use crate::ElementIdExt;

#[macro_export]
macro_rules! conitional_transition {
    (
        $id:expr, $window:expr, $cx:expr, $duration:expr, $($rest:tt)+
    ) => {{
        use gpui_transitions::{WindowUseTransition};

        let value = $crate::conditional_transition_branches!(@condition [ $($rest)+ ]);

        let transition = $window.use_keyed_transition(
            $id,
            $cx,
            $duration,
            |_window, _cx| value,
        )
        .with_easing(gpui::ease_out_quint());

        let value = value.into();

        if transition.read_goal($cx) != &value {
            transition.update($cx, |this, _cx| *this = value);
            $cx.notify(transition.entity_id());
        }

        transition
    }};
}

#[macro_export]
macro_rules! conitional_transition_update {
    (
        $cx:expr, $transition:expr, $($rest:tt)+
    ) => {{
        let value = $crate::conditional_transition_branches!(@condition [ $($rest)+ ]).into();

        if $transition.read_goal($cx) != &value {
            $transition.update($cx, |this, cx| {
                *this = value;
                cx.notify();
            });

        }

        $transition
    }};
}

#[macro_export]
macro_rules! conditional_transition_branches {
    // Default branch wasn't last.
    (@branch_list [ _ => $value:expr, $($rest:tt)+ ]) => {{
        compile_error!("`_ => value` is only allowed on the last branch.");
    }};

    // Entry point.
    (@condition [ { $cond:expr => $value:expr, $($rest:tt)+ } ]) => {{
        if $cond { $value } else { $crate::conditional_transition_branches!(@branch_list [ $($rest)+ ])  }
    }};

    (@branch_list [ $cond:expr => $value:expr, $($rest:tt)+ ]) => {{
        if $cond { $value } else { $crate::conditional_transition_branches!(@branch_list [ $($rest)+ ])  }
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

    let checked_transition = window
        .use_keyed_transition(
            base_id.into().with_suffix("state:checked"),
            cx,
            duration,
            |_cx, _window| is_checked_float,
        )
        .with_easing(ease_out_quint());

    checked_transition.update(cx, |this, cx| {
        if *this != is_checked_float {
            *this = is_checked_float;
            cx.notify();
        }
    });

    checked_transition
}
