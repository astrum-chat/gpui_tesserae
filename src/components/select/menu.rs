use std::sync::Arc;

use gpui::{ElementId, div, prelude::*, px};
use gpui_squircle::{SquircleStyled, squircle};
use smallvec::SmallVec;

use crate::{
    ElementIdExt, PositionalParentElement,
    components::{
        Toggle, ToggleVariant,
        select::{SelectItem, SelectState},
    },
    theme::{ThemeExt, ThemeLayerKind},
    utils::PixelsExt,
};

struct SelectMenuStyles {
    opacity: f32,
}

impl Default for SelectMenuStyles {
    fn default() -> Self {
        Self { opacity: 1. }
    }
}

#[derive(IntoElement)]
pub struct SelectMenu<V: 'static, I: SelectItem<Value = V> + 'static> {
    id: ElementId,
    layer: ThemeLayerKind,
    state: Arc<SelectState<V, I>>,
    styles: SelectMenuStyles,
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> SelectMenu<V, I> {
    pub fn new(id: impl Into<ElementId>, state: impl Into<Arc<SelectState<V, I>>>) -> Self {
        Self {
            id: id.into(),
            layer: ThemeLayerKind::Tertiary,
            state: state.into(),
            styles: SelectMenuStyles::default(),
        }
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.styles.opacity = opacity;
        self
    }
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> RenderOnce for SelectMenu<V, I> {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        const INSET: f32 = 4.;

        let background_color = self.layer.resolve(cx);
        let border_color = self.layer.next().resolve(cx);
        let corner_radius = cx.get_theme().layout.corner_radii.md;
        let line_height = cx.get_theme().layout.text.default_font.line_height;
        let text_size = cx.get_theme().layout.text.default_font.sizes.body.clone();
        let horizontal_padding = cx.get_theme().layout.padding.lg - px(INSET);
        let vertical_padding =
            cx.get_theme()
                .layout
                .size
                .lg
                .padding_needed_for_height(window, text_size, line_height)
                - px(INSET);

        let state = self.state.clone();

        let items = state
            .items
            .read(cx)
            .iter()
            .map(|(item_name, item)| {
                let selected = state.selected_item.read(cx).as_ref() == Some(item_name);

                Toggle::new(self.id.with_suffix("item").with_suffix(item_name))
                    .checked(selected)
                    .variant(if selected {
                        ToggleVariant::Secondary
                    } else {
                        ToggleVariant::Tertiary
                    })
                    .justify_start()
                    .rounded(corner_radius - px(INSET))
                    .child_left(item.display(window, cx))
                    .pl(horizontal_padding)
                    .pr(horizontal_padding)
                    .pt(vertical_padding)
                    .pb(vertical_padding)
                    .map(|this| {
                        let state = self.state.clone();
                        let item_name = item_name.clone();

                        this.on_click(move |checked, _window, cx| {
                            if *checked {
                                let _ = state.select_item(cx, item_name.clone()).unwrap();
                            } else {
                                let _ = state.cancel_selection(cx);
                            }
                        })
                    })
                    .into_any_element()
            })
            .collect::<SmallVec<[_; 2]>>();

        div()
            .opacity(self.styles.opacity)
            .w_full()
            .flex()
            .flex_col()
            .p(px(INSET))
            .child(
                squircle()
                    .absolute_expand()
                    .rounded(corner_radius)
                    .bg(background_color)
                    .border_color(border_color)
                    .border(px(1.))
                    .border_inside(),
            )
            .children(items)
    }
}
