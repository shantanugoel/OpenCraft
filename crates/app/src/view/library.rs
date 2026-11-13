//! The "Library" tab of the right panel (decision 0029, section 11).
//!
//! The library lists the action kinds of the engine in their groups. The
//! groups and their order come only from `Event::Kinds`: the window has no
//! list of kinds (rule 8). A kind with no handler is not in that event, thus
//! it is not in the library. The kind menu of a gesture card uses the same
//! groups ([`groups`]).
//!
//! A search field filters the kinds by name. A click on a row sets the kind
//! on the open gesture card of the selected control, and the status bar
//! tells what changed (section 6.1).
//!
//! A row can also go onto a control with a drag (section 12). The row gives
//! a [`KindDrag`] and the [`DragPicture`]; the canvas has the drop targets.
//! A drop and a click use the same path ([`DeviceView::assign_to`]). A drop
//! is one step "Drop Action" of the history (decision 0030).

use super::gesture_card::{KindLabel, TileSize};
use super::*;
use crate::state::gestures;
use gpui_kit::{CursorStyle, FontWeight, Pixels, Point};
use opencraft_engine::{GestureKind, KindDescription};
use std::cell::RefCell;

/// The status bar text after a click in the library with no selected
/// control.
pub(super) const SELECT_FIRST: &str = "Select a control first";

/// The group name of a kind that gives no group.
pub(super) const OTHER_GROUP: &str = "Other";

/// The name of the step of the history that a drop makes (decision 0029,
/// section 13).
pub(super) const DROP_STEP: &str = "Drop Action";

/// The state of the library tab.
pub(super) struct LibraryState {
    /// The search field.
    pub(super) search: Entity<InputState>,
    /// The scroll position of the list.
    scroll: ScrollHandle,
    /// The kind of the row in a drag. It goes back to `None` when the drag
    /// picture goes away, also after a drop outside a control.
    pub(super) dragging: Option<String>,
    _subscription: Subscription,
}

/// The data of a drag from the library: the kind of the row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct KindDrag {
    pub(super) kind: String,
}

/// The picture at the pointer during a drag (section 12): a chip with the
/// kind tile and the kind name.
pub(super) struct DragPicture {
    label: KindLabel,
    /// The point of the press in the row. GPUI puts the top-left corner of
    /// the picture at the pointer minus this offset (kb/gpui.md).
    offset: Point<Pixels>,
}

impl Render for DragPicture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // GPUI puts the corner of this element at the pointer minus the
        // offset. The margin moves the chip to its place from the pointer.
        let (x, y) = theme::DRAG_PICTURE_OFFSET;
        div().child(
            self.label
                .drag_chip()
                .ml(self.offset.x + x)
                .mt(self.offset.y + y),
        )
    }
}

impl LibraryState {
    /// `count` is the number of kinds in the library now.
    pub(super) fn new(count: usize, window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder(count)));
        let subscription = cx.subscribe_in(&search, window, |_, _, event, _, cx| {
            if matches!(event, TextEvent::Change) {
                cx.notify();
            }
        });
        Self {
            search,
            scroll: ScrollHandle::new(),
            dragging: None,
            _subscription: subscription,
        }
    }
}

/// The groups of `kinds` that have a kind for which `show` is true: the
/// name of each group and the positions in `kinds` of its kinds. The groups
/// are in the order of their first kind, and the kinds are in the order of
/// `kinds` (section 11).
pub(super) fn groups(
    kinds: &[KindDescription],
    show: impl Fn(&KindDescription) -> bool,
) -> Vec<(SharedString, Vec<usize>)> {
    let mut groups: Vec<(SharedString, Vec<usize>)> = Vec::new();
    for (index, kind) in kinds.iter().enumerate() {
        let name = if kind.group.trim().is_empty() {
            OTHER_GROUP
        } else {
            kind.group.as_str()
        };
        match groups.iter_mut().find(|(group, _)| group == name) {
            Some((_, members)) if show(kind) => members.push(index),
            Some(_) => {}
            None => groups.push((
                name.to_string().into(),
                if show(kind) { vec![index] } else { Vec::new() },
            )),
        }
    }
    groups.retain(|(_, members)| !members.is_empty());
    groups
}

/// True if the name of `kind` contains `text`, with no difference between
/// capital and small letters. An empty text matches each kind.
pub(super) fn matches(kind: &KindDescription, text: &str) -> bool {
    kind.name
        .to_lowercase()
        .contains(&text.trim().to_lowercase())
}

/// The placeholder of the search field: "Search *N* actions".
pub(super) fn placeholder(count: usize) -> String {
    match count {
        1 => "Search 1 action".into(),
        n => format!("Search {n} actions"),
    }
}

/// The text with no result: "No action matches “*text*”."
pub(super) fn no_result(text: &str) -> String {
    format!("No action matches “{}”.", text.trim())
}

/// The gesture that a click in the library or a drop sets when no card is
/// open: the first gesture of the control (section 12). For a knob it is
/// "Turn": the binding "Turn right" stands for the "Turn" card, and
/// [`DeviceView::set_turn`] writes the two turn bindings.
pub(super) fn first_gesture(control: ControlKind) -> Option<(GestureKind, &'static str)> {
    if control == ControlKind::Knob {
        return Some((GestureKind::TurnRight, "Turn"));
    }
    gestures(control).first().copied()
}

/// The status bar text after a click in the library or a drop, for example
/// "Key 8 gets “Open an application” for Press" (section 6.1).
pub(super) fn assigned_text(control: ControlId, kind: &str, gesture: &str) -> String {
    format!("{} gets “{kind}” for {gesture}", control_name(control))
}

impl DeviceView {
    /// The placeholder of the search field follows the count of the kinds.
    pub(super) fn sync_library(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = placeholder(self.state.kinds.len());
        self.library
            .search
            .update(cx, |search, cx| search.set_placeholder(text, window, cx));
    }

    /// Set `kind`, with its default settings, on the open gesture card of
    /// the selected control. With no open card, set it on the first gesture
    /// of the control, and open that card. With no selected control, only
    /// show "Select a control first". The right panel stays on its tab.
    pub(super) fn assign_kind(&mut self, kind: &str, window: &mut Window, cx: &mut Context<Self>) {
        // The page as the selection has the touch bar as its control, for
        // the swipes. It is not a selected control (section 11).
        let control = self.framed_control().filter(|_| !self.state.page_selected);
        let Some(control) = control else {
            self.show_message(SELECT_FIRST, window, cx);
            self.focus_workspace(window, cx);
            return;
        };
        self.assign_to(control, kind, None, window, cx);
    }

    /// A drop of `kind` on `control` (section 12). The canvas selected the
    /// control first. The drop uses the first gesture of the control, also
    /// when a different card is open, and it is one step "Drop Action" of
    /// the history.
    pub(super) fn drop_kind(
        &mut self,
        control: ControlId,
        kind: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.library.dragging = None;
        if self.framed_control() == Some(control) {
            self.assign_to(control, kind, Some(DROP_STEP), window, cx);
        }
        cx.notify();
    }

    /// Set `kind`, with its default settings, on a gesture of the selected
    /// `control`, and show the message of section 6.1. A click
    /// (`drop` is `None`) uses the open card, else the first gesture. A
    /// drop (`drop` is the name of the step) always uses the first gesture,
    /// and its commands go to the engine as one step with that name.
    fn assign_to(
        &mut self,
        control: ControlId,
        kind: &str,
        drop: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The open card can be the "Turn" card of a knob, or the "Tap" card
        // of its bar panel (section 10.4).
        let open = self
            .state
            .gesture
            .filter(|_| drop.is_none())
            .and_then(|open| Some((open, self.card_name(control, open)?)));
        let first = open.is_none();
        let (Some((gesture, gesture_name)), Some(name)) = (
            open.or_else(|| first_gesture(control.kind)),
            self.state
                .kinds
                .iter()
                .find(|k| k.kind == kind)
                .map(|k| k.name.clone()),
        ) else {
            return;
        };
        self.close_kinds();
        let set = |view: &mut Self| {
            if first && control.kind == ControlKind::Knob {
                view.set_turn(control, Some(kind.to_string()));
            } else {
                view.choose_card(gesture, Some(kind.to_string()));
            }
        };
        match drop {
            Some(step) => self.one_step(step, set),
            None => set(self),
        }
        self.sync_form(window, cx);
        let target = self
            .state
            .binding_of(gesture)
            .map_or(control, |b| b.control);
        self.show_message(assigned_text(target, &name, gesture_name), window, cx);
        self.focus_workspace(window, cx);
    }

    /// Do `change`, and send the commands that it sends as one
    /// `Command::Group` with the name `name`: one step of the history
    /// (decision 0030). A group in `change` gives its commands to this
    /// group. A change that sends nothing sends no group.
    fn one_step(&mut self, name: &str, change: impl FnOnce(&mut Self)) {
        let commands: Rc<RefCell<Vec<Command>>> = Rc::default();
        let collect = commands.clone();
        let send = std::mem::replace(
            &mut self.send,
            Rc::new(move |command| collect.borrow_mut().push(command)),
        );
        change(self);
        self.send = send;
        let commands: Vec<Command> = commands
            .take()
            .into_iter()
            .flat_map(|command| match command {
                Command::Group { commands, .. } => commands,
                command => vec![command],
            })
            .collect();
        if !commands.is_empty() {
            (self.send)(Command::Group {
                name: name.into(),
                commands,
            });
        }
    }

    /// True while a row of the library is in a drag. A drag of a different
    /// kind, for example of a pane divider, is not a drag of the library.
    pub(super) fn kind_drag(&self, cx: &App) -> bool {
        self.library.dragging.is_some() && cx.has_active_drag()
    }

    pub(super) fn library(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let text = self.library.search.read(cx).value().to_string();
        let groups = groups(&self.state.kinds, |kind| matches(kind, &text));
        let list = if groups.is_empty() && !text.trim().is_empty() {
            div()
                .id("library-no-result")
                .debug_selector(|| "library-no-result".into())
                .pt(theme::EMPTY_STATE_TOP)
                .px(theme::SECTION_PADDING_SIDE)
                .text_center()
                .text_size(theme::TEXT_LABEL)
                .text_color(theme::TEXT_TERTIARY)
                .child(no_result(&text))
                .into_any_element()
        } else {
            div()
                .pt(theme::SPACE_6)
                .px(theme::SPACE_10)
                .pb(theme::SPACE_12)
                .flex()
                .flex_col()
                .children(
                    groups
                        .into_iter()
                        .enumerate()
                        .map(|(number, (name, members))| {
                            self.library_group(number, name, members, cx)
                        }),
                )
                .into_any_element()
        };
        div()
            .id("library")
            .debug_selector(|| "library".into())
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_shrink_0()
                    .py(theme::SPACE_12)
                    .px(theme::SPACE_16)
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_10)
                    .border_b_1()
                    .border_color(theme::LINE_SECTION)
                    // The filter chips are a LATER place (section 11): while
                    // "Actions" is the only filter, the row is not drawn.
                    .child(controls::search_field(
                        "library-search",
                        &self.library.search,
                        theme::HEIGHT_SEARCH_FIELD,
                        window,
                        cx,
                    )),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("library-list")
                            .debug_selector(|| "library-list".into())
                            .size_full()
                            .track_scroll(&self.library.scroll)
                            .overflow_y_scroll()
                            .child(list),
                    )
                    .child(div().absolute().top_0().left_0().size_full().child(
                        Scrollbar::vertical(&self.library.scroll).mode(ScrollbarMode::Always),
                    )),
            )
            .into_any_element()
    }

    /// One group of the library: its header and its rows.
    fn library_group(
        &self,
        number: usize,
        name: SharedString,
        members: Vec<usize>,
        cx: &mut Context<Self>,
    ) -> Div {
        let drag = self
            .library
            .dragging
            .as_deref()
            .filter(|_| cx.has_active_drag());
        div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_1)
            .pt(theme::SPACE_8)
            .child(
                controls::group_title(name)
                    .debug_selector(move || format!("library-group-{number}")),
            )
            .children(members.into_iter().map(|index| {
                let kind = &self.state.kinds[index];
                let label = KindLabel::new(kind);
                let id = kind.kind.clone();
                let in_drag = drag.is_some_and(|d| *d == kind.kind);
                let view = cx.entity().downgrade();
                let picture = label.clone();
                div()
                    .id(SharedString::from(format!("library-{index}")))
                    .debug_selector(move || format!("library-{index}"))
                    .flex_shrink_0()
                    .w_full()
                    .h(theme::HEIGHT_LIBRARY_ROW)
                    .px(theme::SPACE_8)
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_10)
                    .rounded(theme::RADIUS_TRACK)
                    .border_1()
                    .border_color(gpui_kit::transparent_black())
                    .text_size(theme::TEXT_BODY)
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_grab()
                    .map(|d| {
                        if in_drag {
                            // The row in a drag (section 11).
                            d.bg(theme::ACCENT_TINT_DRAG)
                                .border_dashed()
                                .border_color(theme::ACCENT_LINE)
                                .text_color(theme::TEXT_SECONDARY)
                        } else if drag.is_some() {
                            // No hover on the other rows during a drag.
                            d
                        } else {
                            d.hover(|d| d.bg(theme::HOVER_ROW))
                                .active(|d| d.bg(theme::PRESSED))
                        }
                    })
                    .on_drag(
                        KindDrag { kind: id.clone() },
                        move |drag: &KindDrag, offset, window, cx| {
                            let _ = view.update(cx, |view, cx| {
                                view.library.dragging = Some(drag.kind.clone());
                                cx.notify();
                            });
                            // The closed hand while the row is in the drag.
                            window.defer(cx, |window, cx| {
                                cx.set_active_drag_cursor_style(CursorStyle::ClosedHand, window);
                            });
                            let label = picture.clone();
                            let picture = cx.new(|_| DragPicture { label, offset });
                            // The drag picture goes away at the end of each
                            // drag, also with no drop.
                            let view = view.clone();
                            cx.observe_release(&picture, move |_, cx| {
                                let _ = view.update(cx, |view, cx| {
                                    view.library.dragging = None;
                                    cx.notify();
                                });
                            })
                            .detach();
                            picture
                        },
                    )
                    .child(label.tile(TileSize::Library))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(label.name().clone()),
                    )
                    .children(kind.help.clone().map(|help| {
                        div()
                            .flex_shrink_0()
                            .text_size(theme::TEXT_CAPTION)
                            .font_weight(FontWeight::NORMAL)
                            .text_color(theme::TEXT_TERTIARY)
                            .child(help)
                    }))
                    .on_click(
                        cx.listener(move |view, _, window, cx| view.assign_kind(&id, window, cx)),
                    )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(id: &str, name: &str, group: &str) -> KindDescription {
        KindDescription {
            kind: id.into(),
            name: name.into(),
            group: group.into(),
            ..Default::default()
        }
    }

    #[test]
    fn the_groups_follow_the_order_of_the_kinds_and_the_search() {
        let kinds = [
            kind("a", "Open an application", "System"),
            kind("b", "Media key", "Media"),
            kind("c", "Type text", "System"),
            kind("d", "Plug-in action", ""),
        ];
        let all = groups(&kinds, |_| true);
        assert_eq!(
            all,
            [
                ("System".into(), vec![0, 2]),
                ("Media".into(), vec![1]),
                (OTHER_GROUP.into(), vec![3]),
            ]
        );
        // No difference between capital and small letters. A group with no
        // kind does not show.
        assert_eq!(
            groups(&kinds, |k| matches(k, "TEXT")),
            [("System".into(), vec![2])]
        );
        assert!(groups(&kinds, |k| matches(k, "nothing")).is_empty());
        assert_eq!(placeholder(11), "Search 11 actions");
        assert_eq!(no_result("xyz "), "No action matches “xyz”.");
    }

    #[test]
    fn a_click_with_no_open_card_uses_the_first_gesture_of_the_control() {
        assert_eq!(
            first_gesture(ControlKind::Key),
            Some((GestureKind::Press, "Press"))
        );
        assert_eq!(
            first_gesture(ControlKind::BarPanel),
            Some((GestureKind::Tap, "Tap"))
        );
        assert_eq!(
            first_gesture(ControlKind::Knob),
            Some((GestureKind::TurnRight, "Turn"))
        );
        assert_eq!(
            assigned_text(
                ControlId::new(ControlKind::Key, 8),
                "Open an application",
                "Press"
            ),
            "Key 8 gets “Open an application” for Press"
        );
    }
}
