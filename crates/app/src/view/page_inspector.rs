//! The page inspector (decision 0029, section 10.6): the header with a
//! picture of the page, the "Page" section, and the "Touch bar gestures"
//! section with the swipe cards.
//!
//! The page is the active page of the engine. The commands are the same
//! commands as the commands of the sidebar, thus each one is one step of
//! the history (decision 0030). The name field sends one rename, at Return
//! or when the field loses the focus: one edit of the name is one step.
//! #191 adds the "Background" section.

use super::controls::square_button;
use super::pages::{page_title, Row};
use super::*;
use gpui_kit::{assets::IconName, component::input::AnyInputState, KeyDownEvent};
use std::cell::Cell;

/// The state of the page inspector that is not in the engine.
pub(super) struct PageInspector {
    /// The name field of the "Page" section.
    pub(super) name: Entity<InputState>,
    /// The page of the edit of the name, from the moment that the field got
    /// the focus. `None` when no edit is open.
    editing: Option<usize>,
    /// True when the next frame must scroll the inspector to the "Touch bar
    /// gestures" section. A cell, because the inspector draws with `&self`.
    scroll_to_swipes: Cell<bool>,
    _subscription: Subscription,
}

impl PageInspector {
    pub(super) fn new(window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx));
        // The section handles Return and Escape before the field
        // (`DeviceView::page_section`).
        let subscription =
            cx.subscribe_in(&name, window, |view, _, event, window, cx| match event {
                TextEvent::Focus => {
                    view.page_inspector.editing = Some(view.state.page);
                }
                TextEvent::Blur => view.save_page_name(window, cx),
                _ => {}
            });
        Self {
            name,
            editing: None,
            scroll_to_swipes: Cell::new(false),
            _subscription: subscription,
        }
    }
}

impl DeviceView {
    /// Make the page the selection, and scroll the inspector to the "Touch
    /// bar gestures" section: the "◀ Touch bar ▶" control does this
    /// (section 8.2).
    pub(super) fn select_page_swipes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.select_page(window, cx);
        self.page_inspector.scroll_to_swipes.set(true);
        cx.notify();
    }

    /// Stop the edit of the name before an event changes the pages: the
    /// page number of the edit can be a different page then. Call this
    /// before the state applies the event.
    pub(super) fn page_event(
        &mut self,
        event: &Event,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let changed = match event {
            Event::Pages { names, active, .. } => {
                *names != self.state.pages || *active != self.state.page
            }
            Event::Profiles { active, .. } => self.state.profile.as_ref() != Some(active),
            _ => false,
        };
        if changed && self.page_inspector.editing.take().is_some() {
            self.focus_workspace(window, cx);
        }
    }

    /// Show the name of the active page in the name field, when no edit is
    /// open.
    pub(super) fn sync_page_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.page_inspector.editing.is_some() {
            return;
        }
        let title = if self.state.pages.is_empty() {
            String::new()
        } else {
            page_title(&self.state.pages, self.state.page)
        };
        let name = self.page_inspector.name.clone();
        if name.read(cx).value().as_ref() != title {
            name.update(cx, |input, cx| input.set_value(title, window, cx));
        }
    }

    /// End the edit of the name. A new name that is not empty renames the
    /// page; else the field shows the name again.
    fn save_page_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.page_inspector.editing.take() else {
            return;
        };
        let name = self.page_inspector.name.read(cx).value().trim().to_string();
        // The page of the edit is the active page, and a page with no name
        // shows its number: that text is no new name.
        if page == self.state.page
            && !name.is_empty()
            && name != page_title(&self.state.pages, page)
        {
            (self.send)(Command::RenamePage { page, name });
        }
        self.sync_page_name(window, cx);
        cx.notify();
    }

    /// Return keeps the name, Escape stops the edit with no change. The
    /// focus goes back to the workspace in both cases.
    fn page_name_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "enter" => self.save_page_name(window, cx),
            "escape" => {
                self.page_inspector.editing = None;
                self.sync_page_name(window, cx);
            }
            _ => return,
        }
        cx.stop_propagation();
        self.focus_workspace(window, cx);
        cx.notify();
    }

    /// Do a page command for the active page. An open edit of the name ends
    /// first, so that its rename comes before the command.
    fn page_command(
        &mut self,
        command: impl FnOnce(&mut Self, usize, &mut Window, &mut Context<Self>),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.save_page_name(window, cx);
        self.focus_workspace(window, cx);
        let page = self.state.page;
        if page < self.state.pages.len() {
            command(self, page, window, cx);
        }
        cx.notify();
    }

    /// The page inspector in `editor`, the scroll container of the
    /// inspector. When the "◀ Touch bar ▶" control selected the page, the
    /// container scrolls to the "Touch bar gestures" section.
    pub(super) fn page_inspector(
        &self,
        editor: Stateful<Div>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let names = &self.state.pages;
        let page = self.state.page;
        let title = page_title(names, page);
        let profile = self
            .state
            .profile
            .as_ref()
            .and_then(|id| self.profile(id))
            .map(|p| p.name.clone())
            .unwrap_or_default();
        let mut children = vec![
            form::header(
                Some(self.page_picture()),
                format!("Page “{title}”"),
                format!(
                    "Page {} of {} · Profile “{profile}”",
                    page + 1,
                    names.len().max(1)
                ),
                None,
            )
            .into_any_element(),
            self.page_section(window, cx),
        ];
        if let Some(touch_bar) = self.state.touch_bar() {
            if self.page_inspector.scroll_to_swipes.take() {
                self.inspector_scroll.scroll_to_item(children.len());
            }
            children.push(
                form::section(
                    "touch-bar-gestures",
                    "Touch bar gestures",
                    Some("On this page"),
                )
                .children(self.gesture_cards(touch_bar, window, cx))
                .into_any_element(),
            );
        }
        editor.children(children)
    }

    /// The picture of the header (section 10.6): the display with the
    /// images of the key faces and the bar panels of the page, scaled down.
    fn page_picture(&self) -> AnyElement {
        let picture = div()
            .id("page-picture")
            .debug_selector(|| "page-picture".into())
            .relative()
            .flex_shrink_0()
            .w(theme::PAGE_PICTURE_WIDTH)
            .h(theme::PAGE_PICTURE_HEIGHT)
            .rounded(theme::RADIUS_CONTROL)
            .overflow_hidden()
            .bg(theme::DEVICE_DISPLAY)
            .border_1()
            .border_color(theme::LINE_PANE);
        let Some(layout) = self.state.layout.as_ref() else {
            return picture.into_any_element();
        };
        // One scale for both directions, so that the parts keep their form.
        // The border takes one point at each side.
        let inner_width = f32::from(theme::PAGE_PICTURE_WIDTH) - 2.;
        let inner_height = f32::from(theme::PAGE_PICTURE_HEIGHT) - 2.;
        let scale = (inner_width / f32::from(layout.display_width.max(1)))
            .min(inner_height / f32::from(layout.display_height.max(1)));
        let left = (inner_width - f32::from(layout.display_width) * scale) / 2.;
        let top = (inner_height - f32::from(layout.display_height) * scale) / 2.;
        picture
            .children(layout.surfaces.iter().filter_map(|surface| {
                let image = self.images.get(&surface.id)?.clone();
                Some(
                    img(image)
                        .absolute()
                        .left(px(left + f32::from(surface.x) * scale))
                        .top(px(top + f32::from(surface.y) * scale))
                        .w(px(f32::from(surface.width) * scale))
                        .h(px(f32::from(surface.height) * scale))
                        .rounded(theme::RADIUS_PAGE_PICTURE_ITEM),
                )
            }))
            .into_any_element()
    }

    /// The "Page" section: the name, the order, "Duplicate", and "Remove…".
    fn page_section(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let page = self.state.page;
        let count = self.state.pages.len();
        let up = page > 0;
        let down = page + 1 < count;
        let remove = count > 1;
        let row = |label: &str, value: AnyElement| {
            form::row(
                label.to_string(),
                theme::FORM_LABEL_WIDE,
                theme::FORM_GAP,
                value,
            )
        };
        let order = |id: &'static str, icon, label, enabled: bool, to: fn(usize) -> usize| {
            square_button(id, icon, label, enabled, theme::HEIGHT_CONTROL).when(enabled, |d| {
                d.on_click(cx.listener(move |view, _, window, cx| {
                    view.page_command(
                        |view, page, _, _| (view.send)(Command::MovePage { page, to: to(page) }),
                        window,
                        cx,
                    )
                }))
            })
        };
        form::section("page", "Page", None)
            .child(
                form::form(theme::FORM_GAP)
                    .child(row(
                        "Name",
                        div()
                            .flex_1()
                            .min_w_0()
                            // The field gets no Return and no Escape: the
                            // window would put the key into the text.
                            .capture_key_down(cx.listener(|view, event, window, cx| {
                                view.page_name_key(event, window, cx)
                            }))
                            .child(
                                controls::text_field(
                                    &AnyInputState::Input(self.page_inspector.name.clone()),
                                    theme::BG_FIELD,
                                    window,
                                    cx,
                                )
                                .id("page-name")
                                .debug_selector(|| "page-name".into()),
                            )
                            .into_any_element(),
                    ))
                    .child(row(
                        "Order",
                        div()
                            .flex()
                            .gap(theme::GAP_ITEMS)
                            .child(order(
                                "page-move-up",
                                IconName::ChevronUp,
                                "Move the page up",
                                up,
                                |page| page - 1,
                            ))
                            .child(order(
                                "page-move-down",
                                IconName::ChevronDown,
                                "Move the page down",
                                down,
                                |page| page + 1,
                            ))
                            .into_any_element(),
                    ))
                    .child(row(
                        "",
                        div()
                            .flex()
                            .gap(theme::GAP_ITEMS)
                            .child(
                                button("page-duplicate", "Duplicate")
                                    .px(theme::SPACE_10)
                                    .on_click(cx.listener(|view, _, window, cx| {
                                        view.page_command(
                                            |view, page, _, _| {
                                                (view.send)(Command::AddPage {
                                                    copy_of: Some(page),
                                                })
                                            },
                                            window,
                                            cx,
                                        )
                                    })),
                            )
                            .child(
                                button_if("page-remove", "Remove…", remove)
                                    .px(theme::SPACE_10)
                                    .when(remove, |d| {
                                        d.text_color(theme::DESTRUCTIVE_TEXT).on_click(cx.listener(
                                            |view, _, window, cx| {
                                                view.page_command(
                                                    |view, page, window, cx| {
                                                        view.confirm_remove(
                                                            Row::Page(page),
                                                            window,
                                                            cx,
                                                        )
                                                    },
                                                    window,
                                                    cx,
                                                )
                                            },
                                        ))
                                    }),
                            )
                            .into_any_element(),
                    )),
            )
            .into_any_element()
    }
}
