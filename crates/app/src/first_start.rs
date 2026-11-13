//! The first start window (decision 0029, section 18): the device, the four
//! starter profiles, and the button "Start".
//!
//! The window shows before the main window when the profiles directory has
//! no profile file at the start (`Shell::set_first_start`). It is a separate
//! window of 800 × 560, which the user cannot change in size.
//!
//! - **The left column.** A small picture of the device with the key faces
//!   and the bar panels of the selected starter profile, and the state of
//!   the device: "*model name* found", or "Connect your device".
//! - **The right column.** The title, the four starter profile cards, a
//!   box about the Accessibility permission, and "Start".
//!
//! The pictures of the key faces and the bar panels are the images of the
//! engine (`opencraft_engine::page_preview`, rule 11): the window draws no
//! copy of a face. A surface with no content shows as an empty tile.
//!
//! "Start", Return, and a double-click on a card send the selected starter
//! profile to the shell (`Shell::start_with`). The shell sends it to the
//! engine, which writes it (rule 9), and opens the main window. The window
//! does not ask for the Accessibility permission. The main window asks
//! when the first action that needs it gets on a control
//! (`view::permission_sheet`).

use crate::{
    app_menu::{CloseWindow, FIRST_START},
    images,
    settings_window::{names_text, permission, Permission},
    shell::Shell,
    starters::{Starter, STARTERS},
    state::ViewState,
    view::{
        canvas::knob_ring,
        controls::{self, Accessible},
        theme,
    },
};
use gpui_kit::{
    assets::IconName, component::Icon, div, img, point, prelude::*, px, relative, size, AnyElement,
    App, Bounds, Context, Div, Entity, FocusHandle, FontWeight, KeyDownEvent, ObjectFit, Render,
    RenderImage, Role, SharedString, Stateful, Subscription, TitlebarOptions, Window, WindowBounds,
    WindowOptions,
};
use opencraft_engine::{
    page_preview, ControlKind, ControlLayout, DeviceLayout, DeviceSettings, SurfaceId,
};
use std::{collections::BTreeMap, sync::Arc};

#[cfg(test)]
mod tests;

/// The text below the title.
const INTRO: &str = "Choose a profile to start with. You can change each key later.";

/// The text below the device name with a device.
pub const CONNECTED_TEXT: &str = "Connected by USB";
/// The texts with no device.
pub const NO_DEVICE_TITLE: &str = "Connect your device";
pub const NO_DEVICE_TEXT: &str =
    "OpenCraft finds it when you connect it. You can also continue now.";

/// The options of the window: 800 × 560 in the middle of the main screen,
/// with a transparent title bar and the window buttons only. The user cannot
/// change the size, thus the zoom button is dim, as the minimize button.
pub fn window_options(cx: &App) -> WindowOptions {
    let (width, height) = theme::FIRST_START_SIZE;
    let window_size = size(px(width), px(height));
    let bounds = match cx.primary_display() {
        Some(display) => Bounds::centered_at(display.visible_bounds().center(), window_size),
        None => Bounds::centered(None, window_size, cx),
    };
    let (x, y) = theme::FIRST_START_WINDOW_BUTTONS;
    WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some("Welcome to OpenCraft".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(x, y)),
        }),
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    }
}

/// What the window shows of the shell. The window keeps a copy: the shell
/// is being updated while it opens the window (see `settings_window`).
#[derive(Clone, PartialEq)]
pub struct Snapshot {
    pub connected: bool,
    pub layout: Option<Arc<DeviceLayout>>,
    /// The knob lights of the picture, as on the canvas.
    pub device_settings: Option<DeviceSettings>,
    pub permission: Permission,
}

impl Snapshot {
    pub fn of(state: &ViewState) -> Self {
        Self {
            connected: state.connected,
            layout: state.layout.clone(),
            device_settings: state.device_settings.clone(),
            permission: permission(state),
        }
    }
}

/// The text of the box about the permission: "Three actions need the
/// Accessibility permission: … OpenCraft asks for it when you add the first
/// one." The kind names come from their descriptions (section 20). `None`
/// if the permission is given, or if no kind needs it.
pub fn permission_note(permission: &Permission) -> Option<String> {
    if permission.given || permission.kinds.is_empty() {
        return None;
    }
    let count = permission.kinds.len();
    let number = match count {
        1 => "One action needs".to_string(),
        2 => "Two actions need".to_string(),
        3 => "Three actions need".to_string(),
        4 => "Four actions need".to_string(),
        n => format!("{n} actions need"),
    };
    Some(format!(
        "{number} the Accessibility permission: {}. OpenCraft asks for it when you add the \
         first one.",
        names_text(&permission.kinds)
    ))
}

/// The texts that change with the state of the device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texts {
    /// The title of the right column.
    pub title: &'static str,
    /// The two lines below the device picture.
    pub device: String,
    pub device_text: &'static str,
}

impl Texts {
    pub fn of(snapshot: &Snapshot) -> Self {
        if snapshot.connected {
            let name = snapshot
                .layout
                .as_ref()
                .map_or_else(String::new, |l| l.name.clone());
            Self {
                title: "Your device is ready",
                device: format!("{name} found"),
                device_text: CONNECTED_TEXT,
            }
        } else {
            Self {
                title: "Welcome to OpenCraft",
                device: NO_DEVICE_TITLE.into(),
                device_text: NO_DEVICE_TEXT,
            }
        }
    }
}

/// The key faces and the bar panels of the first page of each starter
/// profile, as images of the engine, for one layout.
struct Previews {
    layout: Arc<DeviceLayout>,
    /// One map for each starter profile, in the order of [`STARTERS`]. A
    /// surface with no content is not in the map.
    images: Vec<BTreeMap<SurfaceId, Arc<RenderImage>>>,
}

impl Previews {
    fn new(layout: Arc<DeviceLayout>) -> Self {
        let images = STARTERS
            .iter()
            .map(|starter| {
                let Ok(profile) = starter.profile() else {
                    return BTreeMap::new();
                };
                let Some(page) = profile.page(profile.first_page()) else {
                    return BTreeMap::new();
                };
                page_preview(page, &layout)
                    .into_iter()
                    .filter(|(id, _)| !page.content(*id).is_empty())
                    .map(|(id, image)| (id, images::from_rgb(&image)))
                    .collect()
            })
            .collect();
        Self { layout, images }
    }

    fn drop_images(self, window: &mut Window) {
        for map in self.images {
            for (_, image) in map {
                let _ = window.drop_image(image);
            }
        }
    }
}

pub struct FirstStartView {
    shell: Entity<Shell>,
    pub(crate) snapshot: Snapshot,
    _shell_subscription: Subscription,
    focus: FocusHandle,
    /// The number of the selected card in [`STARTERS`].
    pub(crate) selected: usize,
    /// The focus of each card. The cards are one group: Tab reaches the
    /// selected card only, and the arrow keys move the selection and the
    /// focus, as in a group of radio buttons of macOS.
    cards: Vec<FocusHandle>,
    previews: Option<Previews>,
    /// Gives the focus back to the root element when the element with the
    /// focus is not drawn any more.
    _focus_lost: Subscription,
}

impl FirstStartView {
    pub fn new(
        shell: &Entity<Shell>,
        snapshot: Snapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscription = cx.observe_in(shell, window, |view, shell, window, cx| {
            // The shell notifies for each engine event, also for the input of
            // the device. Draw again only for a change that the window shows.
            let snapshot = Snapshot::of(&shell.read(cx).state);
            if snapshot != view.snapshot {
                view.snapshot = snapshot;
                view.sync_previews(window);
                cx.notify();
            }
        });
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let focus_lost = cx.on_focus_lost(window, |view, window, cx| window.focus(&view.focus, cx));
        let mut view = Self {
            shell: shell.clone(),
            snapshot,
            _shell_subscription: subscription,
            focus,
            selected: 0,
            cards: STARTERS.iter().map(|_| cx.focus_handle()).collect(),
            previews: None,
            _focus_lost: focus_lost,
        };
        view.sync_previews(window);
        view
    }

    /// Make the images of the starter profiles for the layout of the
    /// snapshot, if the layout is new.
    fn sync_previews(&mut self, window: &mut Window) {
        let Some(layout) = self.snapshot.layout.clone() else {
            return;
        };
        if self
            .previews
            .as_ref()
            .is_some_and(|p| Arc::ptr_eq(&p.layout, &layout) || *p.layout == *layout)
        {
            return;
        }
        if let Some(old) = self.previews.take() {
            old.drop_images(window);
        }
        self.previews = Some(Previews::new(layout));
    }

    /// The images of the starter profile `index`.
    fn images(&self, index: usize) -> Option<&BTreeMap<SurfaceId, Arc<RenderImage>>> {
        self.previews.as_ref()?.images.get(index)
    }

    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < STARTERS.len() && index != self.selected {
            self.selected = index;
            cx.notify();
        }
    }

    /// Start with the selected starter profile, and close this window. The
    /// shell opens the main window.
    fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.selected;
        self.shell
            .update(cx, |shell, cx| shell.start_with(selected, cx));
        window.remove_window();
    }

    /// Return starts. The arrow keys move the selection in the grid of
    /// cards.
    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if modifiers.platform || modifiers.control || modifiers.alt {
            return;
        }
        let columns = 2;
        let selected = self.selected;
        let next = match event.keystroke.key.as_str() {
            "enter" => {
                cx.stop_propagation();
                return self.start(window, cx);
            }
            "left" if !selected.is_multiple_of(columns) => selected - 1,
            "right" if selected % columns + 1 < columns => selected + 1,
            "up" if selected >= columns => selected - columns,
            "down" => selected + columns,
            _ => return,
        };
        cx.stop_propagation();
        // The focus goes with the selection, if a card has the focus.
        let card_focused = self.cards[selected].is_focused(window);
        self.select(next, cx);
        if card_focused {
            window.focus(&self.cards[self.selected], cx);
        }
    }

    /// The left column: the device picture and the state of the device.
    fn left_column(&self) -> Stateful<Div> {
        let texts = Texts::of(&self.snapshot);
        let (title, text) = (texts.device, texts.device_text);
        div()
            .id("first-start-left")
            .debug_selector(|| "first-start-left".into())
            .w(theme::FIRST_START_LEFT)
            .flex_shrink_0()
            .h_full()
            .border_r_1()
            .border_color(theme::LINE_SECTION)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(theme::SPACE_18)
            .child(self.picture())
            .child(
                div()
                    .max_w(theme::FIRST_START_LEFT_TEXT)
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(theme::SPACE_4)
                    .text_center()
                    .child(
                        div()
                            .id("first-start-device")
                            .debug_selector(|| "first-start-device".into())
                            .readable_text(title.clone())
                            .text_size(theme::TEXT_DEVICE_STATE)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(
                        div()
                            .id("first-start-device-text")
                            .debug_selector(|| "first-start-device-text".into())
                            .readable_text(text)
                            .text_size(theme::TEXT_LABEL)
                            .text_color(theme::TEXT_SECONDARY)
                            .line_height(relative(theme::LINE_HEIGHT_TEXT))
                            .child(text),
                    ),
            )
    }

    /// The small device picture: the key faces, the bar panels, and the
    /// knobs of the layout, with the images of the selected starter profile.
    fn picture(&self) -> Stateful<Div> {
        let (width, height) = theme::PICTURE_SIZE;
        let connected = self.snapshot.connected;
        let images = self.images(self.selected);
        let controls = |kind: ControlKind| -> Vec<ControlLayout> {
            let mut list: Vec<ControlLayout> = self
                .snapshot
                .layout
                .as_ref()
                .map(|l| {
                    l.controls
                        .iter()
                        .filter(|c| c.id.kind == kind)
                        .copied()
                        .collect()
                })
                .unwrap_or_default();
            list.sort_by_key(|c| (c.row, c.column));
            list
        };
        let keys = controls(ControlKind::Key);
        let panels = controls(ControlKind::BarPanel);
        let knobs = controls(ControlKind::Knob);
        let tile = |surface: Option<SurfaceId>, panel: bool| {
            let image = surface.and_then(|id| images.and_then(|m| m.get(&id)).cloned());
            let radius = if panel {
                theme::RADIUS_PICTURE_PANEL
            } else {
                theme::RADIUS_PICTURE_KEY
            };
            div()
                .when(panel, |d| d.flex_1().h(theme::PICTURE_PANEL_HEIGHT))
                .when(!panel, |d| d.size(theme::PICTURE_KEY))
                .flex_shrink_0()
                .rounded(radius)
                .overflow_hidden()
                .bg(if panel {
                    theme::PICTURE_PANEL_EMPTY
                } else {
                    theme::BG_TILE
                })
                .children(image.map(|image| {
                    img(image)
                        .size_full()
                        .object_fit(ObjectFit::Cover)
                        .rounded(radius)
                }))
        };
        let mut rows: BTreeMap<u8, Vec<ControlLayout>> = BTreeMap::new();
        for key in keys {
            rows.entry(key.row).or_default().push(key);
        }
        let settings = self.snapshot.device_settings.as_ref();
        div()
            .id("first-start-picture")
            .debug_selector(|| "first-start-picture".into())
            .relative()
            .w(width)
            .h(height)
            .flex_shrink_0()
            .child(
                div()
                    .size_full()
                    .p(theme::PICTURE_PADDING)
                    .rounded(theme::RADIUS_PICTURE)
                    .bg(theme::DEVICE_BODY)
                    .border_1()
                    .border_color(theme::DEVICE_BODY_BORDER)
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(theme::SPACE_8)
                    .when(!connected, |d| d.opacity(theme::PICTURE_NO_DEVICE_OPACITY))
                    .child(div().flex().flex_col().gap(theme::SPACE_6).children(
                        rows.into_values().map(|row| {
                            div()
                                .flex()
                                .justify_between()
                                .children(row.into_iter().map(|key| tile(key.surface, false)))
                        }),
                    ))
                    .when(!panels.is_empty(), |d| {
                        d.child(
                            div()
                                .flex()
                                .gap(theme::SPACE_6)
                                .children(panels.iter().map(|panel| tile(panel.surface, true))),
                        )
                    })
                    .when(!knobs.is_empty(), |d| {
                        d.child(div().flex().children(knobs.iter().map(|knob| {
                            div().flex_1().flex().justify_center().child(
                                div()
                                    .size(theme::PICTURE_KNOB)
                                    .rounded_full()
                                    .border(theme::PICTURE_KNOB_RING)
                                    .border_color(knob_ring(settings, knob.id))
                                    .bg(theme::DEVICE_KNOB),
                            )
                        })))
                    }),
            )
            .when(connected, |d| {
                d.child(
                    div()
                        .id("first-start-badge")
                        .debug_selector(|| "first-start-badge".into())
                        .absolute()
                        .top(theme::BADGE_CHECK_OUTSIDE)
                        .right(theme::BADGE_CHECK_OUTSIDE)
                        .size(theme::BADGE_CHECK)
                        .rounded_full()
                        .bg(theme::SUCCESS_FILL)
                        .border(theme::BADGE_CHECK_BORDER)
                        .border_color(theme::BG_BAR)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            Icon::new(IconName::Check)
                                .size(theme::ICON_BADGE_CHECK)
                                .text_color(theme::TEXT_ON_ACCENT),
                        ),
                )
            })
    }

    /// The right column: the title, the cards, the box about the
    /// permission, and "Start".
    fn right_column(&self, cx: &mut Context<Self>) -> Div {
        let title = Texts::of(&self.snapshot).title;
        let cards: Vec<AnyElement> = STARTERS
            .iter()
            .enumerate()
            .map(|(index, starter)| self.card(index, starter, cx).into_any_element())
            .collect();
        let mut rows = Vec::new();
        let mut cards = cards.into_iter();
        while let Some(first) = cards.next() {
            rows.push(
                div()
                    .flex()
                    .gap(theme::SPACE_12)
                    .child(div().flex_1().min_w_0().flex().child(first))
                    .child(div().flex_1().min_w_0().flex().children(cards.next())),
            );
        }
        let start = controls::main_button("first-start-start", "Start", theme::SPACE_16)
            .on_click(cx.listener(|view, _, window, cx| view.start(window, cx)));
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .pt(theme::FIRST_START_RIGHT_TOP)
            .px(theme::FIRST_START_RIGHT_SIDE)
            .pb(theme::FIRST_START_RIGHT_BOTTOM)
            .child(
                div()
                    .id("first-start-title")
                    .debug_selector(|| "first-start-title".into())
                    .readable_text(title)
                    .text_size(theme::TEXT_DISPLAY)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(
                div()
                    .id("first-start-intro")
                    .readable_text(INTRO)
                    .mt(theme::SPACE_6)
                    .text_color(theme::TEXT_SECONDARY)
                    .line_height(relative(theme::FIRST_START_LINE_HEIGHT))
                    .child(INTRO),
            )
            .child(
                div()
                    .id("first-start-cards")
                    .role(Role::RadioGroup)
                    .mt(theme::SPACE_20)
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_12)
                    .children(rows),
            )
            .child(div().flex_1())
            .children(permission_note(&self.snapshot.permission).map(|text| {
                div()
                    .id("first-start-permission")
                    .debug_selector(|| "first-start-permission".into())
                    .mb(theme::SPACE_16)
                    .py(theme::SPACE_10)
                    .px(theme::SPACE_12)
                    .rounded(theme::RADIUS_NOTE_BOX)
                    .bg(theme::BG_CARD)
                    .border_1()
                    .border_color(theme::LINE_PANE)
                    .flex()
                    .items_start()
                    .gap(theme::SPACE_10)
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::TEXT_SECONDARY)
                    .line_height(relative(theme::LINE_HEIGHT_TEXT))
                    .child(
                        div().flex_shrink_0().pt(theme::SPACE_1).child(
                            Icon::new(IconName::Shield)
                                .size(theme::ICON_NOTE)
                                .text_color(theme::TEXT_SECONDARY),
                        ),
                    )
                    .child(
                        div()
                            .id("first-start-permission-text")
                            .readable_text(text.clone())
                            .flex_1()
                            .min_w_0()
                            .child(text),
                    )
            }))
            .child(div().flex().justify_end().child(start))
    }

    /// The card of the starter profile `index`.
    fn card(&self, index: usize, starter: &Starter, cx: &mut Context<Self>) -> Stateful<Div> {
        let selected = index == self.selected;
        let id: SharedString = format!("starter-{index}").into();
        let selector = id.clone();
        let images = self.images(index);
        let keys: Vec<Option<SurfaceId>> = self
            .snapshot
            .layout
            .as_ref()
            .map(|layout| {
                let mut keys: Vec<&ControlLayout> = layout
                    .controls
                    .iter()
                    .filter(|c| c.id.kind == ControlKind::Key)
                    .collect();
                keys.sort_by_key(|c| (c.row, c.column));
                keys.into_iter().map(|c| c.surface).collect()
            })
            .unwrap_or_default();
        let columns = self
            .snapshot
            .layout
            .as_ref()
            .and_then(|l| {
                l.controls
                    .iter()
                    .filter(|c| c.id.kind == ControlKind::Key)
                    .map(|c| usize::from(c.column))
                    .max()
            })
            .unwrap_or(1)
            .max(1);
        let cell = |surface: &Option<SurfaceId>| {
            let image = surface.and_then(|id| images.and_then(|m| m.get(&id)).cloned());
            div()
                .flex_1()
                .min_w_0()
                .h(theme::STARTER_CELL)
                .rounded(theme::RADIUS_STARTER_CELL)
                .overflow_hidden()
                .bg(theme::BG_TILE)
                .children(image.map(|image| {
                    img(image)
                        .size_full()
                        .object_fit(ObjectFit::Cover)
                        .rounded(theme::RADIUS_STARTER_CELL)
                }))
        };
        // Tab reaches the selected card only (see `cards`).
        let focus = self.cards[index].clone().tab_stop(selected);
        controls::keyboard_focus_tracked(div().id(id.clone()), &focus, controls::Ring::Inside)
            .debug_selector(move || selector.to_string())
            .accessible(Role::RadioButton, starter.name)
            .aria_description(starter.description)
            .checked(selected)
            .flex_1()
            .min_w_0()
            .p(theme::SPACE_12)
            .rounded(theme::RADIUS_STARTER_CARD)
            .border_1()
            .flex()
            .flex_col()
            .gap(theme::SPACE_6)
            .cursor_pointer()
            .when(selected, |d| {
                d.bg(theme::ACCENT_TINT_START)
                    .border_color(theme::ACCENT_LINE)
            })
            .when(!selected, |d| {
                d.bg(theme::BG_CARD)
                    .border_color(theme::BORDER_FIELD)
                    .hover(|d| d.border_color(theme::BORDER_CONTROL).bg(theme::BG_FIELD))
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_3)
                    .children(keys.chunks(columns).map(|row| {
                        div()
                            .flex()
                            .gap(theme::SPACE_3)
                            .children(row.iter().map(cell))
                    })),
            )
            .child(div().font_weight(FontWeight::SEMIBOLD).child(starter.name))
            .child(
                div()
                    .text_size(theme::TEXT_HELP)
                    .text_color(theme::TEXT_SECONDARY)
                    .line_height(relative(theme::LINE_HEIGHT_TEXT))
                    .child(starter.description),
            )
            .on_click(
                cx.listener(move |view, event: &gpui_kit::ClickEvent, window, cx| {
                    view.select(index, cx);
                    if event.click_count() >= 2 {
                        view.start(window, cx);
                    }
                }),
            )
    }
}

impl Render for FirstStartView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("first-start")
            .key_context(FIRST_START)
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &CloseWindow, window, _| window.remove_window()))
            .on_action(cx.listener(|view, _: &controls::FocusOwner, window, cx| {
                window.focus(&view.focus, cx)
            }))
            .on_key_down(cx.listener(|view, event, window, cx| view.key_down(event, window, cx)))
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::BG_BAR)
            .text_color(theme::TEXT_PRIMARY)
            .text_size(theme::TEXT_BODY)
            .cursor_default()
            // The title bar has the window buttons only.
            .child(div().h(theme::FIRST_START_TITLE_HEIGHT).flex_shrink_0())
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.left_column())
                    .child(self.right_column(cx)),
            )
    }
}
