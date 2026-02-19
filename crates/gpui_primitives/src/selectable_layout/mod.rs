//! Inline — a container that renders children as inline text (flowing left-to-right,
//! wrapping at word/character boundaries) with character-level selection support.
//!
//! Each child implements `InlinedChild`, providing its text content and text styling.
//! SelectableLayout concatenates all children's text, shapes it as one wrapped text block,
//! and paints each visual line. Optional per-child decorations (backgrounds, padding)
//! are painted behind the text.

mod state;

pub use state::{Copy, SelectAll, SelectableLayoutState};

use std::ops::Range;

use gpui::{
    App, Bounds, Corners, CursorStyle, Element, ElementId, Entity, FocusHandle, Focusable, Font,
    GlobalElementId, Hitbox, HitboxBehavior, Hsla, InspectorElementId, InteractiveElement,
    IntoElement, KeyBinding, LayoutId, MouseButton, PaintQuad, ParentElement, Pixels, Refineable,
    RenderOnce, ShapedLine, SharedString, Style, StyleRefinement, Styled, TextRun, Window, point,
    prelude::FluentBuilder, px, relative, size,
};

#[cfg(feature = "squircle")]
use crate::utils::build_squircle_path;
use crate::utils::{
    VisibleLineInfo, VisualLineInfo, WIDTH_WRAP_BASE_MARGIN, build_selection_primitive,
    build_visual_lines_from_wrap_boundaries, compute_interior_corner_patches,
    compute_selection_corners, create_text_run, rgb_a, selection_config_from_options,
};

/// Break info for line break children: amount of empty lines.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BreakInfo {
    /// 0 = flush to next line, n >= 1 = flush + n empty lines of gap.
    pub amount: usize,
    /// Font size from the line break child (reserved for future use).
    #[allow(dead_code)]
    pub font_size: Pixels,
}

/// Controls whether a decoration's bounds (padding) affect layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecorationDisplay {
    /// Decoration bounds affect layout — padding reserves space,
    /// preventing overlap with adjacent children.
    #[default]
    Block,
    /// Decoration is paint-only — padding is used for the visual
    /// rect but doesn't affect text positioning or line height.
    Overlay,
}

#[derive(Clone, Debug)]
pub struct InlineStyles {
    pub(crate) background: Hsla,
    pub(crate) corner_radius: Corners<Pixels>,
    #[cfg(feature = "squircle")]
    pub(crate) corner_smoothing: Option<f32>,
    pub(crate) padding_x: Pixels,
    pub(crate) padding_y: Pixels,
    pub(crate) display: DecorationDisplay,
}

impl InlineStyles {
    pub fn new() -> Self {
        Self {
            background: Hsla::transparent_black(),
            corner_radius: Corners::default(),
            #[cfg(feature = "squircle")]
            corner_smoothing: None,
            padding_x: Pixels::ZERO,
            padding_y: Pixels::ZERO,
            display: DecorationDisplay::Block,
        }
    }

    pub fn bg(mut self, color: Hsla) -> Self {
        self.background = color;
        self
    }

    pub fn corner_radius(mut self, radius: Corners<Pixels>) -> Self {
        self.corner_radius = radius;
        self
    }

    #[cfg(feature = "squircle")]
    pub fn corner_smoothing(mut self, smoothing: f32) -> Self {
        self.corner_smoothing = Some(smoothing);
        self
    }

    pub fn padding_x(mut self, padding: Pixels) -> Self {
        self.padding_x = padding;
        self
    }

    pub fn padding_y(mut self, padding: Pixels) -> Self {
        self.padding_y = padding;
        self
    }

    pub fn display(mut self, display: DecorationDisplay) -> Self {
        self.display = display;
        self
    }
}

/// Trait for SelectableLayout children that provide text content and styling.
///
/// SelectableLayout concatenates all children's text into one string, shapes and wraps it,
/// and paints text directly. Children no longer produce `AnyElement`s.
pub trait InlinedChild {
    /// The text content of this child.
    fn copy_text(&self) -> SharedString;

    /// The text run style for this child's text.
    /// `len` is the byte length of this child's text (provided for convenience).
    fn text_run(&self, len: usize) -> TextRun;

    /// Optional per-child font size. Returns `None` to use the SelectableLayout default.
    fn font_size(&self) -> Option<Pixels> {
        None
    }

    /// Line break info: `None` = not a break, `Some(0)` = go to next line,
    /// `Some(n)` = go to next line + n empty lines between.
    fn line_break(&self) -> Option<usize> {
        None
    }

    /// Optional decoration painted behind each text segment of this child.
    fn decoration(&self) -> Option<InlineStyles> {
        None
    }
}

/// Conversion trait that allows `.child("text")` by capturing SelectableLayout's default font/color.
pub trait IntoInlinedChild {
    /// Convert into a boxed `InlinedChild`, using the given defaults for plain text.
    fn into_inlined_child(self, font: &Font, text_color: Hsla) -> Box<dyn InlinedChild>;
}

impl<T: InlinedChild + 'static> IntoInlinedChild for T {
    fn into_inlined_child(self, _font: &Font, _text_color: Hsla) -> Box<dyn InlinedChild> {
        Box::new(self)
    }
}

impl IntoInlinedChild for &str {
    fn into_inlined_child(self, font: &Font, text_color: Hsla) -> Box<dyn InlinedChild> {
        Box::new(TextChild {
            text: self.to_string(),
            font: font.clone(),
            color: text_color,
        })
    }
}

/// A line break child that forces subsequent children onto a new line.
struct LineBreakChild {
    amount: usize,
    size: Option<Pixels>,
}

impl InlinedChild for LineBreakChild {
    fn copy_text(&self) -> SharedString {
        // One \n for the line flush (extends the preceding content line's byte range),
        // plus one \n per gap line.
        let count = 1 + self.amount;
        SharedString::from("\n".repeat(count))
    }

    fn text_run(&self, len: usize) -> TextRun {
        TextRun {
            len,
            font: Font::default(),
            color: Hsla::transparent_black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    }

    fn font_size(&self) -> Option<Pixels> {
        self.size
    }

    fn line_break(&self) -> Option<usize> {
        Some(self.amount)
    }
}

/// A plain text child that inherits the SelectableLayout's default font and color.
/// Created automatically when passing `&str` or `String` to `.child()`.
struct TextChild {
    text: String,
    font: Font,
    color: Hsla,
}

impl InlinedChild for TextChild {
    fn copy_text(&self) -> SharedString {
        SharedString::from(self.text.clone())
    }

    fn text_run(&self, len: usize) -> TextRun {
        TextRun {
            len,
            font: self.font.clone(),
            color: self.color,
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    }
}

/// A container that renders children as inline text with wrapping and selection support.
#[derive(IntoElement)]
pub struct SelectableLayout {
    id: ElementId,
    state: Entity<SelectableLayoutState>,
    children: Vec<Box<dyn InlinedChild>>,
    font: Font,
    font_size: Pixels,
    line_height: Pixels,
    text_color: Hsla,
    selection_color: Option<Hsla>,
    selection_rounded: Option<Pixels>,
    #[cfg(feature = "squircle")]
    selection_rounded_smoothing: Option<f32>,
    selection_precise: bool,
    style: StyleRefinement,
}

impl Styled for SelectableLayout {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

#[allow(missing_docs)]
impl SelectableLayout {
    pub fn new(
        id: impl Into<ElementId>,
        state: Entity<SelectableLayoutState>,
        font: Font,
        font_size: Pixels,
        line_height: Pixels,
        text_color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            state,
            children: Vec::new(),
            font,
            font_size,
            line_height,
            text_color,
            selection_color: None,
            selection_rounded: None,
            #[cfg(feature = "squircle")]
            selection_rounded_smoothing: None,
            selection_precise: false,
            style: StyleRefinement::default(),
        }
    }

    pub fn child(mut self, child: impl IntoInlinedChild) -> Self {
        self.children
            .push(child.into_inlined_child(&self.font, self.text_color));
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Box<dyn InlinedChild>>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn line_break(mut self, amount: usize, font_size: Pixels) -> Self {
        self.children.push(Box::new(LineBreakChild {
            amount,
            size: Some(font_size),
        }));
        self
    }

    pub fn selection_color(mut self, color: impl Into<Hsla>) -> Self {
        self.selection_color = Some(color.into());
        self
    }

    pub fn selection_rounded(mut self, radius: impl Into<Pixels>) -> Self {
        self.selection_rounded = Some(radius.into());
        self
    }

    #[cfg(feature = "squircle")]
    pub fn selection_rounded_smoothing(mut self, smoothing: f32) -> Self {
        self.selection_rounded_smoothing = Some(smoothing.clamp(0.0, 1.0));
        self
    }

    pub fn selection_precise(mut self) -> Self {
        self.selection_precise = true;
        self
    }
}

fn register_actions(
    element: gpui::Stateful<gpui::Div>,
    window: &mut Window,
    state: &Entity<SelectableLayoutState>,
) -> gpui::Stateful<gpui::Div> {
    element
        .on_action(window.listener_for(state, SelectableLayoutState::select_all))
        .on_action(window.listener_for(state, SelectableLayoutState::copy))
}

fn register_mouse_handlers(
    element: gpui::Stateful<gpui::Div>,
    window: &mut Window,
    state: &Entity<SelectableLayoutState>,
) -> gpui::Stateful<gpui::Div> {
    element
        .on_mouse_down(
            MouseButton::Left,
            window.listener_for(state, SelectableLayoutState::on_mouse_down),
        )
        .on_mouse_up(
            MouseButton::Left,
            window.listener_for(state, SelectableLayoutState::on_mouse_up),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            window.listener_for(state, SelectableLayoutState::on_mouse_up),
        )
}

impl RenderOnce for SelectableLayout {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let highlight_color = self
            .selection_color
            .unwrap_or_else(|| rgb_a(0x488BFF, 0.3).into());

        // Concatenate all children's text and build text runs.
        let mut combined_text = String::new();
        let mut text_runs: Vec<TextRun> = Vec::with_capacity(self.children.len());
        let mut child_byte_offsets: Vec<usize> = Vec::with_capacity(self.children.len());
        let mut decorations: Vec<Option<InlineStyles>> = Vec::with_capacity(self.children.len());
        let mut child_font_sizes: Vec<Pixels> = Vec::with_capacity(self.children.len());
        let mut child_line_break: Vec<Option<BreakInfo>> = Vec::with_capacity(self.children.len());

        for child in &self.children {
            let text = child.copy_text();
            let len = text.len();
            child_byte_offsets.push(combined_text.len());
            combined_text.push_str(&text);
            text_runs.push(child.text_run(len));
            decorations.push(child.decoration());
            let child_fs = child.font_size().unwrap_or(self.font_size);
            child_font_sizes.push(child_fs);
            child_line_break.push(child.line_break().map(|amount| BreakInfo {
                amount,
                font_size: child_fs,
            }));
        }

        let combined: SharedString = combined_text.into();
        let total_len = combined.len();

        // Store combined text and child info in state.
        self.state.update(cx, |state, _cx| {
            state.combined_text = combined.clone();
            state.child_byte_offsets = child_byte_offsets;
            state.total_text_len = total_len;
            state.update_focus_state(window);
        });

        let focus_handle = self.state.read(cx).focus_handle.clone();

        let user_wants_auto_width =
            matches!(self.style.size.width, None | Some(gpui::Length::Auto));

        let base = gpui::div()
            .id(self.id.clone())
            .min_w_0()
            .map(|mut this: gpui::Stateful<gpui::Div>| {
                this.style().refine(&self.style);
                // The parent div needs a definite width so the child's
                // relative(1.) resolves correctly. Apply w_full() when the
                // user wants auto-width (matching SelectableText wrapped mode).
                if user_wants_auto_width {
                    this = this.w_full();
                }
                this
            })
            .key_context("SelectableLayout")
            .track_focus(&focus_handle);

        let base = register_actions(base, window, &self.state);
        let base = register_mouse_handlers(base, window, &self.state);

        let mut element_style = Style::default();
        element_style.size.width = relative(1.).into();

        // Compute effective line height: base line height + max padding_y * 2 (Block only).
        let max_padding_y = decorations
            .iter()
            .filter_map(|d| d.as_ref())
            .filter(|d| d.display == DecorationDisplay::Block)
            .map(|d| d.padding_y)
            .fold(Pixels::ZERO, |acc, p| if p > acc { p } else { acc });
        let effective_line_height = self.line_height + max_padding_y * 2.0;

        base.child(SelectableLayoutElement {
            state: self.state.clone(),
            combined_text: combined,
            text_runs,
            decorations,
            child_font_sizes,
            child_line_break,
            font: self.font,
            font_size: self.font_size,
            line_height: self.line_height,
            effective_line_height,
            text_color: self.text_color,
            selection_color: highlight_color,
            selection_rounded: self.selection_rounded,
            #[cfg(feature = "squircle")]
            selection_rounded_smoothing: self.selection_rounded_smoothing,
            selection_precise: self.selection_precise,
            style: element_style,
        })
    }
}

impl Focusable for SelectableLayout {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus_handle.clone()
    }
}

// ---------------------------------------------------------------------------
// SelectableLayoutElement — custom Element for inline text flow layout
// ---------------------------------------------------------------------------

struct SelectableLayoutElement {
    state: Entity<SelectableLayoutState>,
    combined_text: SharedString,
    text_runs: Vec<TextRun>,
    decorations: Vec<Option<InlineStyles>>,
    child_font_sizes: Vec<Pixels>,
    child_line_break: Vec<Option<BreakInfo>>,
    font: Font,
    font_size: Pixels,
    line_height: Pixels,
    /// Line height + max(padding_y) * 2 across all decorations.
    /// Used for vertical line spacing so decoration rects don't overlap.
    effective_line_height: Pixels,
    text_color: Hsla,
    selection_color: Hsla,
    selection_rounded: Option<Pixels>,
    #[cfg(feature = "squircle")]
    selection_rounded_smoothing: Option<f32>,
    selection_precise: bool,
    style: Style,
}

/// A single child's shaped text segment within a visual line, positioned at an x-offset
/// that accounts for decoration padding of preceding children.
#[derive(Clone)]
pub(crate) struct ChildSegment {
    /// The shaped text for this child's portion of the visual line.
    pub(crate) shaped_line: ShapedLine,
    /// X offset of the text from the line origin (centered within the child's allocated space).
    pub(crate) x_offset: Pixels,
    /// X offset of the child's allocated space from the line origin.
    pub(crate) child_x: Pixels,
    /// Total allocated width for this child (text + padding).
    pub(crate) child_width: Pixels,
    /// Index into the decorations array (identifies which child this belongs to).
    pub(crate) child_idx: usize,
    /// Byte range in the combined text that this segment covers.
    pub(crate) byte_range: Range<usize>,
}

/// Per-visual-line prepaint data with individually positioned child segments.
#[derive(Clone)]
pub(crate) struct VisualLinePrepaint {
    /// Child segments in order, each with its own x-offset.
    pub(crate) segments: Vec<ChildSegment>,
    /// Total content width including all decoration padding.
    pub(crate) total_width: Pixels,
}

/// Byte range for an effective line (derived from segments, not the text shaper).
#[derive(Clone, Debug)]
struct EffectiveLineRange {
    start_offset: usize,
    end_offset: usize,
}

struct SelectableLayoutPrepaintState {
    /// Per-effective-line segment layout info.
    line_layouts: Vec<VisualLinePrepaint>,
    /// Byte ranges for each effective line.
    effective_line_ranges: Vec<EffectiveLineRange>,
    /// Per-line y offset from the top of the element bounds.
    line_y_offsets: Vec<Pixels>,
    /// Per-line height (may differ for break gap lines vs content lines).
    line_heights: Vec<Pixels>,
    /// Per-line hitboxes covering the total content width (text + padding).
    text_hitboxes: Vec<Hitbox>,
}

impl IntoElement for SelectableLayoutElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SelectableLayoutElement {
    type RequestLayoutState = ();
    type PrepaintState = SelectableLayoutPrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let state = self.state.clone();
        let combined_text = self.combined_text.clone();
        let text_runs = self.text_runs.clone();
        let child_font_sizes = self.child_font_sizes.clone();
        let child_line_break = self.child_line_break.clone();
        let line_height = self.line_height;
        let effective_line_height = self.effective_line_height;
        let style = self.style.clone();

        // Per-child decoration info for overflow wrapping simulation: (padding_x, is_block).
        let child_decoration_info: Vec<(Pixels, bool)> = self
            .decorations
            .iter()
            .map(|d| {
                d.as_ref().map_or((Pixels::ZERO, false), |d| {
                    let is_block = d.display == DecorationDisplay::Block;
                    (d.padding_x, is_block)
                })
            })
            .collect();

        let child_byte_offsets = self.state.read(cx).child_byte_offsets.clone();
        let total_text_len = self.combined_text.len();

        let layout_id = window.request_measured_layout(style, {
            move |known_dimensions, available_space, window, cx| {
                let width = known_dimensions.width.or(match available_space.width {
                    gpui::AvailableSpace::Definite(x) => Some(x),
                    _ => None,
                });

                let Some(container_width) = width else {
                    let cached_count = state.read(cx).precomputed_visual_lines.len().max(1);
                    return size(Pixels::ZERO, effective_line_height * cached_count as f32);
                };

                // Shape consecutive same-font-size children together so word
                // wrapping flows naturally across child boundaries (e.g. bold
                // text followed by normal text on the same line).
                let child_count = child_byte_offsets.len();
                let visual_lines = if combined_text.is_empty() {
                    vec![VisualLineInfo {
                        start_offset: 0,
                        end_offset: 0,
                        wrapped_line_index: 0,
                        visual_index_in_wrapped: 0,
                    }]
                } else {
                    let mut vlines = Vec::new();

                    // Build groups of consecutive non-break children with the
                    // same font size. Each group is shaped as a single
                    // shape_text call with multiple TextRuns.
                    let mut group_start_child: Option<usize> = None;
                    let mut group_fs = Pixels::ZERO;

                    let flush_group = |group_start_child: &mut Option<usize>,
                                       group_fs: Pixels,
                                       end_child: usize,
                                       vlines: &mut Vec<VisualLineInfo>,
                                       combined_text: &str,
                                       child_byte_offsets: &[usize],
                                       text_runs: &[TextRun],
                                       _child_font_sizes: &[Pixels],
                                       child_decoration_info: &[(Pixels, bool)],
                                       child_line_break: &[Option<BreakInfo>],
                                       total_text_len: usize,
                                       container_width: Pixels,
                                       window: &mut Window| {
                        let Some(start) = group_start_child.take() else {
                            return;
                        };

                        let group_byte_start = child_byte_offsets[start];
                        let group_byte_end = if end_child < child_byte_offsets.len() {
                            child_byte_offsets[end_child]
                        } else {
                            total_text_len
                        };

                        if group_byte_start >= group_byte_end {
                            return;
                        }

                        let group_text: SharedString = combined_text
                            [group_byte_start..group_byte_end]
                            .to_string()
                            .into();

                        // Build text runs for all children in the group.
                        let mut runs = Vec::new();
                        for ci in start..end_child {
                            if child_line_break[ci].is_some() {
                                continue;
                            }
                            let ci_start = child_byte_offsets[ci];
                            let ci_end = if ci + 1 < child_byte_offsets.len() {
                                child_byte_offsets[ci + 1]
                            } else {
                                total_text_len
                            };
                            let ci_len = ci_end - ci_start;
                            if ci_len == 0 {
                                continue;
                            }
                            let mut run = text_runs[ci].clone();
                            run.len = ci_len;
                            runs.push(run);
                        }

                        if runs.is_empty() {
                            return;
                        }

                        // Use the max Block padding in the group for wrap width.
                        let max_pad = (start..end_child)
                            .filter(|ci| child_line_break[*ci].is_none())
                            .map(|ci| {
                                let (px, is_block) = &child_decoration_info[ci];
                                if *is_block { *px } else { Pixels::ZERO }
                            })
                            .fold(Pixels::ZERO, |a, b| if b > a { b } else { a });
                        let wrap_width = (container_width - max_pad * 2.0).max(Pixels::ZERO)
                            + WIDTH_WRAP_BASE_MARGIN;

                        let wrapped_lines = window
                            .text_system()
                            .shape_text(group_text, group_fs, &runs, Some(wrap_width), None)
                            .unwrap_or_default();

                        let mut text_offset = group_byte_start;
                        for (wrapped_idx, wrapped_line) in wrapped_lines.iter().enumerate() {
                            let line_len = wrapped_line.len();
                            build_visual_lines_from_wrap_boundaries(
                                vlines,
                                wrapped_line,
                                wrapped_idx,
                                text_offset,
                                line_len,
                            );
                            text_offset += line_len + 1;
                        }
                    };

                    for child_idx in 0..child_count {
                        if child_line_break[child_idx].is_some() {
                            // Flush any pending group before the break.
                            flush_group(
                                &mut group_start_child,
                                group_fs,
                                child_idx,
                                &mut vlines,
                                &combined_text,
                                &child_byte_offsets,
                                &text_runs,
                                &child_font_sizes,
                                &child_decoration_info,
                                &child_line_break,
                                total_text_len,
                                container_width,
                                window,
                            );
                            continue;
                        }

                        let child_start = child_byte_offsets[child_idx];
                        let child_end = if child_idx + 1 < child_count {
                            child_byte_offsets[child_idx + 1]
                        } else {
                            total_text_len
                        };
                        if child_end <= child_start {
                            continue;
                        }

                        let fs = child_font_sizes[child_idx];

                        if group_start_child.is_none() {
                            group_start_child = Some(child_idx);
                            group_fs = fs;
                        } else if fs > group_fs {
                            // Track the max font size in the group for
                            // shape_text wrapping (conservative — no line
                            // will be wider than expected).
                            group_fs = fs;
                        }
                    }

                    // Flush final group.
                    flush_group(
                        &mut group_start_child,
                        group_fs,
                        child_count,
                        &mut vlines,
                        &combined_text,
                        &child_byte_offsets,
                        &text_runs,
                        &child_font_sizes,
                        &child_decoration_info,
                        &child_line_break,
                        total_text_len,
                        container_width,
                        window,
                    );

                    if vlines.is_empty() {
                        vlines.push(VisualLineInfo {
                            start_offset: 0,
                            end_offset: 0,
                            wrapped_line_index: 0,
                            visual_index_in_wrapped: 0,
                        });
                    }
                    vlines
                };

                // Simulate overflow wrapping to compute total height.
                // Content lines use effective_line_height. Break gaps use
                // amount * (line_height + break_font_size).
                let mut total_height = Pixels::ZERO;
                let mut x_cursor = Pixels::ZERO;
                let mut max_x_cursor = Pixels::ZERO;
                let mut has_segments = false;

                let flush_content_line =
                    |total_height: &mut Pixels,
                     x_cursor: &mut Pixels,
                     max_x_cursor: &mut Pixels,
                     has_segments: &mut bool| {
                        if *has_segments {
                            if *x_cursor > *max_x_cursor {
                                *max_x_cursor = *x_cursor;
                            }
                            *total_height += effective_line_height;
                            *x_cursor = Pixels::ZERO;
                            *has_segments = false;
                        }
                    };

                // Process children in child order so breaks appear at the correct
                // position between the text children they separate.
                for (child_idx, &child_start) in child_byte_offsets.iter().enumerate() {
                    if let Some(ref brk) = child_line_break[child_idx] {
                        flush_content_line(
                            &mut total_height,
                            &mut x_cursor,
                            &mut max_x_cursor,
                            &mut has_segments,
                        );
                        if brk.amount > 0 {
                            total_height += line_height * brk.amount as f32;
                        }
                        continue;
                    }

                    let child_end = if child_idx + 1 < child_count {
                        child_byte_offsets[child_idx + 1]
                    } else {
                        total_text_len
                    };

                    let mut child_seg_count = 0usize;
                    for vline in &visual_lines {
                        let overlap_start = child_start.max(vline.start_offset);
                        let overlap_end = child_end.min(vline.end_offset);
                        if overlap_start >= overlap_end {
                            continue;
                        }

                        // Intra-child visual line boundary (word-wrap or newline).
                        if child_seg_count > 0 {
                            flush_content_line(
                                &mut total_height,
                                &mut x_cursor,
                                &mut max_x_cursor,
                                &mut has_segments,
                            );
                        }
                        child_seg_count += 1;

                        let (padding_x, is_block) = &child_decoration_info[child_idx];
                        let pad = if *is_block { *padding_x } else { Pixels::ZERO };
                        let child_fs = child_font_sizes[child_idx];

                        let seg_text = &combined_text[overlap_start..overlap_end];
                        let mut seg_run = text_runs[child_idx].clone();
                        seg_run.len = seg_text.len();
                        let seg_runs = vec![seg_run];
                        let shaped = window.text_system().shape_line(
                            SharedString::from(seg_text.to_string()),
                            child_fs,
                            &seg_runs,
                            None,
                        );

                        let child_width = shaped.width + pad * 2.0;

                        if has_segments
                            && x_cursor + child_width > container_width + WIDTH_WRAP_BASE_MARGIN
                        {
                            flush_content_line(
                                &mut total_height,
                                &mut x_cursor,
                                &mut max_x_cursor,
                                &mut has_segments,
                            );
                        }

                        x_cursor += child_width;
                        has_segments = true;
                    }
                }
                flush_content_line(
                    &mut total_height,
                    &mut x_cursor,
                    &mut max_x_cursor,
                    &mut has_segments,
                );

                // Ensure at least one line height.
                if total_height < effective_line_height {
                    total_height = effective_line_height;
                }

                // Store visual lines for prepaint to consume.
                state.update(cx, |state, _cx| {
                    state.precomputed_visual_lines = visual_lines;
                });

                // Always return the intrinsic content width (widest line)
                // plus a small margin to prevent subpixel feedback loops.
                // The parent div carries user constraints (w_full, max_w, etc.)
                // and Taffy will clamp accordingly. This matches
                // WrappedTextElement which always returns content_width + margin.
                size(max_x_cursor + WIDTH_WRAP_BASE_MARGIN, total_height)
            }
        });

        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let child_byte_offsets = self.state.read(cx).child_byte_offsets.clone();
        let child_count = child_byte_offsets.len();
        let total_len = self.combined_text.len();
        let container_width = bounds.size.width;

        // Read precomputed visual lines from measure callback.
        let visual_lines = self.state.read(cx).precomputed_visual_lines.clone();

        // Phase 1: Split visual lines by child to produce pending items.
        enum PendingItem {
            Segment {
                shaped_line: ShapedLine,
                child_idx: usize,
                byte_range: Range<usize>,
                padding_x: Pixels,
                child_width: Pixels,
            },
            Break {
                info: BreakInfo,
                /// Byte offset where this break child's text starts in the combined string.
                byte_start: usize,
                /// True for real LineBreakChild breaks, false for synthetic
                /// intra-child word-wrap boundaries.
                is_line_break_child: bool,
            },
        }

        let mut pending_items: Vec<PendingItem> = Vec::new();

        // Process children in child order so breaks are interleaved correctly.
        for (child_idx, &child_start) in child_byte_offsets.iter().enumerate() {
            if let Some(brk) = self.child_line_break[child_idx] {
                pending_items.push(PendingItem::Break {
                    info: brk,
                    byte_start: child_start,
                    is_line_break_child: true,
                });
                continue;
            }

            let child_end = if child_idx + 1 < child_count {
                child_byte_offsets[child_idx + 1]
            } else {
                total_len
            };

            let decoration = &self.decorations[child_idx];
            let is_block = decoration
                .as_ref()
                .map_or(false, |d| d.display == DecorationDisplay::Block);
            let padding_x = if is_block {
                decoration.as_ref().map_or(Pixels::ZERO, |d| d.padding_x)
            } else {
                Pixels::ZERO
            };

            let seg_font_size = self.child_font_sizes[child_idx];

            let mut child_seg_count = 0usize;
            for vline in visual_lines.iter() {
                let overlap_start = child_start.max(vline.start_offset);
                let overlap_end = child_end.min(vline.end_offset);
                if overlap_start >= overlap_end {
                    continue;
                }

                // If this child already produced a segment from a previous visual
                // line, the boundary between them is a word-wrap or newline break
                // — insert a flush-to-next-line break.
                if child_seg_count > 0 {
                    pending_items.push(PendingItem::Break {
                        info: BreakInfo {
                            amount: 0,
                            font_size: seg_font_size,
                        },
                        byte_start: 0,
                        is_line_break_child: false,
                    });
                }
                child_seg_count += 1;

                let segment_text = &self.combined_text.as_ref()[overlap_start..overlap_end];
                let display_text: SharedString = segment_text.to_string().into();
                let mut seg_run = self.text_runs[child_idx].clone();
                seg_run.len = segment_text.len();
                let seg_runs = vec![seg_run];

                let shaped =
                    window
                        .text_system()
                        .shape_line(display_text, seg_font_size, &seg_runs, None);

                let child_width = shaped.width + padding_x * 2.0;

                pending_items.push(PendingItem::Segment {
                    shaped_line: shaped,
                    child_idx,
                    byte_range: overlap_start..overlap_end,
                    padding_x,
                    child_width,
                });
            }
        }

        // Phase 2: Lay out pending items into effective lines with per-line y offsets.
        let mut line_layouts: Vec<VisualLinePrepaint> = Vec::new();
        let mut effective_line_ranges: Vec<EffectiveLineRange> = Vec::new();
        let mut visible_lines_info: Vec<VisibleLineInfo> = Vec::new();
        let mut text_hitboxes: Vec<Hitbox> = Vec::new();
        let mut line_y_offsets: Vec<Pixels> = Vec::new();
        let mut line_heights: Vec<Pixels> = Vec::new();

        let mut current_segments: Vec<ChildSegment> = Vec::new();
        let mut x_cursor = Pixels::ZERO;
        let mut y_cursor = Pixels::ZERO;
        for item in pending_items {
            match item {
                PendingItem::Break {
                    info: brk,
                    byte_start,
                    is_line_break_child,
                } => {
                    // Flush current content line if any.
                    if !current_segments.is_empty() {
                        self.flush_effective_line_at_y(
                            &mut current_segments,
                            &mut x_cursor,
                            y_cursor,
                            self.effective_line_height,
                            &mut line_layouts,
                            &mut effective_line_ranges,
                            &mut visible_lines_info,
                            &mut text_hitboxes,
                            &mut line_y_offsets,
                            &mut line_heights,
                            window,
                            &bounds,
                        );
                        // For real LineBreakChild breaks, extend the content line's
                        // byte range to include the flush \n (at byte_start). This
                        // way, selecting to the end of the content line includes the
                        // \n, causing the gap line below to appear selected.
                        // Synthetic breaks (intra-child word-wrap) don't have real
                        // \n bytes, so skip this.
                        if is_line_break_child {
                            if let Some(last_range) = effective_line_ranges.last_mut() {
                                last_range.end_offset = byte_start + 1;
                            }
                        }
                        y_cursor += self.effective_line_height;
                    }
                    // Add gap lines for amount >= 1.
                    // The break child's text is "\n".repeat(1 + amount).
                    // The first \n (at byte_start) is the flush — not a gap line.
                    // Each subsequent \n (byte_start+1, byte_start+2, ...) is a gap line.
                    if brk.amount > 0 {
                        let gap_line_h = self.line_height;
                        // Shape a space for hit-testing, like SelectableText
                        // does for empty lines.
                        let gap_shaped = window.text_system().shape_line(
                            SharedString::from(" "),
                            self.font_size,
                            &[create_text_run(self.font.clone(), self.text_color, 1)],
                            None,
                        );
                        for gap_i in 0..brk.amount {
                            let effective_idx = line_layouts.len();
                            let line_origin = point(bounds.origin.x, bounds.origin.y + y_cursor);
                            let line_bounds = Bounds {
                                origin: line_origin,
                                size: gpui::Size {
                                    width: bounds.size.width,
                                    height: gap_line_h,
                                },
                            };
                            line_layouts.push(VisualLinePrepaint {
                                segments: Vec::new(),
                                total_width: Pixels::ZERO,
                            });
                            // Each gap line owns one \n byte. The first \n
                            // (at byte_start) is the flush character that extends
                            // the preceding content line's range. Gap lines start
                            // at byte_start + 1.
                            let gap_byte = byte_start + 1 + gap_i;
                            effective_line_ranges.push(EffectiveLineRange {
                                start_offset: gap_byte,
                                end_offset: gap_byte + 1,
                            });
                            visible_lines_info.push(VisibleLineInfo {
                                line_index: effective_idx,
                                bounds: line_bounds,
                                shaped_line: gap_shaped.clone(),
                            });
                            line_y_offsets.push(y_cursor);
                            line_heights.push(gap_line_h);
                            y_cursor += gap_line_h;
                        }
                    }
                }
                PendingItem::Segment {
                    shaped_line,
                    child_idx,
                    byte_range,
                    padding_x,
                    child_width,
                } => {
                    // Overflow wrap: flush current line if this segment won't fit.
                    if x_cursor > Pixels::ZERO
                        && x_cursor + child_width > container_width + WIDTH_WRAP_BASE_MARGIN
                    {
                        self.flush_effective_line_at_y(
                            &mut current_segments,
                            &mut x_cursor,
                            y_cursor,
                            self.effective_line_height,
                            &mut line_layouts,
                            &mut effective_line_ranges,
                            &mut visible_lines_info,
                            &mut text_hitboxes,
                            &mut line_y_offsets,
                            &mut line_heights,
                            window,
                            &bounds,
                        );
                        y_cursor += self.effective_line_height;
                    }

                    let child_x = x_cursor;
                    let text_x = child_x + padding_x;

                    current_segments.push(ChildSegment {
                        shaped_line,
                        x_offset: text_x,
                        child_x,
                        child_width,
                        child_idx,
                        byte_range,
                    });

                    x_cursor += child_width;
                }
            }
        }

        // Flush remaining segments.
        if !current_segments.is_empty() {
            self.flush_effective_line_at_y(
                &mut current_segments,
                &mut x_cursor,
                y_cursor,
                self.effective_line_height,
                &mut line_layouts,
                &mut effective_line_ranges,
                &mut visible_lines_info,
                &mut text_hitboxes,
                &mut line_y_offsets,
                &mut line_heights,
                window,
                &bounds,
            );
        }

        // If no lines were produced, add an empty one.
        if line_layouts.is_empty() {
            line_layouts.push(VisualLinePrepaint {
                segments: Vec::new(),
                total_width: Pixels::ZERO,
            });
            effective_line_ranges.push(EffectiveLineRange {
                start_offset: 0,
                end_offset: 0,
            });
            let line_origin = point(bounds.origin.x, bounds.origin.y);
            let line_bounds = Bounds {
                origin: line_origin,
                size: gpui::Size {
                    width: bounds.size.width,
                    height: self.effective_line_height,
                },
            };
            visible_lines_info.push(VisibleLineInfo {
                line_index: 0,
                bounds: line_bounds,
                shaped_line: window.text_system().shape_line(
                    SharedString::default(),
                    self.font_size,
                    &[create_text_run(self.font.clone(), self.text_color, 0)],
                    None,
                ),
            });
            line_y_offsets.push(Pixels::ZERO);
            line_heights.push(self.effective_line_height);
        }

        // Store visible line info and segment layouts in state for hit-testing.
        let line_byte_ranges: Vec<(usize, usize)> = effective_line_ranges
            .iter()
            .map(|r| (r.start_offset, r.end_offset))
            .collect();
        self.state.update(cx, |state, _cx| {
            state.visible_lines_info = visible_lines_info;
            state.line_layouts = line_layouts.clone();
            state.line_byte_ranges = line_byte_ranges;
            state.last_bounds = Some(bounds);
        });

        SelectableLayoutPrepaintState {
            line_layouts,
            effective_line_ranges,
            line_y_offsets,
            line_heights,
            text_hitboxes,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let selected_range = self.state.read(cx).selected_range.clone();

        // Global mouse move handler — fires even when cursor leaves the element/window.
        let state = self.state.clone();
        window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, _window, cx| {
            if phase == gpui::DispatchPhase::Capture {
                return;
            }
            state.update(cx, |state, cx| {
                if state.is_selecting {
                    state.on_mouse_move_global(event.position, cx);
                }
            });
        });

        for hitbox in &prepaint.text_hitboxes {
            window.set_cursor_style(CursorStyle::IBeam, hitbox);
        }

        for (idx, (line_layout, line_range)) in prepaint
            .line_layouts
            .iter()
            .zip(prepaint.effective_line_ranges.iter())
            .enumerate()
        {
            let y_off = prepaint.line_y_offsets[idx];
            let line_h = prepaint.line_heights[idx];

            let line_origin = point(bounds.origin.x, bounds.origin.y + y_off);
            let line_bounds = Bounds {
                origin: line_origin,
                size: gpui::Size {
                    width: bounds.size.width,
                    height: line_h,
                },
            };

            // Vertical offset to center text within the line height slot.
            let text_y_offset = (line_h - self.line_height) / 2.0;

            // Paint decorations behind text, connecting across lines like selections.
            let scale_factor = window.scale_factor();
            for seg in &line_layout.segments {
                if let Some(decoration) = &self.decorations[seg.child_idx] {
                    let dec_height = self.line_height + decoration.padding_y * 2.0;
                    let dec_y = line_origin.y + (line_h - dec_height) / 2.0;

                    // Block decorations use the allocated child space;
                    // Overlay decorations are centered on the text.
                    let (dec_x, dec_width) = if decoration.display == DecorationDisplay::Block {
                        (seg.child_x, seg.child_width)
                    } else {
                        let w = seg.shaped_line.width + decoration.padding_x * 2.0;
                        (seg.x_offset - decoration.padding_x, w)
                    };

                    // Round decoration x positions to device pixels so quad edges
                    // and interior corner patches align exactly.
                    let round = |v: Pixels| -> Pixels {
                        let dp = v * scale_factor;
                        px(f32::from(dp).round() / scale_factor)
                    };
                    let this_start_x = round(dec_x);
                    let this_end_x = round(dec_x + dec_width).min(line_bounds.size.width);

                    // Find the same child's decoration x-range on adjacent lines.
                    let prev_dec = if idx > 0 {
                        Self::decoration_x_range_for_child(
                            &prepaint.line_layouts[idx - 1],
                            seg.child_idx,
                            &self.decorations,
                        )
                        .map(|(s, e)| (round(s), round(e)))
                    } else {
                        None
                    };
                    let next_dec = if idx + 1 < prepaint.line_layouts.len() {
                        Self::decoration_x_range_for_child(
                            &prepaint.line_layouts[idx + 1],
                            seg.child_idx,
                            &self.decorations,
                        )
                        .map(|(s, e)| (round(s), round(e)))
                    } else {
                        None
                    };

                    // Use the max corner radius for interior corner patch logic.
                    let radius = decoration
                        .corner_radius
                        .top_left
                        .max(decoration.corner_radius.top_right)
                        .max(decoration.corner_radius.bottom_left)
                        .max(decoration.corner_radius.bottom_right);

                    // A corner is only squared if the step is large enough for an
                    // interior corner patch to actually render. The patch needs
                    // step >= radius/2 + MIN_INTERIOR_CORNER_STEP (2px), so use
                    // that as the probe threshold.
                    let probe = compute_selection_corners(
                        this_start_x,
                        this_end_x,
                        prev_dec,
                        next_dec,
                        radius / 2.0 + px(2.0),
                        scale_factor,
                    );
                    let corners = Corners {
                        top_left: if probe.top_left == Pixels::ZERO {
                            Pixels::ZERO
                        } else {
                            decoration.corner_radius.top_left
                        },
                        top_right: if probe.top_right == Pixels::ZERO {
                            Pixels::ZERO
                        } else {
                            decoration.corner_radius.top_right
                        },
                        bottom_left: if probe.bottom_left == Pixels::ZERO {
                            Pixels::ZERO
                        } else {
                            decoration.corner_radius.bottom_left
                        },
                        bottom_right: if probe.bottom_right == Pixels::ZERO {
                            Pixels::ZERO
                        } else {
                            decoration.corner_radius.bottom_right
                        },
                    };

                    // When the decoration continues to an adjacent line, extend the
                    // quad to fill the full line height on that edge so interior
                    // corner patches (which start at the quad edge) connect
                    // seamlessly with no gap.
                    let extended_top = if prev_dec.is_some() {
                        line_bounds.origin.y
                    } else {
                        dec_y
                    };
                    let extended_bottom = if next_dec.is_some() {
                        line_bounds.origin.y + line_bounds.size.height
                    } else {
                        dec_y + dec_height
                    };

                    let dec_bounds = Bounds {
                        origin: point(line_bounds.origin.x + this_start_x, extended_top),
                        size: gpui::Size {
                            width: this_end_x - this_start_x,
                            height: extended_bottom - extended_top,
                        },
                    };

                    // Paint decoration background, using squircle path when available.
                    let mut used_squircle = false;
                    #[cfg(feature = "squircle")]
                    if let Some(smoothing) = decoration.corner_smoothing {
                        if smoothing > 0.0 {
                            if let Some(prim) = build_squircle_path(
                                dec_bounds,
                                corners,
                                smoothing,
                                decoration.background,
                            ) {
                                prim.paint(window);
                                used_squircle = true;
                            }
                        }
                    }
                    if !used_squircle {
                        window.paint_quad(PaintQuad {
                            bounds: dec_bounds,
                            corner_radii: corners,
                            background: decoration.background.into(),
                            border_widths: gpui::Edges::default(),
                            border_color: Hsla::transparent_black(),
                            border_style: gpui::BorderStyle::default(),
                        });
                    }

                    // Paint interior corner patches to fill gaps at connection points.
                    // Use the extended decoration bounds so patches start exactly
                    // at the painted quad edges (not the line bounds).
                    #[cfg(feature = "squircle")]
                    let smoothing = decoration.corner_smoothing;
                    #[cfg(not(feature = "squircle"))]
                    let smoothing: Option<f32> = None;
                    let patches = compute_interior_corner_patches(
                        this_start_x,
                        this_end_x,
                        prev_dec,
                        next_dec,
                        radius,
                        smoothing,
                        scale_factor,
                        line_bounds.origin.x,
                        dec_bounds.origin.y,
                        dec_bounds.origin.y + dec_bounds.size.height,
                        dec_bounds.size.height,
                        Pixels::ZERO,
                        decoration.background,
                    );
                    for patch in patches {
                        patch.paint(window);
                    }
                }
            }

            // Paint selection behind text.
            if !selected_range.is_empty() {
                self.paint_line_selection(
                    idx,
                    line_layout,
                    line_range,
                    &line_bounds,
                    &selected_range,
                    &prepaint.effective_line_ranges,
                    &prepaint.line_layouts,
                    window,
                );
            }

            // Paint each child segment's text at its offset position.
            for seg in &line_layout.segments {
                // Use a line height that accommodates the segment's font metrics so
                // that decorations (strikethrough, underline) are positioned correctly
                // even when the segment's font is larger than the base line height.
                let seg_line_height = self
                    .line_height
                    .max(seg.shaped_line.ascent + seg.shaped_line.descent);
                // Shift origin upward to compensate for extra internal padding that
                // paint_line adds when line_height > base line_height, keeping the
                // glyphs at the same vertical position.
                let y_adjust = (self.line_height - seg_line_height) / 2.0;
                let seg_origin = point(
                    line_origin.x + seg.x_offset,
                    line_origin.y + text_y_offset + y_adjust,
                );
                let _ = seg.shaped_line.paint(
                    seg_origin,
                    seg_line_height,
                    gpui::TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
        }

        // Store container bounds.
        self.state.update(cx, |state, _cx| {
            state.last_bounds = Some(bounds);
        });
    }
}

impl SelectableLayoutElement {
    /// Flush the current set of segments as a completed effective line at a specific y offset.
    fn flush_effective_line_at_y(
        &self,
        segments: &mut Vec<ChildSegment>,
        x_cursor: &mut Pixels,
        y_offset: Pixels,
        height: Pixels,
        line_layouts: &mut Vec<VisualLinePrepaint>,
        effective_line_ranges: &mut Vec<EffectiveLineRange>,
        visible_lines_info: &mut Vec<VisibleLineInfo>,
        text_hitboxes: &mut Vec<Hitbox>,
        line_y_offsets: &mut Vec<Pixels>,
        line_heights: &mut Vec<Pixels>,
        window: &mut Window,
        bounds: &Bounds<Pixels>,
    ) {
        if segments.is_empty() {
            return;
        }

        let effective_idx = line_layouts.len();
        let total_width = *x_cursor;

        let line_start = segments.first().unwrap().byte_range.start;
        let line_end = segments.last().unwrap().byte_range.end;

        let line_origin = point(bounds.origin.x, bounds.origin.y + y_offset);
        let line_bounds = Bounds {
            origin: line_origin,
            size: gpui::Size {
                width: bounds.size.width,
                height,
            },
        };

        if total_width > Pixels::ZERO {
            let text_bounds = Bounds {
                origin: line_origin,
                size: gpui::Size {
                    width: total_width,
                    height,
                },
            };
            text_hitboxes.push(window.insert_hitbox(text_bounds, HitboxBehavior::Normal));
        }

        // Shape a full line for hit-testing from the byte range spanning all segments.
        // Use the max font size across segments (hit-testing primarily uses per-segment
        // ChildSegment layouts, so the full-line shape is a fallback).
        let full_text: SharedString = self.combined_text.as_ref()[line_start..line_end]
            .to_string()
            .into();
        let full_runs = self.slice_runs_for_range(line_start, line_end);
        let max_fs = segments
            .iter()
            .map(|s| self.child_font_sizes[s.child_idx])
            .fold(self.font_size, |a, b| if b > a { b } else { a });
        let full_shaped = window
            .text_system()
            .shape_line(full_text, max_fs, &full_runs, None);

        visible_lines_info.push(VisibleLineInfo {
            line_index: effective_idx,
            bounds: line_bounds,
            shaped_line: full_shaped,
        });

        effective_line_ranges.push(EffectiveLineRange {
            start_offset: line_start,
            end_offset: line_end,
        });

        line_layouts.push(VisualLinePrepaint {
            segments: std::mem::take(segments),
            total_width,
        });

        line_y_offsets.push(y_offset);
        line_heights.push(height);

        *x_cursor = Pixels::ZERO;
    }

    /// Slice the global text runs to produce runs for a byte range of the combined text.
    fn slice_runs_for_range(&self, start: usize, end: usize) -> Vec<TextRun> {
        if start >= end {
            return vec![create_text_run(self.font.clone(), self.text_color, 0)];
        }

        let mut result = Vec::new();
        let mut run_offset = 0;

        for run in &self.text_runs {
            let run_end = run_offset + run.len;
            // Does this run overlap with [start, end)?
            if run_end > start && run_offset < end {
                let overlap_start = start.max(run_offset);
                let overlap_end = end.min(run_end);
                let overlap_len = overlap_end - overlap_start;
                if overlap_len > 0 {
                    let mut sliced = run.clone();
                    sliced.len = overlap_len;
                    result.push(sliced);
                }
            }
            run_offset = run_end;
        }

        if result.is_empty() {
            result.push(create_text_run(self.font.clone(), self.text_color, 0));
        }

        result
    }

    /// Map a byte offset in the combined text to an x position relative to the line origin,
    /// accounting for per-child segment offsets (decoration padding).
    fn x_for_byte_offset(segments: &[ChildSegment], byte_offset: usize) -> Pixels {
        for seg in segments {
            if byte_offset >= seg.byte_range.start && byte_offset <= seg.byte_range.end {
                let local = byte_offset - seg.byte_range.start;
                return seg.x_offset + seg.shaped_line.x_for_index(local);
            }
        }
        // Offset is past all segments — return the end of the last segment.
        if let Some(last) = segments.last() {
            last.x_offset + last.shaped_line.width
        } else {
            Pixels::ZERO
        }
    }

    /// Paint the selection highlight for a single effective line.
    fn paint_line_selection(
        &self,
        line_idx: usize,
        line_layout: &VisualLinePrepaint,
        line_range: &EffectiveLineRange,
        line_bounds: &Bounds<Pixels>,
        selected_range: &Range<usize>,
        all_line_ranges: &[EffectiveLineRange],
        all_line_layouts: &[VisualLinePrepaint],
        window: &mut Window,
    ) {
        // Intersect selection with this effective line's byte range.
        let sel_start = selected_range.start.max(line_range.start_offset);
        let sel_end = selected_range.end.min(line_range.end_offset);

        let is_gap_line = line_layout.segments.is_empty();

        if sel_start >= sel_end {
            return;
        }

        let x_start = if is_gap_line {
            Pixels::ZERO
        } else {
            Self::x_for_byte_offset(&line_layout.segments, sel_start)
        };
        let mut x_end = if is_gap_line {
            Pixels::ZERO
        } else {
            Self::x_for_byte_offset(&line_layout.segments, sel_end)
        };

        // Trailing whitespace indicator: show that selection continues past this line.
        let last_seg_end = line_layout.segments.last().map_or(0, |s| s.byte_range.end);
        let extends_past = selected_range.end > line_range.end_offset || sel_end > last_seg_end;

        // Compute space width for trailing whitespace indicator.
        let space_width = if self.selection_precise {
            let space_line = window.text_system().shape_line(
                SharedString::from(" "),
                self.font_size,
                &[create_text_run(self.font.clone(), self.text_color, 1)],
                None,
            );
            space_line.width
        } else {
            Pixels::ZERO
        };

        if extends_past || is_gap_line {
            if self.selection_precise {
                // Precise mode: add one space-character width (matching SelectableText).
                x_end += space_width;
            } else {
                // Non-precise: extend to container edge.
                x_end = line_bounds.size.width;
            }
        }

        // Clamp to container right edge to prevent overflow.
        x_end = x_end.min(line_bounds.size.width);

        let sel_bounds = Bounds {
            origin: point(line_bounds.origin.x + x_start, line_bounds.origin.y),
            size: gpui::Size {
                width: x_end - x_start,
                height: line_bounds.size.height,
            },
        };

        #[cfg(feature = "squircle")]
        let selection_rounded_smoothing = self.selection_rounded_smoothing;
        #[cfg(not(feature = "squircle"))]
        let selection_rounded_smoothing: Option<f32> = None;
        let config =
            selection_config_from_options(self.selection_rounded, selection_rounded_smoothing);
        let scale_factor = window.scale_factor();

        let this_start_x = x_start;
        let this_end_x = x_end;

        let prev_row_sel = if line_idx > 0 {
            Self::line_selection_x_range_from_range(
                &all_line_ranges[line_idx - 1],
                &all_line_layouts[line_idx - 1],
                selected_range,
                self.selection_precise,
                line_bounds.size.width,
                space_width,
            )
        } else {
            None
        };
        let next_row_sel = if line_idx + 1 < all_line_ranges.len() {
            Self::line_selection_x_range_from_range(
                &all_line_ranges[line_idx + 1],
                &all_line_layouts[line_idx + 1],
                selected_range,
                self.selection_precise,
                line_bounds.size.width,
                space_width,
            )
        } else {
            None
        };

        let corners = compute_selection_corners(
            this_start_x,
            this_end_x,
            prev_row_sel,
            next_row_sel,
            config.corner_radius,
            scale_factor,
        );

        let selection_prim = build_selection_primitive(
            *line_bounds,
            this_start_x,
            this_end_x,
            Pixels::ZERO,
            self.selection_color,
            &config,
            corners,
        );
        selection_prim.paint(window);

        // Paint interior corner patches.
        let corner_smoothing = {
            #[cfg(feature = "squircle")]
            {
                config.corner_smoothing
            }
            #[cfg(not(feature = "squircle"))]
            {
                None
            }
        };
        let patches = compute_interior_corner_patches(
            this_start_x,
            this_end_x,
            prev_row_sel,
            next_row_sel,
            config.corner_radius,
            corner_smoothing,
            scale_factor,
            line_bounds.origin.x,
            sel_bounds.origin.y,
            sel_bounds.origin.y + sel_bounds.size.height,
            line_bounds.size.height,
            Pixels::ZERO,
            self.selection_color,
        );
        for patch in patches {
            patch.paint(window);
        }
    }

    /// Find the decoration x-range (start_x, end_x) for a given child on a line.
    fn decoration_x_range_for_child(
        line_layout: &VisualLinePrepaint,
        child_idx: usize,
        decorations: &[Option<InlineStyles>],
    ) -> Option<(Pixels, Pixels)> {
        let seg = line_layout
            .segments
            .iter()
            .find(|s| s.child_idx == child_idx)?;
        let decoration = decorations[child_idx].as_ref()?;

        let (dec_x, dec_width) = if decoration.display == DecorationDisplay::Block {
            (seg.child_x, seg.child_width)
        } else {
            let w = seg.shaped_line.width + decoration.padding_x * 2.0;
            (seg.x_offset - decoration.padding_x, w)
        };

        Some((dec_x, dec_x + dec_width))
    }

    /// Compute (start_x, end_x) of the selection on the given effective line.
    fn line_selection_x_range_from_range(
        line_range: &EffectiveLineRange,
        line_layout: &VisualLinePrepaint,
        selected_range: &Range<usize>,
        selection_precise: bool,
        container_width: Pixels,
        space_width: Pixels,
    ) -> Option<(Pixels, Pixels)> {
        let is_gap = line_layout.segments.is_empty();

        let sel_start = selected_range.start.max(line_range.start_offset);
        let sel_end = selected_range.end.min(line_range.end_offset);

        if sel_start >= sel_end {
            return None;
        }

        let x_start = if is_gap {
            Pixels::ZERO
        } else {
            Self::x_for_byte_offset(&line_layout.segments, sel_start)
        };
        let mut x_end = if is_gap {
            Pixels::ZERO
        } else {
            Self::x_for_byte_offset(&line_layout.segments, sel_end)
        };

        let last_seg_end = line_layout.segments.last().map_or(0, |s| s.byte_range.end);
        let extends_past = selected_range.end > line_range.end_offset || sel_end > last_seg_end;

        if extends_past || is_gap {
            if selection_precise {
                x_end += space_width;
            } else {
                x_end = container_width;
            }
        }

        // Clamp to container right edge to prevent overflow.
        x_end = x_end.min(container_width);

        Some((x_start, x_end))
    }
}

/// Registers default key bindings for SelectableLayout.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", SelectAll, Some("SelectableLayout")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some("SelectableLayout")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", SelectAll, Some("SelectableLayout")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some("SelectableLayout")),
    ]);
}
