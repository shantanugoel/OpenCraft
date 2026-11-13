//! The "Inspector" tab of the right panel (decision 0029, section 10): the
//! inspector of the selected control, or the device inspector
//! (`device_inspector`).
//!
//! The inspector of a key face or a bar panel has the header, the
//! "Appearance" section, and the "Gestures" section with one gesture card
//! for each gesture (`actions`). A knob has the header, its gesture cards,
//! and the section of the bar panel of its feedback (`knob`). The page, with the swipes of the touch bar, has its own
//! inspector (`page_inspector`). The parts are in `form` and
//! `gesture_card`.

use super::*;
use gpui_kit::{assets::IconName, component::input::AnyInputState, component::Icon, rgb, Rgba};
use opencraft_engine::{
    label_color, ControlLayout, DeviceLayout, LabelChange, LabelPosition, SurfaceContent,
    LABEL_SIZES,
};

/// The text colours of the popover of the label colour, before the fill
/// colours of [`COLOURS`]: white and black. These are data: the profile
/// keeps them.
const TEXT_COLOURS: [u32; 2] = [0xffffff, 0x000000];

/// The state of the inspector that is not in the form of the cards.
pub(super) struct InspectorState {
    /// The popover of the fill sample "Other colour" is open.
    pub(super) colour_open: bool,
    /// The field for `#RRGGBB` in that popover.
    colour_input: Entity<InputState>,
    _colour_subscription: Subscription,
    /// The menu of the label size is open.
    size_open: bool,
    /// The popover of the label colour is open.
    text_colour_open: bool,
    /// The field for `#RRGGBB` in that popover.
    text_colour_input: Entity<InputState>,
    _text_colour_subscription: Subscription,
}

impl InspectorState {
    pub(super) fn new(window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let colour_input = cx.new(|cx| InputState::new(window, cx).placeholder("#RRGGBB"));
        let subscription =
            cx.subscribe_in(&colour_input, window, |view, input, event, window, cx| {
                if matches!(event, TextEvent::PressEnter { .. }) {
                    if let Some(colour) = parse_colour(&input.read(cx).value()) {
                        view.set_colour(Some(colour));
                        view.inspector.colour_open = false;
                        view.focus_workspace(window, cx);
                        cx.notify();
                    }
                }
            });
        let text_colour_input = cx.new(|cx| InputState::new(window, cx).placeholder("#RRGGBB"));
        let text_subscription = cx.subscribe_in(
            &text_colour_input,
            window,
            |view, input, event, window, cx| {
                if matches!(event, TextEvent::PressEnter { .. }) {
                    if let Some(colour) = parse_colour(&input.read(cx).value()) {
                        view.set_label_style(LabelChange::Color(Some(colour)));
                        view.inspector.text_colour_open = false;
                        view.focus_workspace(window, cx);
                        cx.notify();
                    }
                }
            },
        );
        Self {
            colour_open: false,
            colour_input,
            _colour_subscription: subscription,
            size_open: false,
            text_colour_open: false,
            text_colour_input,
            _text_colour_subscription: text_subscription,
        }
    }

    /// True if a menu or a popover of the "Appearance" section is open.
    pub(super) fn is_open(&self) -> bool {
        self.colour_open || self.size_open || self.text_colour_open
    }

    /// Close each menu and popover of the "Appearance" section.
    pub(super) fn close(&mut self) {
        self.colour_open = false;
        self.size_open = false;
        self.text_colour_open = false;
    }
}

/// A click handler that a function can return. A function cannot return
/// the result of `cx.listener` as `impl Fn`: see `docs/kb/gpui.md`.
type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// The click handler of a sample in the popover of the label colour.
/// `None` is "Automatic".
fn choose_text_colour(value: Option<Color>, cx: &mut Context<DeviceView>) -> ClickHandler {
    Box::new(cx.listener(move |view, _, window, cx| {
        view.set_label_style(LabelChange::Color(value));
        view.close_kinds();
        view.focus_workspace(window, cx);
        cx.notify();
    }))
}

/// The ID of the segment of a label position, and its text.
fn position_segment(position: LabelPosition) -> (&'static str, &'static str) {
    match position {
        LabelPosition::Top => ("label-position-top", "Top"),
        LabelPosition::Middle => ("label-position-middle", "Middle"),
        LabelPosition::Bottom => ("label-position-bottom", "Bottom"),
        LabelPosition::Hidden => ("label-position-none", "None"),
    }
}

/// A colour from the text `#RRGGBB` or `RRGGBB`.
pub(super) fn parse_colour(text: &str) -> Option<Color> {
    let digits = text.trim().trim_start_matches('#');
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16).ok().map(colour)
}

/// The text `#RRGGBB` of a colour.
fn colour_text(colour: Color) -> String {
    let [r, g, b] = colour.0;
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// The colour of a fill sample.
fn sample_fill(colour: Color) -> Rgba {
    let [r, g, b] = colour.0;
    rgb((u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b))
}

/// "first" to "fifth" for 1 to 5 (decision 0029, section 10.1).
fn ordinal(number: u8) -> Option<&'static str> {
    ["first", "second", "third", "fourth", "fifth"]
        .get(usize::from(number).checked_sub(1)?)
        .copied()
}

/// The second line of the header of a control: its position, or the bar
/// panel of the feedback of a knob (section 10.1).
fn position_text(control: &ControlLayout, layout: &DeviceLayout) -> String {
    match control.id.kind {
        ControlKind::Key => {
            let row = match control.row {
                1 => "Top row".to_string(),
                n => ordinal(n).map_or_else(
                    || format!("Row {n}"),
                    |o| {
                        let mut o = o.to_string();
                        o[..1].make_ascii_uppercase();
                        format!("{o} row")
                    },
                ),
            };
            let key = ordinal(control.column)
                .map_or_else(|| format!("key {}", control.column), |o| format!("{o} key"));
            format!("{row}, {key}")
        }
        ControlKind::BarPanel => ordinal(control.column).map_or_else(
            || format!("Touch bar, panel {}", control.column),
            |o| format!("Touch bar, {o} panel"),
        ),
        ControlKind::Knob => control
            .feedback_surface
            .and_then(|surface| {
                layout
                    .controls
                    .iter()
                    .find(|c| c.id.kind == ControlKind::BarPanel && c.surface == Some(surface))
            })
            .map_or_else(
                || "No bar panel".to_string(),
                |panel| format!("Feedback on {}", control_name(panel.id).to_lowercase()),
            ),
        ControlKind::TouchBar => "Swipes on this page".to_string(),
    }
}

/// The hint of the "Gestures" section.
fn gestures_hint(kind: ControlKind) -> &'static str {
    match kind {
        ControlKind::Key => "What the key does",
        ControlKind::BarPanel => "What the bar panel does",
        ControlKind::Knob => "What the knob does",
        ControlKind::TouchBar => "What the touch bar does",
    }
}

impl DeviceView {
    /// The content of the tab. It scrolls, and a scrollbar shows when it is
    /// taller than the panel. Thus the user sees that more settings are
    /// below.
    pub(super) fn inspector(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let content = if self.device.open {
            self.device_inspector(window, cx)
        } else {
            self.control_inspector(window, cx)
        };
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .child(
                content
                    .size_full()
                    .track_scroll(&self.inspector_scroll)
                    .overflow_y_scroll()
                    .flex()
                    .flex_col(),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .child(Scrollbar::vertical(&self.inspector_scroll).mode(ScrollbarMode::Always)),
            )
            .into_any_element()
    }

    /// The inspector of the selected control (section 10.1).
    fn control_inspector(&self, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let editor = div().id("editor").debug_selector(|| "editor".into());
        if self.state.page_selected {
            return self.page_inspector(editor, window, cx);
        }
        let control = self.state.selected_control.and_then(|id| {
            let layout = self.state.layout.as_ref()?;
            layout.controls.iter().find(|c| c.id == id).cloned()
        });
        let (Some(control), Some(layout)) = (control, self.state.layout.as_ref()) else {
            return editor.child(
                div()
                    .pt(theme::EMPTY_STATE_TOP)
                    .px(theme::SECTION_PADDING_SIDE)
                    .text_center()
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::TEXT_TERTIARY)
                    .child("Select a control, the page, or the device."),
            );
        };
        let name = control_name(control.id);
        editor
            .child(form::header(
                self.header_picture(&control, layout),
                name.clone(),
                position_text(&control, layout),
                (control.id.kind != ControlKind::TouchBar)
                    .then(|| self.control_more_button(control.id, cx)),
            ))
            .children(
                self.state
                    .selected
                    .map(|surface| self.appearance(surface, window, cx)),
            )
            .child(
                form::section("gestures", "Gestures", Some(gestures_hint(control.id.kind)))
                    .children(if control.id.kind == ControlKind::Knob {
                        self.knob_cards(control.id, window, cx)
                    } else {
                        self.gesture_cards(control.id, window, cx)
                    }),
            )
            // A knob also edits the bar panel of its feedback (section 10.4).
            .children(
                (control.id.kind == ControlKind::Knob)
                    .then(|| self.knob_panel(control.id, layout, window, cx))
                    .flatten(),
            )
    }

    /// The picture of the header: the image of the surface from the engine,
    /// or a knob with the ring of the canvas.
    fn header_picture(&self, control: &ControlLayout, layout: &DeviceLayout) -> Option<AnyElement> {
        if control.id.kind == ControlKind::Knob {
            return Some(
                div()
                    .flex_shrink_0()
                    .size(theme::HEADER_PICTURE)
                    .rounded_full()
                    .bg(theme::DEVICE_KNOB)
                    .border(theme::HEADER_KNOB_RING)
                    // The same ring colour as the knob on the canvas.
                    .border_color(super::canvas::knob_ring(
                        self.state.device_settings.as_ref(),
                        control.id,
                    ))
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .mt(theme::SPACE_4)
                            .size(theme::HEADER_KNOB_MARK)
                            .rounded_full()
                            .bg(theme::DEVICE_KNOB_DOT),
                    )
                    .into_any_element(),
            );
        }
        let surface = layout.surface(control.surface?)?;
        let width = theme::HEADER_PICTURE;
        let (height, radius) = if surface.id.kind == SurfaceKind::KeyFace {
            (width, theme::RADIUS_POPOVER)
        } else {
            (
                width * (f32::from(surface.height) / f32::from(surface.width.max(1))),
                theme::RADIUS_CONTROL,
            )
        };
        Some(
            div()
                .flex_shrink_0()
                .w(width)
                .h(height)
                .rounded(radius)
                .overflow_hidden()
                .bg(theme::DEVICE_DISPLAY)
                .border_1()
                .border_color(theme::DEVICE_SURFACE_BORDER)
                .children(
                    self.images
                        .get(&surface.id)
                        .cloned()
                        .map(|image| img(image).size_full().rounded(radius)),
                )
                .into_any_element(),
        )
    }

    /// Change one label option of the selected surface.
    fn set_label_style(&mut self, change: LabelChange) {
        if let Some(surface) = self.state.edit_surface() {
            (self.send)(Command::SetLabelStyle { surface, change });
        }
    }

    /// The "Appearance" section of a key face or a bar panel (section
    /// 10.2). #190 changes "Image" to "Icon".
    fn appearance(&self, id: SurfaceId, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        form::section("appearance", "Appearance", None)
            .child(form::form(theme::FORM_GAP).children(self.appearance_rows(id, window, cx)))
            .into_any_element()
    }

    /// The rows of "Appearance": "Image", "Title", "Position", "Text", and
    /// "Fill". The "Bar panel" section of a knob (section 10.4) also has
    /// them.
    pub(super) fn appearance_rows(
        &self,
        id: SurfaceId,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let content = self.state.surfaces.get(&id).map(|s| &s.content);
        let has_image = content.is_some_and(|c| c.image.is_some());
        let content = content.cloned().unwrap_or_default();
        let row = |label: &str, value: AnyElement| {
            form::row(label.to_string(), theme::FORM_LABEL, theme::FORM_GAP, value)
        };
        vec![
            row(
                "Image",
                div()
                    .flex()
                    .gap(theme::GAP_ITEMS)
                    .child(
                        button("choose-image", "Image file…")
                            .on_click(cx.listener(|view, _, window, cx| view.dialog(window, cx))),
                    )
                    .child(
                        button_if("remove-image", "Remove", has_image).when(has_image, |d| {
                            d.on_click(cx.listener(move |view, _, _, _| {
                                (view.send)(Command::SetImage {
                                    surface: id,
                                    path: None,
                                })
                            }))
                        }),
                    )
                    .into_any_element(),
            ),
            row(
                "Title",
                controls::text_field(
                    &AnyInputState::Input(self.input.clone()),
                    theme::BG_FIELD,
                    window,
                    cx,
                )
                .id("title-field")
                .debug_selector(|| "title-field".into())
                .into_any_element(),
            ),
            row("Position", self.label_position(&content, cx)),
            row("Text", self.label_text(&content, window, cx)),
            row("Fill", self.fill_samples(id, window, cx)),
        ]
        .into_iter()
        .map(IntoElement::into_any_element)
        .collect()
    }

    /// The row "Position": a segmented control with "Top", "Middle",
    /// "Bottom", and "None". The selected segment is the position that the
    /// surface shows, also when the profile has no position (the engine
    /// gives the automatic position).
    fn label_position(&self, content: &SurfaceContent, cx: &mut Context<Self>) -> AnyElement {
        let current = content.label_style.position_for(content.image.is_some());
        let segments = LabelPosition::ALL
            .into_iter()
            .map(|position| {
                let (id, text) = position_segment(position);
                let selected = position == current;
                controls::segment(id, text, selected, theme::HEIGHT_SEGMENT_INSPECTOR)
                    .min_w_0()
                    .when(!selected, |d| {
                        d.on_click(cx.listener(move |view, _, window, cx| {
                            view.set_label_style(LabelChange::Position(Some(position)));
                            view.focus_workspace(window, cx);
                            cx.notify();
                        }))
                    })
            })
            .collect();
        // In a form, the track has the full width and the border
        // `border-field`, so that it is as high as a text field.
        controls::segmented("label-position", segments)
            .flex_1()
            .min_w_0()
            .border_1()
            .border_color(theme::BORDER_FIELD)
            .into_any_element()
    }

    /// The row "Text": the popup button of the size, the colour button, and
    /// the toggle button "B" for bold.
    fn label_text(
        &self,
        content: &SurfaceContent,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let style = content.label_style;
        let bold = style.bold;
        div()
            .flex()
            .items_center()
            .gap(theme::GAP_ITEMS)
            .child(self.size_popup(style.size, window, cx))
            .child(self.text_colour(content, window, cx))
            .child(
                controls::toggle_button(
                    "label-bold",
                    div()
                        .text_size(theme::TEXT_BODY)
                        .font_weight(gpui_kit::FontWeight::BOLD)
                        .child("B"),
                    bold,
                    "Bold",
                )
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.set_label_style(LabelChange::Bold(!bold));
                    view.close_kinds();
                    view.focus_workspace(window, cx);
                    cx.notify();
                })),
            )
            .into_any_element()
    }

    /// The popup button of the label size, for example "20 pt", and its
    /// menu with the sizes of [`LABEL_SIZES`].
    fn size_popup(&self, size: u8, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let open = self.inspector.size_open;
        let menu = open.then(|| {
            let rows = LABEL_SIZES
                .into_iter()
                .map(|item| {
                    controls::menu_row(format!("label-size-{item}"))
                        .child(
                            div()
                                .flex_shrink_0()
                                .w(theme::ICON_SMALL)
                                .when(item == size, |d| {
                                    d.child(Icon::new(IconName::Check).size(theme::ICON_SMALL))
                                }),
                        )
                        .child(format!("{item} pt"))
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.set_label_style(LabelChange::Size(item));
                            view.close_kinds();
                            view.focus_workspace(window, cx);
                            cx.notify();
                        }))
                        .into_any_element()
                })
                .collect();
            controls::popup_menu(
                "label-size-menu",
                controls::MenuPlace {
                    button_height: theme::HEIGHT_CONTROL,
                    width: theme::WIDTH_SIZE_MENU,
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
        div()
            .relative()
            .flex_shrink_0()
            .w(theme::WIDTH_SIZE_POPUP)
            .flex()
            .child(
                controls::popup_button(
                    "label-size",
                    theme::HEIGHT_CONTROL,
                    open,
                    div().truncate().child(format!("{size} pt")),
                )
                .text_size(theme::TEXT_LABEL)
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.close_kinds();
                    view.inspector.size_open = !open;
                    view.focus_workspace(window, cx);
                    cx.notify();
                })),
            )
            .children(menu)
            .into_any_element()
    }

    /// The colour button of the label, and its popover: "Automatic", white,
    /// black, the fill colours, and a field for `#RRGGBB`.
    fn text_colour(
        &self,
        content: &SurfaceContent,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let open = self.inspector.text_colour_open;
        let current = content.label_style.color;
        let shown = label_color(content);
        let tooltip = match current {
            Some(colour) => format!("Text colour {}", colour_text(colour)),
            None => format!("Text colour: automatic ({})", colour_text(shown)),
        };
        let popover = open.then(|| {
            let automatic = controls::colour_sample(
                "label-colour-automatic",
                None,
                current.is_none(),
                Some("Automatic: black or white for the fill".into()),
            )
            .flex()
            .items_center()
            .justify_center()
            .text_size(theme::TEXT_CAPTION)
            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
            .text_color(theme::TEXT_SECONDARY)
            .child("A")
            .on_click(choose_text_colour(None, cx));
            let samples: Vec<_> = TEXT_COLOURS
                .into_iter()
                .chain(COLOURS)
                .enumerate()
                .map(|(index, value)| {
                    controls::colour_sample(
                        format!("label-colour-{index}"),
                        Some(rgb(value)),
                        current == Some(colour(value)),
                        Some(format!("Text #{value:06X}").into()),
                    )
                    .on_click(choose_text_colour(Some(colour(value)), cx))
                })
                .collect();
            controls::popup_menu(
                "label-colour-popover",
                // The trailing edge on the button: the popover then fits in
                // the right panel at its smallest width.
                controls::MenuPlace {
                    button_height: theme::HEIGHT_CONTROL,
                    width: theme::WIDTH_COLOUR_POPOVER,
                    align_end: true,
                },
                vec![div()
                    .capture_key_down(cx.listener(
                        |view, event: &gpui_kit::KeyDownEvent, window, cx| {
                            // Escape in the field closes the popover with no change.
                            if event.keystroke.key == "escape" {
                                cx.stop_propagation();
                                view.close_kinds();
                                view.focus_workspace(window, cx);
                                cx.notify();
                            }
                        },
                    ))
                    .p(theme::SPACE_6)
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_8)
                    .child(
                        div()
                            .text_size(theme::TEXT_LABEL)
                            .text_color(theme::TEXT_SECONDARY)
                            .child("Text colour"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(theme::SPACE_6)
                            .child(automatic)
                            .children(samples),
                    )
                    .child(
                        controls::text_field(
                            &AnyInputState::Input(self.inspector.text_colour_input.clone()),
                            theme::BG_FIELD,
                            window,
                            cx,
                        )
                        .id("label-colour-field")
                        .debug_selector(|| "label-colour-field".into()),
                    )
                    .child(form::help("Type #RRGGBB and push Return."))
                    .into_any_element()],
                window,
                cx.listener(|view, _, window, cx| {
                    view.close_kinds();
                    view.focus_workspace(window, cx);
                    cx.notify();
                }),
            )
        });
        div()
            .relative()
            .flex_shrink_0()
            .child(
                controls::colour_button("label-colour", sample_fill(shown), open, tooltip)
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.close_kinds();
                        if open {
                            view.focus_workspace(window, cx);
                        } else {
                            view.inspector.text_colour_open = true;
                            let text = current.map(colour_text).unwrap_or_default();
                            let input = view.inspector.text_colour_input.clone();
                            input.update(cx, |input, cx| {
                                input.set_value(text, window, cx);
                                input.focus(window, cx);
                            });
                        }
                        cx.notify();
                    })),
            )
            .children(popover)
            .into_any_element()
    }

    /// The fill samples: "No fill", the colours of [`COLOURS`], and "Other
    /// colour", which opens a popover with a field for `#RRGGBB`.
    fn fill_samples(&self, id: SurfaceId, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let current = self.state.surfaces.get(&id).and_then(|s| s.content.color);
        let custom = current.filter(|c| !COLOURS.iter().any(|&v| colour(v) == *c));
        let other = controls::colour_sample(
            "other-colour",
            custom.map(sample_fill),
            custom.is_some(),
            (!self.inspector.colour_open).then(|| "Other colour".into()),
        )
        .when(custom.is_none(), |d| {
            d.flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_dashed()
                .border_color(theme::BORDER_SAMPLE_OTHER)
                .text_size(theme::TEXT_BODY)
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .text_color(theme::TEXT_SECONDARY)
                .child("+")
        })
        .on_click(cx.listener(move |view, _, window, cx| {
            let open = !view.inspector.colour_open;
            view.close_kinds();
            if open {
                view.inspector.colour_open = true;
                let text = custom.map(colour_text).unwrap_or_default();
                let input = view.inspector.colour_input.clone();
                input.update(cx, |input, cx| {
                    input.set_value(text, window, cx);
                    input.focus(window, cx);
                });
            } else {
                view.focus_workspace(window, cx);
            }
            cx.notify();
        }))
        .children(self.inspector.colour_open.then(|| {
            controls::popup_menu(
                "other-colour-popover",
                controls::MenuPlace {
                    button_height: theme::SAMPLE,
                    width: theme::WIDTH_COLOUR_POPOVER,
                    align_end: true,
                },
                vec![div()
                    .capture_key_down(cx.listener(
                        |view, event: &gpui_kit::KeyDownEvent, window, cx| {
                            // Escape in the field closes the popover with no change.
                            if event.keystroke.key == "escape" {
                                cx.stop_propagation();
                                view.close_kinds();
                                view.focus_workspace(window, cx);
                                cx.notify();
                            }
                        },
                    ))
                    .p(theme::SPACE_6)
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_8)
                    .child(
                        div()
                            .text_size(theme::TEXT_LABEL)
                            .text_color(theme::TEXT_SECONDARY)
                            .child("Other colour"),
                    )
                    .child(
                        controls::text_field(
                            &AnyInputState::Input(self.inspector.colour_input.clone()),
                            theme::BG_FIELD,
                            window,
                            cx,
                        )
                        .id("other-colour-field")
                        .debug_selector(|| "other-colour-field".into()),
                    )
                    .child(form::help("Type #RRGGBB and push Return."))
                    .into_any_element()],
                window,
                cx.listener(|view, _, window, cx| {
                    view.close_kinds();
                    view.focus_workspace(window, cx);
                    cx.notify();
                }),
            )
        }));
        div()
            .flex()
            .flex_wrap()
            .gap(theme::SPACE_6)
            .child(
                controls::colour_sample(
                    "no-colour",
                    None,
                    current.is_none(),
                    Some("No fill".into()),
                )
                .on_click(cx.listener(|view, _, _, cx| {
                    view.set_colour(None);
                    cx.notify();
                })),
            )
            .children(COLOURS.into_iter().enumerate().map(|(index, value)| {
                controls::colour_sample(
                    format!("colour-{index}"),
                    Some(rgb(value)),
                    current == Some(colour(value)),
                    Some(format!("Fill #{value:06X}").into()),
                )
                .on_click(cx.listener(move |view, _, _, _| view.set_colour(Some(colour(value)))))
            }))
            .child(other)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_text_has_six_hex_digits() {
        assert_eq!(parse_colour("#1A6FD8"), Some(colour(0x1a6fd8)));
        assert_eq!(parse_colour(" 1a6fd8 "), Some(colour(0x1a6fd8)));
        for text in ["", "#12345", "#1234567", "#GGGGGG", "+12345"] {
            assert_eq!(parse_colour(text), None, "{text}");
        }
        assert_eq!(colour_text(colour(0x0a0b0c)), "#0A0B0C");
    }

    #[test]
    fn the_position_text_follows_the_layout() {
        let Event::Layout(layout) = crate::state::tests::layout() else {
            panic!()
        };
        let text = |kind| {
            let control = layout.controls.iter().find(|c| c.id.kind == kind).unwrap();
            position_text(control, &layout)
        };
        assert_eq!(text(ControlKind::Key), "Top row, first key");
        assert_eq!(text(ControlKind::BarPanel), "Touch bar, first panel");
        assert_eq!(text(ControlKind::Knob), "Feedback on bar panel 2");
        assert_eq!(ordinal(6), None);
        assert_eq!(ordinal(0), None);
    }
}
