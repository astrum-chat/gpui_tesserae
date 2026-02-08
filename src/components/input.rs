use std::time::Duration;

use gpui::{
    AbsoluteLength, App, Corners, DefiniteLength, Edges, ElementId, Entity, FocusHandle, Focusable,
    Hsla, InteractiveElement, IntoElement, Length, ParentElement, Pixels, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, div, ease_out_quint, prelude::FluentBuilder, px, relative,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::Lerp;

use crate::{
    conitional_transition,
    primitives::{
        FocusRing,
        input::{Input as PrimitiveInput, InputState},
    },
    theme::{ThemeExt, ThemeLayerKind},
    utils::{
        ElementIdExt, PixelsExt, PositionalChildren, PositionalParentElement, RgbaExt,
        disabled_transition,
    },
};

struct InputStyles {
    gap: Option<DefiniteLength>,
    padding: Edges<Option<DefiniteLength>>,
    inner_padding: Edges<Option<DefiniteLength>>,
    corner_radii: Corners<Option<Pixels>>,
    text_size: Option<AbsoluteLength>,
    width: Length,
    min_width: Option<Length>,
    min_height: Option<Length>,
    max_width: Option<Length>,
    max_height: Option<Length>,
}

impl Default for InputStyles {
    fn default() -> Self {
        Self {
            gap: None,
            padding: Edges::default(),
            inner_padding: Edges::default(),
            corner_radii: Corners::default(),
            text_size: None,
            width: Length::Auto,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
        }
    }
}

/// A styled text input component with validation and theming support.
#[derive(IntoElement)]
pub struct Input {
    id: ElementId,
    invalid: bool,
    disabled: bool,
    force_hover: bool,
    on_hover: Option<Box<dyn Fn(&bool, &mut gpui::Window, &mut App) + 'static>>,
    layer: ThemeLayerKind,
    children: PositionalChildren,
    style: InputStyles,
    base: PrimitiveInput,
}

impl Input {
    /// Creates a new input with the given ID and state entity.
    pub fn new(id: impl Into<ElementId>, state: Entity<InputState>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            invalid: false,
            disabled: false,
            force_hover: false,
            on_hover: None,
            layer: ThemeLayerKind::Tertiary,
            children: PositionalChildren::default(),
            style: InputStyles::default(),
            base: PrimitiveInput::new(id, state),
        }
    }

    /// Sets a fixed width.
    pub fn w(mut self, width: impl Into<Length>) -> Self {
        self.style.width = width.into();
        self
    }

    /// Sets width to auto, sizing based on content.
    pub fn w_auto(mut self) -> Self {
        self.style.width = Length::Auto;
        self
    }

    /// Sets width to fill the parent container.
    pub fn w_full(mut self) -> Self {
        self.style.width = relative(100.).into();
        self
    }

    /// Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
    pub fn min_w(mut self, width: impl Into<Length>) -> Self {
        self.style.min_width = Some(width.into());
        self
    }

    /// Sets the minimum width to 0. [Docs](https://tailwindcss.com/docs/min-width)
    pub fn min_w_0(mut self) -> Self {
        self.style.min_width = Some(px(0.).into());
        self
    }

    /// Sets the minimum width to 100%. [Docs](https://tailwindcss.com/docs/min-width)
    pub fn min_w_full(mut self) -> Self {
        self.style.min_width = Some(relative(100.).into());
        self
    }

    /// Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
    pub fn min_h(mut self, height: impl Into<Length>) -> Self {
        self.style.min_height = Some(height.into());
        self
    }

    /// Sets the minimum height to 0. [Docs](https://tailwindcss.com/docs/min-height)
    pub fn min_h_0(mut self) -> Self {
        self.style.min_height = Some(px(0.).into());
        self
    }

    /// Sets the minimum height to 100%. [Docs](https://tailwindcss.com/docs/min-height)
    pub fn min_h_full(mut self) -> Self {
        self.style.min_height = Some(relative(100.).into());
        self
    }

    /// Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
    pub fn max_w(mut self, width: impl Into<Length>) -> Self {
        self.style.max_width = Some(width.into());
        self
    }

    /// Sets the maximum width to 0. [Docs](https://tailwindcss.com/docs/max-width)
    pub fn max_w_0(mut self) -> Self {
        self.style.max_width = Some(px(0.).into());
        self
    }

    /// Sets the maximum width to 100%. [Docs](https://tailwindcss.com/docs/max-width)
    pub fn max_w_full(mut self) -> Self {
        self.style.max_width = Some(relative(100.).into());
        self
    }

    /// Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
    pub fn max_h(mut self, height: impl Into<Length>) -> Self {
        self.style.max_height = Some(height.into());
        self
    }

    /// Sets the maximum height to 0. [Docs](https://tailwindcss.com/docs/max-height)
    pub fn max_h_0(mut self) -> Self {
        self.style.max_height = Some(px(0.).into());
        self
    }

    /// Sets the maximum height to 100%. [Docs](https://tailwindcss.com/docs/max-height)
    pub fn max_h_full(mut self) -> Self {
        self.style.max_height = Some(relative(100.).into());
        self
    }

    /// Marks the input as invalid, showing destructive styling.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets the disabled state, preventing interaction.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Forces the hover visual state regardless of actual hover.
    pub fn force_hover(mut self, force_hover: bool) -> Self {
        self.force_hover = force_hover;
        self
    }

    /// Sets a callback invoked when hover state changes.
    pub fn on_hover(
        mut self,
        on_hover: impl Fn(&bool, &mut gpui::Window, &mut App) + 'static,
    ) -> Self {
        self.on_hover = Some(Box::new(on_hover));
        self
    }

    /// Sets the background layer for theming depth.
    pub fn layer(mut self, layer: ThemeLayerKind) -> Self {
        self.layer = layer;
        self
    }

    /// Sets the color for placeholder text.
    pub fn placeholder_text_color(mut self, color: impl Into<Hsla>) -> Self {
        self.base = self.base.placeholder_text_color(color);
        self
    }

    /// Sets the background color for selected text.
    pub fn selection_color(mut self, color: impl Into<Hsla>) -> Self {
        self.base = self.base.selection_color(color);
        self
    }

    /// Sets the placeholder text shown when input is empty.
    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.base = self.base.placeholder(text);
        self
    }

    /// Transforms each character for display without modifying the stored value. Useful for password fields.
    pub fn transform_text(
        mut self,
        transform: impl Fn(char) -> char + Send + Sync + 'static,
    ) -> Self {
        self.base = self.base.transform_text(transform);
        self
    }

    /// Transform the text value whenever it changes.
    /// Unlike `transform_text`, this actually modifies the stored value.
    /// - `text`: The full text after the raw change was applied
    /// - `inserted_ranges`: Character ranges where new text was inserted, or None for deletion-only
    pub fn map_text(
        mut self,
        f: impl Fn(SharedString) -> SharedString + Send + Sync + 'static,
    ) -> Self {
        self.base = self.base.map_text(f);
        self
    }

    /// Sets the maximum number of visible lines before scrolling. Use `multiline()` for unlimited.
    pub fn line_clamp(mut self, line_clamp: usize) -> Self {
        self.base = self.base.line_clamp(line_clamp);
        self
    }

    /// Enables multi-line mode with unconstrained height (no scrolling).
    /// Equivalent to `.line_clamp(usize::MAX)`.
    pub fn multiline(mut self) -> Self {
        self.base = self.base.multiline();
        self
    }

    /// Enables or disables word wrapping. Sets `line_clamp` to 1 if currently 0.
    pub fn word_wrap(mut self, enabled: bool) -> Self {
        self.base = self.base.word_wrap(enabled);
        self
    }

    /// Sets a callback to invoke on `Submit` action.
    /// Forces the `InsertNewlineSecondary` action to be used for newline.
    pub fn on_submit(mut self, callback: impl Fn(&mut gpui::Window, &mut App) + 'static) -> Self {
        self.base = self.base.on_submit(callback);
        self
    }

    /// Disables the submit action when set to `true`.
    /// When disabled, the `Submit` action will not trigger the `on_submit` callback.
    pub fn submit_disabled(mut self, disabled: bool) -> Self {
        self.base = self.base.submit_disabled(disabled);
        self
    }

    /// Forces the `InsertNewlineSecondary` action to be used for newline.
    pub fn secondary_newline(mut self) -> Self {
        self.base = self.base.secondary_newline();
        self
    }

    /// Set the maximum number of undo/redo history entries to keep.
    /// Defaults to 200.
    pub fn max_history(mut self, cx: &mut App, max: usize) -> Self {
        self.base = self.base.max_history(cx, max);
        self
    }

    /// Returns the current text value from the input state.
    pub fn read_text(&self, cx: &mut App) -> SharedString {
        self.base.read_text(cx)
    }

    /// Sets uniform corner radius for all corners.
    pub fn rounded(mut self, rounded: impl Into<Pixels>) -> Self {
        let rounded = rounded.into();
        self.style.corner_radii = Corners::all(Some(rounded));
        self
    }

    /// Sets the top-left corner radius.
    pub fn rounded_tl(mut self, rounded: impl Into<Pixels>) -> Self {
        self.style.corner_radii.top_left = Some(rounded.into());
        self
    }

    /// Sets the top-right corner radius.
    pub fn rounded_tr(mut self, rounded: impl Into<Pixels>) -> Self {
        self.style.corner_radii.top_right = Some(rounded.into());
        self
    }

    /// Sets the bottom-left corner radius.
    pub fn rounded_bl(mut self, rounded: impl Into<Pixels>) -> Self {
        self.style.corner_radii.bottom_left = Some(rounded.into());
        self
    }

    /// Sets the bottom-right corner radius.
    pub fn rounded_br(mut self, rounded: impl Into<Pixels>) -> Self {
        self.style.corner_radii.bottom_right = Some(rounded.into());
        self
    }

    /// Sets the gap between child elements.
    pub fn gap(mut self, gap: impl Into<DefiniteLength>) -> Self {
        self.style.gap = Some(gap.into());
        self
    }

    /// Sets uniform outer padding for all sides.
    pub fn p(mut self, padding: impl Into<DefiniteLength>) -> Self {
        let padding = padding.into();
        self.style.padding = Edges::all(Some(padding));
        self
    }

    /// Sets top outer padding.
    pub fn pt(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.padding.top = Some(padding.into());
        self
    }

    /// Sets bottom outer padding.
    pub fn pb(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.padding.bottom = Some(padding.into());
        self
    }

    /// Sets left outer padding.
    pub fn pl(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.padding.left = Some(padding.into());
        self
    }

    /// Sets right outer padding.
    pub fn pr(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.padding.right = Some(padding.into());
        self
    }

    /// Sets uniform inner padding around the text area.
    pub fn inner_p(mut self, padding: impl Into<DefiniteLength>) -> Self {
        let padding = padding.into();
        self.style.inner_padding = Edges::all(Some(padding));
        self
    }

    /// Sets top inner padding.
    pub fn inner_pt(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.inner_padding.top = Some(padding.into());
        self
    }

    /// Sets bottom inner padding.
    pub fn inner_pb(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.inner_padding.bottom = Some(padding.into());
        self
    }

    /// Sets left inner padding.
    pub fn inner_pl(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.inner_padding.left = Some(padding.into());
        self
    }

    /// Sets right inner padding.
    pub fn inner_pr(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.inner_padding.right = Some(padding.into());
        self
    }

    /// Sets the font size for the input text.
    pub fn text_size(mut self, padding: impl Into<AbsoluteLength>) -> Self {
        self.style.text_size = Some(padding.into());
        self
    }
}

macro_rules! apply_corner_radii {
    ($this:expr, $corner_radii_override:expr, $corner_radius:expr) => {
        $this
            .rounded_tl(
                $corner_radii_override
                    .top_left
                    .unwrap_or($corner_radius.into()),
            )
            .rounded_tr(
                $corner_radii_override
                    .top_right
                    .unwrap_or($corner_radius.into()),
            )
            .rounded_bl(
                $corner_radii_override
                    .bottom_left
                    .unwrap_or($corner_radius.into()),
            )
            .rounded_br(
                $corner_radii_override
                    .bottom_right
                    .unwrap_or($corner_radius.into()),
            )
    };
}

macro_rules! apply_padding {
    (
        $this:expr,
        $padding_override:expr,
        $vertical_padding:expr,
        $horizontal_padding:expr
    ) => {
        $this
            .pt($padding_override.top.unwrap_or($vertical_padding.into()))
            .pb($padding_override.bottom.unwrap_or($vertical_padding.into()))
            .pl($padding_override.left.unwrap_or($horizontal_padding.into()))
            .pr($padding_override
                .right
                .unwrap_or($horizontal_padding.into()))
    };
}

impl RenderOnce for Input {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let (primary_text_color, secondary_text_color) =
            cx.get_theme().variants.active(cx).colors.text.all();
        let primary_accent_color = cx.get_theme().variants.active(cx).colors.accent.primary;
        let destructive_accent_color = cx.get_theme().variants.active(cx).colors.accent.destructive;
        let background_color = self.layer.resolve(cx);
        let border_color = self.layer.next().resolve(cx);
        let border_hover_color = border_color.lerp(&primary_text_color, 0.07);
        let font_family = cx.get_theme().layout.text.default_font.family[0].clone();
        let line_height = cx.get_theme().layout.text.default_font.line_height;
        let text_size = self
            .style
            .text_size
            .unwrap_or_else(|| cx.get_theme().layout.text.default_font.sizes.body.clone());
        let corner_radius = cx.get_theme().layout.corner_radii.md;
        let corner_radii_override = self.style.corner_radii;
        let padding_override = self.style.padding;
        let inner_padding_override = self.style.inner_padding;
        let horizontal_padding = cx.get_theme().layout.padding.lg;
        let vertical_padding =
            cx.get_theme()
                .layout
                .size
                .lg
                .padding_needed_for_height(window, text_size, line_height);

        let is_invalid = self.invalid;

        let is_hover_state =
            window.use_keyed_state(self.id.with_suffix("state:hover"), cx, |_cx, _window| false);
        let is_hover = self.force_hover || *is_hover_state.read(cx);

        let focus_handle = self.focus_handle(cx).clone();
        let is_focus = focus_handle.is_focused(window);

        let is_disabled = self.disabled;
        let disabled_transition = disabled_transition(self.id.clone(), window, cx, is_disabled);

        if is_focus && is_disabled {
            window.blur();
        }

        let border_color_transition = conitional_transition!(
            self.id.with_suffix("state:transition:border_color"),
            window,
            cx,
            Duration::from_millis(400),
            {
                is_invalid => destructive_accent_color,
                is_focus => primary_accent_color,
                is_hover => border_hover_color,
                _ => border_color
            }
        )
        .with_easing(ease_out_quint());

        let focus_ring_color_transition = conitional_transition!(
            self.id.with_suffix("state:transition:focus_ring_color"),
            window,
            cx,
            Duration::from_millis(400),
            {
                is_invalid => destructive_accent_color,
                _ => primary_accent_color
            }
        )
        .with_easing(ease_out_quint());

        div()
            .id(self.id.clone())
            .w(self.style.width)
            .min_h_auto()
            .when_some(self.style.min_width, |this, v| this.min_w(v))
            .when_some(self.style.min_height, |this, v| this.min_h(v))
            .when_some(self.style.max_width, |this, v| this.max_w(v))
            .when_some(self.style.max_height, |this, v| this.max_h(v))
            .map(|this| {
                apply_padding!(this, padding_override, vertical_padding, horizontal_padding)
            })
            .flex()
            .flex_col()
            .opacity(*disabled_transition.evaluate(window, cx))
            .child(
                FocusRing::new(self.id.with_suffix("focus_ring"), focus_handle.clone())
                    .border_color(*focus_ring_color_transition.evaluate(window, cx))
                    .map(|this| apply_corner_radii!(this, corner_radii_override, corner_radius)),
            )
            .child(
                squircle()
                    .absolute_expand()
                    .map(|this| apply_corner_radii!(this, corner_radii_override, corner_radius))
                    .bg(background_color)
                    .border(px(1.))
                    .border_inside()
                    .border_color(*border_color_transition.evaluate(window, cx)),
            )
            .child(
                // We need to nest all of the main children to prevent layout
                // issues where the input primitive vanishes in certain conditions.
                div()
                    .w_auto()
                    .h_auto()
                    .flex()
                    .flex_col()
                    .gap(self.style.gap.unwrap_or(horizontal_padding.into()))
                    .children(self.children.top)
                    .child(
                        div()
                            .w_full()
                            .min_w_full()
                            .flex()
                            .min_h_auto()
                            .gap(horizontal_padding)
                            .items_center()
                            .map(|this| {
                                apply_padding!(this, inner_padding_override, px(0.), px(0.))
                            })
                            .children(self.children.left)
                            .child(
                                self.base
                                    .w_full()
                                    .text_size(text_size)
                                    .font_family(font_family)
                                    .text_color(primary_text_color)
                                    .placeholder_text_color(secondary_text_color)
                                    .selection_color(primary_accent_color.alpha(0.3))
                                    .selection_rounded(px(6.))
                                    .selection_rounded_smoothing(1.)
                                    .line_height(line_height)
                                    .disabled(is_disabled),
                            )
                            .children(self.children.right),
                    )
                    .children(self.children.bottom),
            )
            .when(!is_disabled, |this| {
                this.on_hover(move |hover, window, cx| {
                    is_hover_state.update(cx, |this, cx| {
                        *this = *hover;
                        cx.notify();
                    });

                    if let Some(callback) = self.on_hover.as_ref() {
                        (callback)(hover, window, cx);
                    }
                })
            })
    }
}

impl PositionalParentElement for Input {
    fn children_mut(&mut self) -> &mut crate::utils::PositionalChildren {
        &mut self.children
    }
}

impl Focusable for Input {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.base.focus_handle(cx)
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext, VisualTestContext};

    #[gpui::test]
    fn test_input_creation(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state);
            assert!(!input.invalid, "Input should start valid");
            assert!(!input.disabled, "Input should start enabled");
        });
    }

    #[gpui::test]
    fn test_input_invalid_state(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state.clone()).invalid(true);
            assert!(input.invalid, "Input should be invalid");

            let input = Input::new("test-input", state).invalid(false);
            assert!(!input.invalid, "Input should be valid");
        });
    }

    #[gpui::test]
    fn test_input_disabled_state(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state.clone()).disabled(true);
            assert!(input.disabled, "Input should be disabled");

            let input = Input::new("test-input", state).disabled(false);
            assert!(!input.disabled, "Input should be enabled");
        });
    }

    #[gpui::test]
    fn test_input_layer(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state.clone()).layer(ThemeLayerKind::Primary);
            assert!(
                matches!(input.layer, ThemeLayerKind::Primary),
                "Input should have primary layer"
            );

            let input = Input::new("test-input", state).layer(ThemeLayerKind::Secondary);
            assert!(
                matches!(input.layer, ThemeLayerKind::Secondary),
                "Input should have secondary layer"
            );
        });
    }

    #[gpui::test]
    fn test_input_placeholder(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let placeholder: SharedString = "Enter text here...".into();
            let input = Input::new("test-input", state).placeholder(placeholder.clone());
            assert_eq!(
                input.base.get_placeholder(),
                &placeholder,
                "Input should have custom placeholder"
            );
        });
    }

    #[gpui::test]
    fn test_input_rounded(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state).rounded(px(8.));
            assert!(
                input.style.corner_radii.top_left.is_some(),
                "Input should have rounded corners"
            );
            assert!(
                input.style.corner_radii.top_right.is_some(),
                "Input should have rounded corners"
            );
            assert!(
                input.style.corner_radii.bottom_left.is_some(),
                "Input should have rounded corners"
            );
            assert!(
                input.style.corner_radii.bottom_right.is_some(),
                "Input should have rounded corners"
            );
        });
    }

    #[gpui::test]
    fn test_input_individual_corners(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state)
                .rounded_tl(px(4.))
                .rounded_tr(px(8.))
                .rounded_bl(px(12.))
                .rounded_br(px(16.));

            assert_eq!(input.style.corner_radii.top_left, Some(px(4.)));
            assert_eq!(input.style.corner_radii.top_right, Some(px(8.)));
            assert_eq!(input.style.corner_radii.bottom_left, Some(px(12.)));
            assert_eq!(input.style.corner_radii.bottom_right, Some(px(16.)));
        });
    }

    #[gpui::test]
    fn test_input_padding(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state).p(px(10.));
            assert!(
                input.style.padding.top.is_some(),
                "Input should have top padding"
            );
            assert!(
                input.style.padding.bottom.is_some(),
                "Input should have bottom padding"
            );
            assert!(
                input.style.padding.left.is_some(),
                "Input should have left padding"
            );
            assert!(
                input.style.padding.right.is_some(),
                "Input should have right padding"
            );
        });
    }

    #[gpui::test]
    fn test_input_individual_padding(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state)
                .pt(px(4.))
                .pb(px(8.))
                .pl(px(12.))
                .pr(px(16.));

            assert!(input.style.padding.top.is_some());
            assert!(input.style.padding.bottom.is_some());
            assert!(input.style.padding.left.is_some());
            assert!(input.style.padding.right.is_some());
        });
    }

    #[gpui::test]
    fn test_input_inner_padding(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state).inner_p(px(5.));
            assert!(
                input.style.inner_padding.top.is_some(),
                "Input should have inner top padding"
            );
            assert!(
                input.style.inner_padding.bottom.is_some(),
                "Input should have inner bottom padding"
            );
            assert!(
                input.style.inner_padding.left.is_some(),
                "Input should have inner left padding"
            );
            assert!(
                input.style.inner_padding.right.is_some(),
                "Input should have inner right padding"
            );
        });
    }

    #[gpui::test]
    fn test_input_individual_inner_padding(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state)
                .inner_pt(px(2.))
                .inner_pb(px(4.))
                .inner_pl(px(6.))
                .inner_pr(px(8.));

            assert!(input.style.inner_padding.top.is_some());
            assert!(input.style.inner_padding.bottom.is_some());
            assert!(input.style.inner_padding.left.is_some());
            assert!(input.style.inner_padding.right.is_some());
        });
    }

    #[gpui::test]
    fn test_input_gap(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state).gap(px(10.));
            assert!(input.style.gap.is_some(), "Input should have gap");
        });
    }

    #[gpui::test]
    fn test_input_text_size(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state).text_size(px(16.));
            assert!(
                input.style.text_size.is_some(),
                "Input should have text size"
            );
        });
    }

    #[gpui::test]
    fn test_input_builder_chain(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        cx.update(|_cx| {
            let input = Input::new("test-input", state)
                .invalid(false)
                .disabled(false)
                .layer(ThemeLayerKind::Tertiary)
                .placeholder("Type here...")
                .rounded(px(8.))
                .p(px(10.))
                .gap(px(5.));

            assert!(!input.invalid);
            assert!(!input.disabled);
            assert!(matches!(input.layer, ThemeLayerKind::Tertiary));
        });
    }

    #[gpui::test]
    fn test_input_on_hover_callback(cx: &mut TestAppContext) {
        use std::cell::Cell;
        use std::rc::Rc;

        let state = cx.new(|cx| InputState::new(cx));
        let hovered = Rc::new(Cell::new(false));

        cx.update(|_cx| {
            let hovered_clone = hovered.clone();

            let input = Input::new("test-input", state).on_hover(move |is_hover, _window, _cx| {
                hovered_clone.set(*is_hover);
            });

            assert!(
                input.on_hover.is_some(),
                "Input should have on_hover callback"
            );
        });
    }

    #[gpui::test]
    fn test_input_renders_in_window(cx: &mut TestAppContext) {
        use crate::theme::{Theme, ThemeExt};

        let window = cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            cx.open_window(Default::default(), |_window, cx| {
                cx.new(|cx| InputTestView {
                    state: cx.new(|cx| InputState::new(cx)),
                })
            })
            .unwrap()
        });

        let _cx = VisualTestContext::from_window(window.into(), cx);

        // The window creation itself validates rendering works
    }

    #[gpui::test]
    fn test_input_state_read_text(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        let text = state.read_with(cx, |state, _| state.value());
        assert!(text.is_empty(), "Input state should start empty");
    }

    /// Test view that contains an Input
    struct InputTestView {
        state: Entity<InputState>,
    }

    impl gpui::Render for InputTestView {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            div()
                .size_full()
                .child(Input::new("test-input", self.state.clone()).placeholder("Type here..."))
        }
    }
}
