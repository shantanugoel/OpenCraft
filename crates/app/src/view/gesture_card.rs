//! The gesture card (decision 0029, section 10.3): the part of the
//! inspector for one gesture of a control.
//!
//! The card is a component with no state. The caller keeps the state (which
//! card is open, which menu is open) and gives the body rows and the click
//! handlers. Thus each inspector uses the same card:
//!
//! - A card of a key, a bar panel, or a knob: [`GestureCard::new`] with the
//!   gesture name, and [`GestureCard::action`] when the gesture has an
//!   action.
//! - A swipe card of the page (#181): the same, with
//!   [`GestureCard::name_width`] [`theme::GESTURE_NAME_WIDE`].
//! - The "Turn" card of a knob (#182): the same, with the rows of the
//!   direction and of the link as body rows.
//! - A nested card of a toggle or a sequence (#183):
//!   [`GestureCard::nested`], with an empty name. A closed nested card is
//!   one row with the kind and [`GestureCard::summary`]. An open nested
//!   card has the kind button in its head, the summary, and an
//!   [`GestureCard::accessory`] at the end, for example the "⋯" button of
//!   a step.
//! - The step list of a sequence (#183): [`GestureCard::lead`], the part of
//!   the body above the form, with no padding.
//! - The button "Test the action" (#188): [`GestureCard::on_test`]. The
//!   card adds it as the last body row of a card with an action.
//!
//! The element identifiers come from the `id` of the card:
//!
//! | Identifier | Element |
//! |---|---|
//! | `{id}` | The card |
//! | `{id}-head` | The head row. A click out of the popup opens or closes the card. |
//! | `{id}-name` | The gesture name in the head row |
//! | `{id}-kind` | The action popup, the kind button of an open nested card, or the full card with no action. A click opens the kind menu. |
//! | `{id}-test` | The button "Test the action" |

use super::{controls, form, theme};
use gpui_kit::{
    assets::IconName, component::Icon, div, prelude::*, relative, AnyElement, App, ClickEvent, Div,
    FontWeight, Pixels, Rgba, SharedString, Stateful, Window,
};
use opencraft_engine::KindDescription;
use std::rc::Rc;

type Handler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// The text and the tile of the action of a card.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct KindLabel {
    name: SharedString,
    letter: SharedString,
    colour: Rgba,
    available: bool,
}

/// The size of a kind tile: in a nested card, in a gesture card and in the
/// kind menu, in a library row, or in the drag picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TileSize {
    Nested,
    Card,
    Library,
    Drag,
}

impl KindLabel {
    /// The label of a kind with a handler. The tile has the colour of the
    /// group of the kind (decision 0029, section 20). Until #190 gives the
    /// kind icons, the tile shows the first letter of the name.
    pub(super) fn new(kind: &KindDescription) -> Self {
        let name: SharedString = kind.name.clone().into();
        let letter = name
            .chars()
            .next()
            .map_or_else(|| "?".to_string(), |c| c.to_uppercase().to_string());
        Self {
            name,
            letter: letter.into(),
            colour: theme::kind_tile(&kind.group),
            available: true,
        }
    }

    /// The label of a kind with no handler (rule 8): "Not available" and a
    /// question mark.
    pub(super) fn unavailable() -> Self {
        Self {
            name: "Not available".into(),
            letter: "?".into(),
            colour: theme::KIND_TILE_UNAVAILABLE,
            available: false,
        }
    }

    /// The name of the kind.
    pub(super) fn name(&self) -> &SharedString {
        &self.name
    }

    /// The tile of the kind: 18 × 18, 20 × 20, or 24 × 24.
    pub(super) fn tile(&self, size: TileSize) -> Div {
        let (size, radius, text) = match size {
            TileSize::Nested => (
                theme::KIND_TILE_NESTED,
                theme::RADIUS_BADGE,
                theme::TEXT_BADGE,
            ),
            TileSize::Card => (theme::KIND_TILE, theme::RADIUS_SEGMENT, theme::TEXT_CAPTION),
            TileSize::Library => (
                theme::KIND_TILE_LIBRARY,
                theme::RADIUS_CONTROL,
                theme::TEXT_LABEL,
            ),
            TileSize::Drag => (
                theme::KIND_TILE_DRAG,
                theme::RADIUS_SEGMENT,
                theme::TEXT_CAPTION,
            ),
        };
        div()
            .flex_shrink_0()
            .size(size)
            .flex()
            .items_center()
            .justify_center()
            .rounded(radius)
            .bg(self.colour)
            .text_size(text)
            .font_weight(FontWeight::BOLD)
            .text_color(theme::SWITCH_KNOB_COLOUR)
            .child(self.letter.clone())
    }

    /// The tile and the name, as in the popup and in the kind menu.
    pub(super) fn content(&self, nested: bool) -> Div {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(theme::SPACE_8)
            .child(self.tile(if nested {
                TileSize::Nested
            } else {
                TileSize::Card
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .when(!self.available, |d| d.text_color(theme::TEXT_TERTIARY))
                    .child(self.name.clone()),
            )
    }
}

/// The height of the element that opens the kind menu of a card: the
/// popup of a card with an action, or the card with no action. The menu of
/// [`controls::popup_menu`] needs it.
pub(super) fn menu_button_height(action: bool, nested: bool) -> Pixels {
    match (action, nested) {
        (true, false) => theme::HEIGHT_ACTION_POPUP,
        (true, true) => theme::HEIGHT_NESTED_KIND,
        (false, false) => theme::HEIGHT_EMPTY_CARD,
        (false, true) => theme::HEIGHT_EMPTY_NESTED,
    }
}

/// One card. Make it with [`GestureCard::new`], and add the body rows with
/// `child`. The body shows only when the card has an action and is open.
#[derive(IntoElement)]
pub(super) struct GestureCard {
    id: SharedString,
    name: SharedString,
    name_width: Pixels,
    action: Option<KindLabel>,
    open: bool,
    nested: bool,
    menu_open: bool,
    menu: Option<AnyElement>,
    summary: Option<SharedString>,
    accessory: Option<AnyElement>,
    lead: Option<AnyElement>,
    body: Vec<AnyElement>,
    on_toggle: Option<Handler>,
    on_menu: Option<Handler>,
    on_test: Option<Handler>,
}

impl GestureCard {
    /// A closed card with no action. `name` is the gesture name, for
    /// example "Press". A nested card has an empty name.
    pub(super) fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_width: theme::FORM_LABEL,
            action: None,
            open: false,
            nested: false,
            menu_open: false,
            menu: None,
            summary: None,
            accessory: None,
            lead: None,
            body: Vec::new(),
            on_toggle: None,
            on_menu: None,
            on_test: None,
        }
    }

    /// The width of the gesture name: 64, or 84 for a swipe.
    pub(super) fn name_width(mut self, width: Pixels) -> Self {
        self.name_width = width;
        self
    }

    /// The gesture has an action. The popup shows `label`.
    pub(super) fn action(mut self, label: KindLabel) -> Self {
        self.action = Some(label);
        self
    }

    /// The card shows its body. A card with no action has no body.
    pub(super) fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The look of a card inside a card (section 10.5): radius 8, the
    /// background `bg-panel`, a smaller padding and tile.
    pub(super) fn nested(mut self) -> Self {
        self.nested = true;
        self
    }

    /// The kind menu of this card is open. `menu` is the menu, from
    /// [`controls::popup_menu`]. The card draws it below the popup, or below
    /// the card with no action.
    pub(super) fn menu(mut self, menu: Option<AnyElement>) -> Self {
        self.menu_open = menu.is_some();
        self.menu = menu;
        self
    }

    /// A short text at the end of the head of a nested card: the short value
    /// of a closed card (`label`, `text-secondary`), for example "⇧ ⌘ A", or
    /// the summary of an open card (`caption`, `text-tertiary`), for
    /// example "2 steps" (section 10.5).
    pub(super) fn summary(mut self, summary: impl Into<SharedString>) -> Self {
        let summary: SharedString = summary.into();
        self.summary = (!summary.is_empty()).then_some(summary);
        self
    }

    /// An element at the end of the head of an open nested card, for
    /// example the "⋯" button of a step. A click on it does not open or
    /// close the card: the element stops the click.
    pub(super) fn accessory(mut self, accessory: impl IntoElement) -> Self {
        self.accessory = Some(accessory.into_any_element());
        self
    }

    /// A part of the body above the form, with no padding, for example the
    /// step list of a sequence (section 10.5). The form of the body rows is
    /// below it, with a line between them.
    pub(super) fn lead(mut self, lead: Option<AnyElement>) -> Self {
        self.lead = lead;
        self
    }

    /// A click on the head row, out of the popup: open or close the card.
    pub(super) fn on_toggle(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    /// A click on the popup, or on the card with no action: open or close
    /// the kind menu.
    pub(super) fn on_menu(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_menu = Some(Rc::new(handler));
        self
    }

    /// The card shows the button "Test the action" as the last row of its
    /// body (decision 0029, section 10.3). A click runs the action as the
    /// gesture does.
    pub(super) fn on_test(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_test = Some(Rc::new(handler));
        self
    }

    fn name_cell(&self) -> Option<Stateful<Div>> {
        (!self.name.is_empty()).then(|| {
            let id = SharedString::from(format!("{}-name", self.id));
            let selector = id.clone();
            div()
                .id(id)
                .debug_selector(move || selector.to_string())
                .flex_shrink_0()
                .w(self.name_width)
                .text_size(theme::TEXT_LABEL)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::TEXT_PRIMARY)
                .child(self.name.clone())
        })
    }

    /// The card with no action: a dashed button of full width.
    fn empty(self) -> AnyElement {
        let id = self.id.clone();
        let kind_id = SharedString::from(format!("{id}-kind"));
        let selector = kind_id.clone();
        let on_menu = self.on_menu.clone();
        let height = menu_button_height(false, self.nested);
        let (radius, padding) = if self.nested {
            (theme::RADIUS_CARD, theme::SPACE_8)
        } else {
            (theme::RADIUS_GESTURE_CARD, theme::SPACE_10)
        };
        div()
            .id(id.clone())
            .debug_selector(move || id.to_string())
            .relative()
            .flex_shrink_0()
            .w_full()
            .child(
                div()
                    .id(kind_id)
                    .debug_selector(move || selector.to_string())
                    .h(height)
                    .px(padding)
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_8)
                    .rounded(radius)
                    .border_1()
                    .border_dashed()
                    .border_color(if self.menu_open {
                        theme::ACCENT_LINE
                    } else {
                        theme::BORDER_OVERLAY
                    })
                    .cursor_pointer()
                    .hover(|d| d.bg(theme::HOVER_ROW))
                    .children(self.name_cell())
                    .child(
                        Icon::new(IconName::Plus)
                            .size(theme::ICON_SMALL)
                            .text_color(theme::TEXT_SECONDARY),
                    )
                    .child(
                        div()
                            .text_size(theme::TEXT_LABEL)
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme::TEXT_SECONDARY)
                            .child("Add an action"),
                    )
                    .when_some(on_menu, |d, on_menu| {
                        d.on_click(move |event, window, cx| on_menu(event, window, cx))
                    }),
            )
            .children(self.menu)
            .into_any_element()
    }
}

impl GestureCard {
    /// A closed nested card: one row of 32 with the tile, the kind name,
    /// and the short value. A click opens the card.
    fn closed_nested(self, action: KindLabel) -> AnyElement {
        let id = self.id.clone();
        let head_id = SharedString::from(format!("{id}-head"));
        let head_selector = head_id.clone();
        let on_toggle = self.on_toggle.clone();
        div()
            .id(id.clone())
            .debug_selector(move || id.to_string())
            .relative()
            .flex_shrink_0()
            .w_full()
            .child(
                div()
                    .id(head_id)
                    .debug_selector(move || head_selector.to_string())
                    .h(theme::HEIGHT_EMPTY_NESTED)
                    .px(theme::SPACE_8)
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_8)
                    .rounded(theme::RADIUS_CONTROL)
                    .bg(theme::BG_CONTROL)
                    .border_1()
                    .border_color(theme::BORDER_CONTROL)
                    .text_size(theme::TEXT_CONTROL)
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .hover(|d| d.bg(theme::HOVER_CONTROL))
                    .active(|d| d.bg(theme::PRESSED))
                    .child(action.content(true))
                    .children(self.summary.map(|summary| {
                        div()
                            .flex_shrink_0()
                            .max_w(relative(0.5))
                            .truncate()
                            .text_size(theme::TEXT_LABEL)
                            .font_weight(FontWeight::NORMAL)
                            .text_color(theme::TEXT_SECONDARY)
                            .child(summary)
                    }))
                    .when_some(on_toggle, |d, on_toggle| {
                        d.on_click(move |event, window, cx| on_toggle(event, window, cx))
                    }),
            )
            .children(self.menu)
            .into_any_element()
    }

    /// The head row of an open nested card: the kind button (tile, kind
    /// name, chevron), which opens the kind menu, the summary, and the
    /// accessory.
    fn nested_head(&mut self, action: &KindLabel) -> Div {
        let kind_id = SharedString::from(format!("{}-kind", self.id));
        let selector = kind_id.clone();
        let on_menu = self.on_menu.clone();
        let menu_open = self.menu_open;
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(theme::SPACE_8)
            .child(
                div()
                    .relative()
                    .min_w_0()
                    .flex()
                    .child(
                        div()
                            .id(kind_id)
                            .debug_selector(move || selector.to_string())
                            .min_w_0()
                            .h(theme::HEIGHT_NESTED_KIND)
                            .pl(theme::SPACE_2)
                            .pr(theme::SPACE_4)
                            .flex()
                            .items_center()
                            .gap(theme::SPACE_6)
                            .rounded(theme::RADIUS_CONTROL)
                            .border_1()
                            .border_color(if menu_open {
                                theme::ACCENT_LINE
                            } else {
                                theme::TRANSPARENT
                            })
                            .text_size(theme::TEXT_CONTROL)
                            .font_weight(FontWeight::MEDIUM)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme::HOVER_ICON))
                            .active(|d| d.bg(theme::PRESSED))
                            .child(action.content(true))
                            .child(
                                Icon::new(IconName::ChevronsUpDown)
                                    .size(theme::ICON_SMALL)
                                    .text_color(theme::TEXT_SECONDARY),
                            )
                            .when_some(on_menu, |d, on_menu| {
                                d.on_click(move |event, window, cx| {
                                    // The click opens the menu, and does not
                                    // open or close the card.
                                    cx.stop_propagation();
                                    on_menu(event, window, cx)
                                })
                            }),
                    )
                    .children(self.menu.take()),
            )
            .child(div().flex_1())
            .children(self.summary.take().map(|summary| {
                div()
                    .flex_shrink_0()
                    .text_size(theme::TEXT_CAPTION)
                    .text_color(theme::TEXT_TERTIARY)
                    .child(summary)
            }))
            .children(self.accessory.take())
    }
}

impl RenderOnce for GestureCard {
    fn render(mut self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let Some(action) = self.action.clone() else {
            return self.empty();
        };
        if self.nested && !self.open {
            return self.closed_nested(action);
        }
        let id = self.id.clone();
        let head_id = SharedString::from(format!("{id}-head"));
        let head_selector = head_id.clone();
        let kind_id = format!("{id}-kind");
        let popup_height = menu_button_height(true, self.nested);
        let (radius, background, head_x, head_y) = if self.nested {
            (
                theme::RADIUS_CARD,
                theme::BG_PANEL,
                theme::SPACE_8,
                theme::SPACE_6,
            )
        } else {
            (
                theme::RADIUS_GESTURE_CARD,
                theme::BG_CARD,
                theme::SPACE_10,
                theme::SPACE_8,
            )
        };
        let on_menu = self.on_menu.clone();
        let on_toggle = self.on_toggle.clone();
        let open = self.open;
        let name = self.name_cell();
        let content = if self.nested {
            self.nested_head(&action)
        } else {
            div()
                .relative()
                .flex_1()
                .min_w_0()
                .flex()
                .child(
                    controls::popup_button(
                        kind_id,
                        popup_height,
                        self.menu_open,
                        action.content(false),
                    )
                    .when_some(on_menu, |d, on_menu| {
                        d.on_click(move |event, window, cx| {
                            // The click opens the menu, and does not open or
                            // close the card.
                            cx.stop_propagation();
                            on_menu(event, window, cx)
                        })
                    }),
                )
                .children(self.menu.take())
        };
        let head = div()
            .id(head_id)
            .debug_selector(move || head_selector.to_string())
            .px(head_x)
            .py(head_y)
            .flex()
            .items_center()
            .gap(theme::SPACE_8)
            .when(on_toggle.is_some(), |d| d.cursor_pointer())
            .children(name)
            .child(content)
            .when_some(on_toggle, |d, on_toggle| {
                d.on_click(move |event, window, cx| on_toggle(event, window, cx))
            });
        let mut body = self.body;
        if let Some(on_test) = self.on_test {
            body.push(
                form::end_row(
                    theme::FORM_LABEL,
                    theme::CARD_FORM_GAP,
                    controls::small_icon_button(
                        format!("{id}-test"),
                        IconName::Play,
                        "Test the action",
                    )
                    .on_click(move |event, window, cx| on_test(event, window, cx)),
                )
                .into_any_element(),
            );
        }
        let body_padding = if self.nested {
            theme::SPACE_8
        } else {
            theme::SPACE_10
        };
        let lead = self.lead.take();
        let has_lead = lead.is_some();
        div()
            .id(id.clone())
            .debug_selector(move || id.to_string())
            .flex_shrink_0()
            .w_full()
            .flex()
            .flex_col()
            .rounded(radius)
            .border_1()
            .border_color(theme::BORDER_FIELD)
            .bg(background)
            .child(head)
            .when(open, |d| {
                d.children(lead.map(|lead| {
                    div()
                        .border_t_1()
                        .border_color(theme::LINE_PANE)
                        .child(lead)
                }))
            })
            .when(open && !body.is_empty(), |d| {
                d.child(
                    form::form(theme::CARD_FORM_GAP)
                        .pt(body_padding)
                        .px(body_padding)
                        .pb(theme::SPACE_12)
                        .border_t_1()
                        .border_color(if has_lead {
                            theme::LINE_SECTION
                        } else {
                            theme::LINE_PANE
                        })
                        .children(body),
                )
            })
            .into_any_element()
    }
}

impl ParentElement for GestureCard {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}
