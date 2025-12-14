use std::time::Duration;

use gpui::{
    AnyElement, CornersRefinement, ElementId, FocusHandle, FontWeight, InteractiveElement,
    IntoElement, ParentElement, Pixels, RenderOnce, Styled, div, ease_out_quint,
    prelude::FluentBuilder, px, relative,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::TransitionExt;
use smallvec::SmallVec;

use crate::{
    ElementIdExt, conitional_transition,
    primitives::{FocusRing, min_w0_wrapper},
    theme::{ThemeExt, ThemeLayerKind},
    utils::PixelsExt,
};

#[derive(Default)]
pub enum ChatBubbleAnchor {
    #[default]
    BottomRight,
    CenterRight,
    TopRight,
    BottomLeft,
    CenterLeft,
    TopLeft,
}

#[derive(IntoElement)]
pub struct ChatBubble {
    id: ElementId,
    layer: ThemeLayerKind,
    anchor: ChatBubbleAnchor,
    children: SmallVec<[AnyElement; 2]>,
    focus_handle: Option<FocusHandle>,
}

impl ChatBubble {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            layer: ThemeLayerKind::Tertiary,
            anchor: ChatBubbleAnchor::default(),
            children: SmallVec::new(),
            focus_handle: None,
        }
    }

    pub fn anchor(mut self, anchor: ChatBubbleAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn focus_handle(mut self, focus_handle: impl Into<FocusHandle>) -> Self {
        self.focus_handle = Some(focus_handle.into());
        self
    }
}

impl RenderOnce for ChatBubble {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let background_color = self.layer.resolve(cx);
        let border_color = self.layer.next().resolve(cx);
        let primary_accent_color = cx.get_theme().variants.active(cx).colors.accent.primary;
        let secondary_text_color = cx.get_theme().variants.active(cx).colors.text.secondary;
        let corner_radius = cx.get_theme().layout.corner_radii.xl;
        let anchor_corner_radius = cx.get_theme().layout.corner_radii.md;
        let corner_radii = calc_corner_radii(&self.anchor, corner_radius, anchor_corner_radius);
        let line_height = cx.get_theme().layout.text.default_font.line_height;
        let text_size = cx.get_theme().layout.text.default_font.sizes.heading_sm;
        let horizontal_padding = cx.get_theme().layout.padding.xl;
        let vertical_padding =
            cx.get_theme()
                .layout
                .size
                .xl
                .padding_needed_for_height(window, text_size, line_height);

        let is_focus = self
            .focus_handle
            .as_ref()
            .map(|this| this.is_focused(window))
            .unwrap_or(false);

        let border_color_transition_state = conitional_transition!(
            self.id.with_suffix("state:transition:border_color"),
            window,
            cx,
            Duration::from_millis(365),
            {
                is_focus => primary_accent_color,
                _ => border_color
            }
        )
        .with_easing(ease_out_quint());

        div()
            .max_w(relative(0.75))
            .flex()
            .flex_col()
            .items_start()
            .justify_start()
            .pt(vertical_padding)
            .pb(vertical_padding)
            .pl(horizontal_padding)
            .pr(horizontal_padding)
            .when_some(self.focus_handle, |this, focus_handle| {
                this.child(
                    FocusRing::new(self.id.with_suffix("focus_ring"), focus_handle.clone()).map(
                        |mut this| {
                            this.corner_radii = corner_radii.clone();
                            this
                        },
                    ),
                )
                .track_focus(&focus_handle)
            })
            .child(
                squircle()
                    .absolute_expand()
                    .bg(background_color)
                    .border(px(1.))
                    .border_inside()
                    .map(|mut this| {
                        this.outer_style().corner_radii = corner_radii;
                        this
                    })
                    .with_transitions(border_color_transition_state, |_cx, this, color| {
                        this.border_color(color)
                    }),
            )
            .child(
                min_w0_wrapper()
                    .font_family("Geist")
                    .text_color(secondary_text_color)
                    .text_size(text_size)
                    .font_weight(FontWeight::NORMAL)
                    .children(self.children),
            )
    }
}

impl ParentElement for ChatBubble {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

fn calc_corner_radii(
    anchor: &ChatBubbleAnchor,
    default_radius: Pixels,
    anchor_radius: Pixels,
) -> CornersRefinement<Pixels> {
    let mut corner_radii = CornersRefinement {
        top_left: Some(default_radius),
        top_right: Some(default_radius),
        bottom_left: Some(default_radius),
        bottom_right: Some(default_radius),
    };

    if matches!(
        anchor,
        ChatBubbleAnchor::BottomRight | ChatBubbleAnchor::CenterRight
    ) {
        corner_radii.bottom_right = Some(anchor_radius);
    }

    if matches!(
        anchor,
        ChatBubbleAnchor::TopRight | ChatBubbleAnchor::CenterRight
    ) {
        corner_radii.top_right = Some(anchor_radius);
    }

    if matches!(
        anchor,
        ChatBubbleAnchor::BottomLeft | ChatBubbleAnchor::CenterLeft
    ) {
        corner_radii.bottom_left = Some(anchor_radius);
    }

    if matches!(
        anchor,
        ChatBubbleAnchor::TopLeft | ChatBubbleAnchor::CenterLeft
    ) {
        corner_radii.top_left = Some(anchor_radius);
    }

    corner_radii
}
