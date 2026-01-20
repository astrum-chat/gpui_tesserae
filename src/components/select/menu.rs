use std::{rc::Rc, sync::Arc};

use gpui::{
    App, ElementId, FocusHandle, InteractiveElement, ParentElement, SharedString, Styled, Window,
    deferred, div, prelude::*, px,
};
use gpui_squircle::{SquircleStyled, squircle};

use crate::{
    ElementIdExt, PositionalParentElement,
    components::{
        Toggle, ToggleVariant,
        select::{Confirm, MoveDown, MoveUp, SelectItem, SelectState},
    },
    primitives::{Clickable, Root},
    theme::{ThemeExt, ThemeLayerKind},
    utils::PixelsExt,
};

#[derive(IntoElement)]
pub struct SelectMenu<V: 'static, I: SelectItem<Value = V> + 'static> {
    id: ElementId,
    layer: ThemeLayerKind,
    state: Arc<SelectState<V, I>>,
    on_item_click: Rc<dyn Fn(bool, Arc<SelectState<V, I>>, SharedString, &mut Window, &mut App)>,
    focus_handle: Option<FocusHandle>,
}

fn default_on_item_click<V: 'static, I: SelectItem<Value = V> + 'static>(
    checked: bool,
    state: Arc<SelectState<V, I>>,
    item_name: SharedString,
    _window: &mut Window,
    cx: &mut App,
) {
    if checked {
        let _ = state.select_item(cx, item_name.clone()).unwrap();
    } else {
        let _ = state.remove_selection(cx);
    }
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> SelectMenu<V, I> {
    pub fn new(id: impl Into<ElementId>, state: impl Into<Arc<SelectState<V, I>>>) -> Self {
        Self {
            id: id.into(),
            layer: ThemeLayerKind::Tertiary,
            state: state.into(),
            on_item_click: Rc::new(default_on_item_click),
            focus_handle: None,
        }
    }

    pub fn layer(mut self, layer: ThemeLayerKind) -> Self {
        self.layer = layer;
        self
    }

    pub fn on_item_click(
        mut self,
        on_item_click: impl Fn(bool, Arc<SelectState<V, I>>, SharedString, &mut Window, &mut App)
        + 'static,
    ) -> Self {
        self.on_item_click = Rc::new(on_item_click);
        self
    }

    pub fn focus_handle(mut self, focus_handle: FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle);
        self
    }
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> RenderOnce for SelectMenu<V, I> {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let background_color = self.layer.resolve(cx);
        let border_color = self.layer.next().resolve(cx);
        let corner_radius = cx.get_theme().layout.corner_radii.md;
        let line_height = cx.get_theme().layout.text.default_font.line_height;
        let text_size = cx.get_theme().layout.text.default_font.sizes.body.clone();
        let padding = cx.get_theme().layout.padding.md;
        let horizontal_padding = cx.get_theme().layout.padding.lg - padding;
        let vertical_padding =
            cx.get_theme()
                .layout
                .size
                .lg
                .padding_needed_for_height(window, text_size, line_height)
                - padding;

        let menu_visible_transition = self.state.menu_visible_transition.clone();
        let menu_visible_delta = menu_visible_transition.evaluate(window, cx).value();

        let state_for_up = self.state.clone();
        let state_for_down = self.state.clone();
        let state_for_confirm = self.state.clone();
        let on_item_click_for_confirm = self.on_item_click.clone();

        let focus_handle = self.focus_handle.unwrap_or_else(|| {
            window
                .use_keyed_state(
                    self.id.with_suffix("state:focus_handle"),
                    cx,
                    |_window, cx| cx.focus_handle().tab_stop(true),
                )
                .read(cx)
                .clone()
        });

        //println!("is menu focus: {}", focus_handle.is_focused(window));

        // Track whether we've synced for this menu open session
        let has_synced = window.use_keyed_state(
            self.id.with_suffix("state:has_synced"),
            cx,
            |_window, _cx| false,
        );

        // Reset sync flag when menu is closed
        if menu_visible_delta == 0. && *has_synced.read(cx) {
            has_synced.update(cx, |synced, _cx| *synced = false);
        }

        deferred(
            div()
                .id(self.id.clone())
                .key_context("SelectMenu")
                .track_focus(&focus_handle)
                .on_action(move |_: &MoveUp, window, cx| {
                    state_for_up.move_highlight_up(window, cx);
                })
                .on_action(move |_: &MoveDown, window, cx| {
                    state_for_down.move_highlight_down(window, cx);
                })
                .on_action(move |_: &Confirm, window, cx| {
                    let highlighted = state_for_confirm.highlighted_item.read(cx).clone();
                    if let Some(item_name) = highlighted {
                        let selected =
                            state_for_confirm.selected_item.read(cx).as_ref() == Some(&item_name);
                        (on_item_click_for_confirm)(
                            !selected,
                            state_for_confirm.clone(),
                            item_name,
                            window,
                            cx,
                        );
                    }
                })
                .when(menu_visible_delta != 0., |this| {
                    // We only want the click event if the menu
                    // is transitioning towards the visible state.
                    if menu_visible_transition.read_goal(cx) == &true.into() {
                        let root = window
                            .root::<Root>()
                            .flatten()
                            .expect("Expected gpui_tesserae::Root to be the root view!");

                        root.update(cx, |root, cx| {
                            let state = self.state.clone();

                            root.on_any_mouse_down(move |_event, _window, cx| {
                                state.hide_menu(cx);
                            });

                            cx.notify();
                        });
                    }

                    // Only sync highlight to selection once when menu first opens
                    if !*has_synced.read(cx) {
                        self.state.sync_highlight_to_selection(cx);
                        focus_handle.focus(window, cx);
                        has_synced.update(cx, |synced, _cx| *synced = true);
                    }

                    this.opacity(menu_visible_delta)
                        .w_full()
                        .flex()
                        .flex_col()
                        .p(padding)
                        .child(
                            squircle()
                                .absolute_expand()
                                .rounded(corner_radius)
                                .bg(background_color)
                                .border_color(border_color)
                                .border(px(1.))
                                .border_inside(),
                        )
                        .children({
                            let state = self.state.clone();

                            state.items.read(cx).iter().map(|(item_name, entry)| {
                                let highlighted_item = self.state.highlighted_item.read(cx).clone();

                                let selected =
                                    self.state.selected_item.read(cx).as_ref() == Some(item_name);
                                let highlighted = highlighted_item.as_ref() == Some(item_name);

                                div()
                                    .w_full()
                                    .flex()
                                    .track_focus(&entry.focus_handle)
                                    .child(
                                        Toggle::new(
                                            self.id.with_suffix("item").with_suffix(item_name),
                                        )
                                        .checked(selected || highlighted)
                                        .variant(if highlighted {
                                            ToggleVariant::Tertiary
                                        } else {
                                            ToggleVariant::Secondary
                                        })
                                        .justify_start()
                                        .rounded(corner_radius - padding)
                                        .child_right(entry.item.display(window, cx))
                                        .pl(horizontal_padding)
                                        .pr(horizontal_padding)
                                        .pt(vertical_padding)
                                        .pb(vertical_padding)
                                        .w_full()
                                        .map(|this| {
                                            let state = self.state.clone();
                                            let item_name = item_name.clone();
                                            let on_item_click = self.on_item_click.clone();

                                            Clickable::on_click(this, move |_event, window, cx| {
                                                (on_item_click)(
                                                    !selected,
                                                    state.clone(),
                                                    item_name.clone(),
                                                    window,
                                                    cx,
                                                )
                                            })
                                        }),
                                    )
                            })
                        })
                }),
        )
    }
}
