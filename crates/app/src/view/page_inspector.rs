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
use gpui_kit::{
    assets::IconName, component::input::AnyInputState, component::Icon, relative, KeyDownEvent,
};
use opencraft_engine::{Background, BackgroundFit, NewBackground};
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
    /// The segment of the "Background" section that the user selected,
    /// before the page has a background of that kind: "Colour" before a
    /// click on a colour, or "Image" before the user chose an image. A
    /// change of the background or of the page removes it.
    background_mode: Option<BackgroundMode>,
    _subscription: Subscription,
}

/// The segments of the "Background" section (section 10.6). "Video" is a
/// LATER place: it is disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BackgroundMode {
    None,
    Colour,
    Image,
}

impl BackgroundMode {
    /// The mode of the background of the page.
    fn of(background: Option<&Background>) -> Self {
        match background {
            None => Self::None,
            Some(Background::Color(_)) => Self::Colour,
            Some(Background::Image { .. }) => Self::Image,
        }
    }
}

/// The text of a fit in the window.
fn fit_text(fit: BackgroundFit) -> &'static str {
    match fit {
        BackgroundFit::Fill => "Fill",
        BackgroundFit::Fit => "Fit",
    }
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
            background_mode: None,
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
        // The segment that the user selected is for the page and the
        // background of before.
        if changed || matches!(event, Event::Background(_)) {
            self.page_inspector.background_mode = None;
            self.inspector.fit_open = false;
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
        children.extend(self.background_section(window, cx));
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
            // The background of the page, behind the surfaces.
            .children(self.background_image.clone().map(|image| {
                img(image)
                    .debug_selector(|| "page-picture-background".into())
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(f32::from(layout.display_width) * scale))
                    .h(px(f32::from(layout.display_height) * scale))
            }))
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

impl DeviceView {
    /// The segment of the "Background" section that shows: the segment that
    /// the user selected, or the kind of the background of the page.
    pub(super) fn background_mode(&self) -> BackgroundMode {
        self.page_inspector
            .background_mode
            .unwrap_or_else(|| BackgroundMode::of(self.state.background.background.as_ref()))
    }

    /// The size of the background of the layout, in pixels. `None` for a
    /// model with no background: the page inspector then has no
    /// "Background" section.
    fn background_size(&self) -> Option<(u16, u16)> {
        self.state
            .layout
            .as_ref()?
            .surfaces
            .iter()
            .find(|s| s.id.kind == SurfaceKind::Background)
            .map(|s| (s.width, s.height))
    }

    /// Give the active page the background colour `color`. The "Colour"
    /// samples call this through `pick_colour`.
    pub(super) fn set_background_colour(&mut self, color: Color) {
        let page = self.state.page;
        if page < self.state.pages.len() {
            (self.send)(Command::SetBackground {
                page,
                background: Some(NewBackground::Color(color)),
            });
        }
    }

    /// A click on a segment of the "Background" section. "None" removes the
    /// background. "Colour" and "Image" show their controls; the page gets
    /// the background when the user picks a colour or an image.
    fn select_background_mode(
        &mut self,
        mode: BackgroundMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_kinds();
        self.focus_workspace(window, cx);
        let current = BackgroundMode::of(self.state.background.background.as_ref());
        if mode == BackgroundMode::None {
            self.page_inspector.background_mode = None;
            if current != BackgroundMode::None {
                (self.send)(Command::SetBackground {
                    page: self.state.page,
                    background: None,
                });
            }
        } else {
            self.page_inspector.background_mode = (mode != current).then_some(mode);
        }
        cx.notify();
    }

    /// The fit of the background image of the page, or the default fit.
    fn background_fit(&self) -> BackgroundFit {
        match &self.state.background.background {
            Some(Background::Image { fit, .. }) => *fit,
            _ => BackgroundFit::default(),
        }
    }

    /// Open the file dialog for the background image of the active page.
    fn background_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_kinds();
        self.focus_workspace(window, cx);
        let page = self.state.page;
        let epoch = self.state.epoch;
        let fit = self.background_fit();
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select a background image".into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            let result = paths.await;
            let _ = view.update_in(cx, |view, _, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        // A page change during the dialog drops the image.
                        if let Some(path) = paths
                            .into_iter()
                            .next()
                            .filter(|_| view.state.epoch == epoch)
                        {
                            // The engine reads, checks, and copies the file.
                            (view.send)(Command::SetBackground {
                                page,
                                background: Some(NewBackground::Image { path, fit }),
                            });
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => view
                        .state
                        .problems
                        .push("OpenCraft could not open the file dialog.".into()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The "Background" section (section 10.6): the segments "None",
    /// "Colour", "Image", and the disabled "Video", the controls of the
    /// selected segment, and the help text.
    fn background_section(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let size = self.background_size()?;
        let mode = self.background_mode();
        let segment = |id: &'static str, text: &'static str, this: BackgroundMode| {
            let selected = mode == this;
            controls::segment(id, text, selected, theme::HEIGHT_SEGMENT_INSPECTOR)
                .min_w_0()
                .when(!selected, |d| {
                    d.on_click(cx.listener(move |view, _, window, cx| {
                        view.select_background_mode(this, window, cx)
                    }))
                })
        };
        let segments = controls::segmented(
            "background-kind",
            vec![
                segment("background-none", "None", BackgroundMode::None),
                segment("background-colour", "Colour", BackgroundMode::Colour),
                segment("background-image", "Image", BackgroundMode::Image),
                controls::segment_if(
                    "background-video",
                    "Video",
                    false,
                    false,
                    theme::HEIGHT_SEGMENT_INSPECTOR,
                )
                .min_w_0()
                .tooltip(|window, cx| {
                    Tooltip::new("A video background comes in a later version.").build(window, cx)
                }),
            ],
        )
        .w_full()
        .border_1()
        .border_color(theme::BORDER_FIELD);
        let body = match mode {
            BackgroundMode::None => None,
            BackgroundMode::Colour => {
                let current = match &self.state.background.background {
                    Some(Background::Color(color)) => Some(*color),
                    _ => None,
                };
                Some(self.colour_samples(current, false, "Background", window, cx))
            }
            BackgroundMode::Image => Some(self.background_image_row(size, window, cx)),
        };
        Some(
            form::section("background", "Background", None)
                .child(segments)
                .children(body.map(|body| div().mt(theme::SPACE_12).child(body)))
                .child(
                    form::help(
                        "The background shows around and behind each key face and bar panel.",
                    )
                    .mt(theme::SPACE_10)
                    .line_height(relative(theme::NOTE_LINE_HEIGHT)),
                )
                .into_any_element(),
        )
    }

    /// The controls of "Image": the preview with the size of the display,
    /// "Choose image…", and the popup button of the fit.
    fn background_image_row(
        &self,
        (width, height): (u16, u16),
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let has_image = matches!(
            self.state.background.background,
            Some(Background::Image { .. })
        );
        let preview = div()
            .id("background-preview")
            .debug_selector(|| "background-preview".into())
            .relative()
            .flex_shrink_0()
            .w(theme::BACKGROUND_PREVIEW_WIDTH)
            .h(theme::BACKGROUND_PREVIEW_HEIGHT)
            .rounded(theme::RADIUS_CARD)
            .overflow_hidden()
            .bg(theme::DEVICE_DISPLAY)
            // The image of the engine (rule 11), only for an image: the
            // segment can show before the page has an image.
            .children(
                self.background_image
                    .clone()
                    .filter(|_| has_image)
                    .map(|image| img(image).size_full()),
            )
            // On an image, the size text has a dark backing, so that it
            // stays readable on a light or a busy image.
            .child(
                div()
                    .absolute()
                    .left(theme::BACKGROUND_BADGE_INSET)
                    .bottom(theme::BACKGROUND_BADGE_INSET)
                    .text_size(theme::TEXT_BADGE)
                    .when(has_image, |d| {
                        d.px(theme::SPACE_4)
                            .rounded(theme::RADIUS_BADGE)
                            .bg(theme::BADGE_ON_IMAGE)
                            .text_color(theme::TEXT_PRIMARY)
                    })
                    .when(!has_image, |d| d.text_color(theme::TEXT_SECONDARY))
                    .child(format!("{width} × {height}")),
            )
            // The border is a layer on top of the image.
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .rounded(theme::RADIUS_CARD)
                    .border_1()
                    .border_color(theme::BORDER_FIELD),
            );
        div()
            .flex()
            .items_start()
            .gap(theme::SPACE_12)
            .child(preview)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_6)
                    .child(
                        button("background-choose", "Choose image…")
                            .justify_center()
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.background_dialog(window, cx)
                            })),
                    )
                    .child(self.fit_popup(has_image, window, cx)),
            )
            .into_any_element()
    }

    /// The popup button of the fit, and its menu with "Fill" and "Fit". With
    /// no image, the button is disabled.
    fn fit_popup(&self, enabled: bool, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let fit = self.background_fit();
        let open = enabled && self.inspector.fit_open;
        let menu = open.then(|| {
            let rows = BackgroundFit::ALL
                .into_iter()
                .map(|item| {
                    controls::menu_row(format!("background-fit-{}", item.key()))
                        .child(
                            div()
                                .flex_shrink_0()
                                .w(theme::ICON_SMALL)
                                .when(item == fit, |d| {
                                    d.child(Icon::new(IconName::Check).size(theme::ICON_SMALL))
                                }),
                        )
                        .child(fit_text(item))
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.close_kinds();
                            view.focus_workspace(window, cx);
                            if item != view.background_fit() {
                                (view.send)(Command::SetBackgroundFit {
                                    page: view.state.page,
                                    fit: item,
                                });
                            }
                            cx.notify();
                        }))
                        .into_any_element()
                })
                .collect();
            controls::popup_menu(
                "background-fit-menu",
                controls::MenuPlace {
                    button_height: theme::HEIGHT_CONTROL,
                    width: theme::WIDTH_FIT_MENU,
                    align_end: false,
                },
                rows,
                window,
                cx.listener(|view, _, window, cx| {
                    view.close_kinds();
                    view.focus_workspace(window, cx);
                    cx.notify();
                }),
            )
        });
        let button = controls::popup_button(
            "background-fit",
            theme::HEIGHT_CONTROL,
            open,
            div().truncate().child(fit_text(fit)),
        )
        .text_size(theme::TEXT_LABEL);
        div()
            .relative()
            .flex()
            .child(if enabled {
                button.on_click(cx.listener(move |view, _, window, cx| {
                    view.close_kinds();
                    view.inspector.fit_open = !open;
                    view.focus_workspace(window, cx);
                    cx.notify();
                }))
            } else {
                button.text_color(theme::TEXT_DISABLED).cursor_default()
            })
            .children(menu)
            .into_any_element()
    }
}
