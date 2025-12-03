use std::{
    borrow::BorrowMut,
    fmt::Debug,
    ops::{Add, Mul, Sub},
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, App, Bounds, Context, Corners, DevicePixels, Edges, Element, ElementId, Entity,
    EntityId, GlobalElementId, InspectorElementId, InteractiveElement, Interactivity, IntoElement,
    LayoutId, ParentElement, Percentage, Pixels, Point, Radians, Rems, Rgba, Size,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, colors::Colors, linear, px,
};

/// A transition that can be applied to an element.
#[derive(Clone)]
pub struct Transition<T: TransitionGoal + Clone + PartialEq + 'static> {
    /// The amount of time for which this transtion should run.
    duration_secs: f32,

    /// A function that takes a delta between 0 and 1 and returns a new delta
    /// between 0 and 1 based on the given easing function.
    easing: Rc<dyn Fn(f32) -> f32>,

    state: Entity<TransitionState<T>>,
}

impl<T: TransitionGoal + Clone + PartialEq + 'static> Transition<T> {
    /// Create a new transition with the given duration and goal.
    pub fn new(
        id: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
        duration: Duration,
        initial_goal: impl Fn(&mut Window, &mut Context<TransitionState<T>>) -> T,
    ) -> Self {
        Self {
            duration_secs: duration.as_secs_f32(),
            easing: Rc::new(linear),
            state: window.use_keyed_state(id, cx, |window, cx| {
                TransitionState::new(initial_goal(window, cx))
            }),
        }
    }

    /// Create a new transition with the given duration using the specified state.
    pub fn from_state(state: Entity<TransitionState<T>>, duration: Duration) -> Self {
        Self {
            duration_secs: duration.as_secs_f32(),
            easing: Rc::new(linear),
            state,
        }
    }

    /// Set the easing function to use for this transition.
    /// The easing function will take a time delta between 0 and 1 and return a new delta
    /// between 0 and 1
    pub fn with_easing(mut self, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        self.easing = Rc::new(easing);
        self
    }

    /// Reads the transition's goal.
    pub fn read<'a>(&self, cx: &'a App) -> &'a T {
        &self.state.read(cx).end_goal
    }

    /// Updates the goal for the transition without notifying gpui of any changes.
    pub fn update<R>(
        &self,
        cx: &mut App,
        update: impl FnOnce(&mut T, &mut crate::Context<TransitionState<T>>) -> R,
    ) -> bool {
        let mut was_updated = false;

        self.state.update(cx, |state, cx| {
            let last_end_goal = state.end_goal.clone();

            update(&mut state.end_goal, cx);

            if state.end_goal == last_end_goal {
                return;
            };

            state.goal_last_updated_at = Instant::now();
            state.start_goal = state
                .start_goal
                .apply_delta(&last_end_goal, state.last_delta);

            was_updated = true;
        });

        was_updated
    }

    /// Sets the goal for the transition without notifying gpui of any changes.
    pub fn set(&self, cx: &mut App, new_goal: T) -> bool {
        let mut was_updated = false;

        self.state.update(cx, |state, _cx| {
            if new_goal == state.end_goal {
                was_updated = false;
                return;
            }

            let last_end_goal = std::mem::replace(&mut state.end_goal, new_goal);

            state.goal_last_updated_at = Instant::now();
            state.start_goal = state
                .start_goal
                .apply_delta(&last_end_goal, state.last_delta);

            was_updated = true;
        });

        was_updated
    }

    /// Get the entity ID associated with this entity
    pub fn entity_id(&self) -> EntityId {
        self.state.entity_id()
    }

    // Evaluates the value for the transition based on the start and end goal.
    fn evaluate(&self, cx: &mut App) -> (bool, T) {
        let mut state_entity = self.state.as_mut(cx);
        let state: &mut TransitionState<T> = state_entity.borrow_mut();

        let elapsed_secs = state.goal_last_updated_at.elapsed().as_secs_f32();
        let delta = (self.easing)((elapsed_secs / self.duration_secs).min(1.));

        debug_assert!(
            (0.0..=1.0).contains(&delta),
            "delta should always be between 0 and 1"
        );

        state.last_delta = delta;

        let evaluated_value = state.start_goal.apply_delta(&state.end_goal, delta);

        (delta != 1., evaluated_value)
    }
}

/// State for a transition.
#[derive(Clone)]
pub struct TransitionState<T: TransitionGoal + Clone + PartialEq + 'static> {
    goal_last_updated_at: Instant,
    start_goal: T,
    end_goal: T,
    last_delta: f32,
}

impl<T: TransitionGoal + Clone + PartialEq + 'static> TransitionState<T> {
    fn new(initial_goal: T) -> Self {
        Self {
            goal_last_updated_at: Instant::now(),
            start_goal: initial_goal.clone(),
            end_goal: initial_goal,
            last_delta: 1.,
        }
    }
}

/// An extension trait for adding the transition wrapper to both Elements and Components
pub trait TransitionExt {
    /// Render this component or element with transitions
    fn with_transitions<'a, T>(
        self,
        transitions: T,
        animator: impl Fn(&mut App, Self, T::Values) -> Self + 'static,
    ) -> TransitionElement<'a, Self, T>
    where
        T: TransitionValues<'a>,
        Self: Sized,
    {
        TransitionElement {
            element: Some(self),
            animator: Box::new(animator),
            transitions,
        }
    }
}

impl<E: IntoElement + 'static> TransitionExt for E {}

/// A GPUI element that applies a transition to another element
pub struct TransitionElement<'a, E, T: TransitionValues<'a>> {
    element: Option<E>,
    transitions: T,
    animator: Box<dyn Fn(&mut App, E, T::Values) -> E + 'a>,
}

impl<E: Element + 'static, T: TransitionValues<'static> + 'static> Element
    for TransitionElement<'static, E, T>
{
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let (request_frame, evaluated_values) = self.transitions.evaluate(cx);

        let element = self.element.take().expect("should only be called once");
        let mut element = (self.animator)(cx, element, evaluated_values).into_any_element();

        if request_frame {
            window.request_animation_frame();
        }

        (element.request_layout(window, cx), element)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: crate::Bounds<crate::Pixels>,
        element: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: crate::Bounds<crate::Pixels>,
        element: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx)
    }
}

impl<E: Element + 'static, T: TransitionValues<'static> + 'static> IntoElement
    for TransitionElement<'static, E, T>
{
    type Element = TransitionElement<'static, E, T>;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<E: Element + Styled + 'static, T: TransitionValues<'static> + 'static> Styled
    for TransitionElement<'static, E, T>
{
    fn style(&mut self) -> &mut StyleRefinement {
        self.element.as_mut().unwrap().style()
    }
}

impl<E: Element + InteractiveElement + 'static, T: TransitionValues<'static> + 'static>
    InteractiveElement for TransitionElement<'static, E, T>
{
    fn interactivity(&mut self) -> &mut Interactivity {
        self.element.as_mut().unwrap().interactivity()
    }
}

impl<E: Element + ParentElement + 'static, T: TransitionValues<'static> + 'static> ParentElement
    for TransitionElement<'static, E, T>
{
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.element.as_mut().unwrap().extend(elements);
    }
}

impl<E: Element + StatefulInteractiveElement + 'static, T: TransitionValues<'static> + 'static>
    StatefulInteractiveElement for TransitionElement<'static, E, T>
{
}

/// A type which can be used as a transition goal.
pub trait TransitionGoal {
    /// Defines how a value is calculated from the start and end goal.
    fn apply_delta(&self, to: &Self, delta: f32) -> Self;
}

macro_rules! float_transition_goals {
    ( $( $ty:ty ),+ ) => {
        $(
            impl TransitionGoal for $ty {
                fn apply_delta(&self, to: &Self, delta: f32) -> Self {
                    lerp(*self, *to, delta as $ty)
                }
            }
        )+
    };
}

float_transition_goals!(f32, f64);

macro_rules! int_transition_goals {
    ( $( $ty:ident as $ty_into:ident ),+ ) => {
        $(
            impl TransitionGoal for $ty {
                fn apply_delta(&self, to: &Self, delta: f32) -> Self {
                    lerp(*self as $ty_into, *to as $ty_into, delta as $ty_into) as $ty
                }
            }
        )+
    };
}

int_transition_goals!(
    usize as f32,
    u8 as f32,
    u16 as f32,
    u32 as f32,
    u64 as f64,
    u128 as f64,
    isize as f32,
    i8 as f32,
    i16 as f32,
    i32 as f32,
    i64 as f64,
    i128 as f64
);

macro_rules! struct_transition_goals {
    ( $( $ty:ident $( < $gen:ident > )? { $( $n:ident ),+ } ),+ $(,)? ) => {
        $(
            impl$(<$gen: TransitionGoal + Clone + Debug + Default + PartialEq>)? TransitionGoal for $ty$(<$gen>)? {
                fn apply_delta(&self, to: &Self, delta: f32) -> Self {
                    $ty$(::<$gen>)? {
                        $(
                            $n: self.$n.apply_delta(&to.$n, delta)
                        ),+
                    }
                }
            }
        )+
    };
}

struct_transition_goals!(
    Point<T> { x, y },
    Size<T> { width, height },
    Edges<T> { top, right, bottom, left },
    Corners<T> { top_left, top_right, bottom_right, bottom_left },
    Bounds<T> { origin, size },
    Rgba { r, g, b, a },
    Colors { text, selected_text, background, disabled, selected, border, separator, container }
);

macro_rules! tuple_struct_transition_goals {
    ( $( $ty:ident ( $n:ty ) ),+ ) => {
        $(
            impl TransitionGoal for $ty {
                fn apply_delta(&self, to: &Self, delta: f32) -> Self {
                    $ty(self.0.apply_delta(&to.0, delta))
                }
            }
        )+
    };
}

tuple_struct_transition_goals!(Radians(f32), Percentage(f32), DevicePixels(i32), Rems(f32));

impl TransitionGoal for Pixels {
    fn apply_delta(&self, to: &Self, delta: f32) -> Self {
        px((self.to_f64() as f32).apply_delta(&(to.to_f64() as f32), delta))
    }
}

fn lerp<T>(a: T, b: T, t: T) -> T
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<Output = T>,
{
    a + (b - a) * t
}

/// A group of values that can be transitioned.
pub trait TransitionValues<'a> {
    /// The underlying type of the values.
    type Values;

    /// Evaluates the values for the transitions based on the start and end goals.
    fn evaluate(&self, cx: &mut App) -> (bool, Self::Values);
}

// Workaround for variadic generics as Rust doesn't support them.
// The main downside to this is that each tuple length needs its own implementation.
macro_rules! impl_with_transitions {
    ($first:ident $(, $rest:ident)*) => {
        impl_with_transitions!(@recurse () $first $(, $rest)*);
    };

    // Nothing left.
    (@recurse ($($prefix:ident),*) ) => {};

    // Generates an impl for the current prefix + head,
    // then recurses to include the next identifier in the prefix.
    (@recurse ($($prefix:ident),*) $head:ident $(,$tail:ident)*) => {
        impl_with_transitions!(@gen ($($prefix,)* $head));
        impl_with_transitions!(@recurse ($($prefix,)* $head) $($tail),*);
    };

    (@gen ($($names:ident),+)) => {
        #[allow(non_snake_case, unused_parens)]
        impl<'a, $($names),+> TransitionValues<'a> for ( $( Transition<$names> ),+, )
        where
            $( $names: TransitionGoal + Clone + PartialEq + 'static ),+
        {
            type Values = ( $( $names ),+);

            fn evaluate(&self, cx: &mut App) -> (bool, Self::Values)
            {
                let ( $( $names ),+ ,) = self;
                let mut request_frame = false;

                let evaluated_values = ($({
                    let (this_request_frame, transioned_value) = $names.evaluate(cx);
                    request_frame = this_request_frame || request_frame;
                    transioned_value
                }),+);

                (request_frame, evaluated_values)
            }
        }
    };
}

impl_with_transitions!(A, B, C, D, E, F);

impl<'a, A> TransitionValues<'a> for Transition<A>
where
    A: TransitionGoal + Clone + PartialEq + 'static,
{
    type Values = A;

    fn evaluate(&self, cx: &mut App) -> (bool, Self::Values) {
        self.evaluate(cx)
    }
}
