//! The command palette (decision 0029, section 15): one search field for
//! the full window.
//!
//! ⌘K, "Search…" in the Edit menu, and the search field of the toolbar open
//! the palette. Its frame is the shared modal layer of `modal`: the dim
//! layer, the focus, Escape, and Return. The palette closes on a click out
//! of it.
//!
//! The palette has no list of its own. Each group of results comes from
//! data that the window has already:
//!
//! | Group | Source |
//! |---|---|
//! | "Actions" | The kinds of `Event::Kinds`, in the order of the library (`library::groups`) |
//! | "Controls" | The controls of the layout, with the labels of the active page |
//! | "Pages" | The pages of each profile of `Event::Profiles`, the active profile first |
//! | "Profiles" | The profiles of `Event::Profiles` |
//! | "Commands" | The table of the menus and of the pane "Shortcuts" (`app_menu::commands`) |
//!
//! Thus a new action kind or a new command shows in the palette with no
//! change here. A command shows only if the window can do it now: the same
//! rule that enables a menu item.
//!
//! The keys: Up and Down (also ⌃P and ⌃N) move the selected row across the
//! groups, and the list scrolls so that the row is visible. Return does the
//! main operation of the row, ⌘↵ does it and shows the result in the
//! Inspector tab, and Escape closes the palette. An action kind goes to the
//! selected control through the same path as a click in the library and a
//! drop (`DeviceView::assign_kind`).

use super::{
    controls,
    gesture_card::{KindLabel, TileSize},
    library,
    modal::{Modal, ModalKey},
    pages::page_title,
    *,
};
use crate::{
    app_menu::{
        self, AddPage, AddProfile, DeviceSettings, EditMenu, ImportProfile, OpenInInspector,
        PageSettings, Search, SelectNext, SelectPrevious, PALETTE,
    },
    state::gestures,
};
use gpui_kit::{assets::IconName, component::Icon, Action, FontWeight, Role};

/// The most rows of one group (section 15).
pub(super) const GROUP_LIMIT: usize = 5;

/// The accessible label of the palette and of the search field of the
/// toolbar.
pub(super) const LABEL: &str = "Search actions, controls, pages and commands";

/// A group of results, in the order of the palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Group {
    Actions,
    Controls,
    Pages,
    Profiles,
    Commands,
}

impl Group {
    pub(super) fn title(self) -> &'static str {
        match self {
            Group::Actions => "Actions",
            Group::Controls => "Controls",
            Group::Pages => "Pages",
            Group::Profiles => "Profiles",
            Group::Commands => "Commands",
        }
    }
}

/// The thing that a row of the palette operates on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Target {
    /// An action kind: its identifier.
    Kind(String),
    Control(ControlId),
    /// A page of a profile: the identity of the profile, and the page from 0.
    Page {
        profile: String,
        page: usize,
    },
    /// A profile: its identity.
    Profile(String),
    /// A command: its position in [`Palette::commands`].
    Command(usize),
}

/// The tile at the start of a row.
#[derive(Clone, Debug)]
enum Tile {
    /// The kind tile of the library.
    Kind(KindLabel),
    /// An icon on `bg-tile`.
    Icon(IconName),
}

/// One row of the palette.
#[derive(Clone, Debug)]
pub(super) struct Item {
    pub(super) group: Group,
    pub(super) name: SharedString,
    pub(super) hint: Option<SharedString>,
    tile: Tile,
    pub(super) target: Target,
}

/// A command of the palette, from the table of `app_menu`, with the keys
/// of its key binding.
struct PaletteCommand {
    title: SharedString,
    icon: IconName,
    action: Box<dyn Action>,
    keys: Option<SharedString>,
}

/// The state of the palette.
pub(super) struct Palette {
    /// True while the palette is open.
    pub(super) open: bool,
    /// The focus of the card of the palette. The search field has the
    /// focus while the palette is open.
    focus: FocusHandle,
    /// The search field.
    pub(super) query: Entity<InputState>,
    /// The position of the selected row in the list of all rows.
    pub(super) selected: usize,
    scroll: ScrollHandle,
    /// The commands that the window can do, from the moment that the
    /// palette opened.
    commands: Vec<PaletteCommand>,
    _subscription: Subscription,
}

impl Palette {
    pub(super) fn new(window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));
        // A new text selects the first row.
        let subscription = cx.subscribe_in(&query, window, |view, _, event, _, cx| {
            if matches!(event, TextEvent::Change) {
                view.palette.selected = 0;
                view.palette.scroll.scroll_to_item(0);
                cx.notify();
            }
        });
        Self {
            open: false,
            focus: cx.focus_handle(),
            query,
            selected: 0,
            scroll: ScrollHandle::new(),
            commands: Vec::new(),
            _subscription: subscription,
        }
    }
}

/// True if `name` contains `text`, with no difference between capital and
/// small letters. An empty text matches each name.
fn contains(name: &str, text: &str) -> bool {
    name.to_lowercase().contains(&text.trim().to_lowercase())
}

/// The text with no result: "No result for “*text*”."
pub(super) fn no_result(text: &str) -> String {
    format!("No result for “{}”.", text.trim())
}

/// The hint of a page: "*profile* · page *N*".
fn page_hint(profile: &str, page: usize) -> String {
    format!("{profile} · page {}", page + 1)
}

/// The icon of a control in the palette.
fn control_icon(kind: ControlKind) -> IconName {
    match kind {
        ControlKind::Key => IconName::Square,
        ControlKind::Knob => IconName::CircleDot,
        ControlKind::BarPanel | ControlKind::TouchBar => IconName::RectangleHorizontal,
    }
}

impl DeviceView {
    /// The handlers of the commands of the table of `app_menu` that only
    /// the palette shows, and of "Search…".
    pub(super) fn window_commands(
        &self,
        root: Stateful<Div>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        root.on_action(cx.listener(|view, _: &Search, window, cx| view.open_palette(window, cx)))
            .on_action(cx.listener(|view, _: &AddPage, _, _| {
                (view.send)(Command::AddPage { copy_of: None })
            }))
            .on_action(cx.listener(|view, _: &PageSettings, window, cx| {
                view.select_page(window, cx);
                view.show_inspector(window, cx);
            }))
            .on_action(cx.listener(|view, _: &DeviceSettings, window, cx| {
                view.select_device(window, cx);
                view.show_inspector(window, cx);
            }))
            .on_action(cx.listener(|view, _: &AddProfile, window, cx| {
                // The new row is in the sidebar.
                if !view.panes.sidebar_visible {
                    view.toggle_sidebar(cx);
                }
                view.start_new_profile(window, cx);
            }))
            .on_action(cx.listener(|view, _: &ImportProfile, window, cx| {
                view.import_profile(window, cx);
            }))
    }

    /// Open the palette with an empty field. The palette keeps the commands
    /// that the window can do now: the focus is still in the window.
    pub(super) fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.open {
            return;
        }
        self.end_text_edit(cx);
        // A menu of the inspector paints above the modal layer.
        self.close_kinds();
        let edit = EditMenu {
            undo: self.state.undo.clone(),
            redo: self.state.redo.clone(),
        };
        let mut commands: Vec<_> = app_menu::commands(&self.panes, &edit)
            .into_iter()
            .filter_map(|command| Some((command.palette?, command)))
            .filter(|(_, command)| {
                let action = command.action.as_ref();
                window.is_action_available_in(action, &self.focus) || cx.is_action_available(action)
            })
            .collect();
        commands.sort_by_key(|(place, _)| *place);
        self.palette.commands = commands
            .into_iter()
            .map(|(_, command)| PaletteCommand {
                keys: crate::settings_window::shortcuts::keys(command.action.as_ref(), cx)
                    .map(Into::into),
                title: command.title.into(),
                icon: command.icon,
                action: command.action,
            })
            .collect();
        self.palette.open = true;
        self.palette
            .query
            .update(cx, |query, cx| query.set_value("", window, cx));
        self.palette.selected = 0;
        self.palette.scroll.scroll_to_item(0);
        let query = self.palette.query.clone();
        query.update(cx, |query, cx| query.focus(window, cx));
        cx.notify();
    }

    /// Close the palette, and give the focus back to the window.
    pub(super) fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.palette.open {
            return;
        }
        self.palette.open = false;
        self.palette.commands.clear();
        self.focus_workspace(window, cx);
        cx.notify();
    }

    /// The rows of the palette for the text in its field, in the order of
    /// the groups. Each group has [`GROUP_LIMIT`] rows at the most.
    pub(super) fn palette_items(&self, cx: &App) -> Vec<Item> {
        let text = self.palette.query.read(cx).value().to_string();
        let mut items = Vec::new();
        let mut add = |group: Group, rows: Vec<Item>| {
            items.extend(
                rows.into_iter()
                    .filter(|row| row.group == group)
                    .take(GROUP_LIMIT),
            );
        };

        // The kinds of the library, in its order.
        let kinds = library::groups(&self.state.kinds, |kind| contains(&kind.name, &text))
            .into_iter()
            .flat_map(|(_, members)| members)
            .map(|index| {
                let kind = &self.state.kinds[index];
                Item {
                    group: Group::Actions,
                    name: kind.name.clone().into(),
                    hint: Some("Action".into()),
                    tile: Tile::Kind(KindLabel::new(kind)),
                    target: Target::Kind(kind.kind.clone()),
                }
            })
            .collect();
        add(Group::Actions, kinds);

        // The controls of the layout, by name and by label. The touch bar is
        // a part of the page.
        let controls = self
            .state
            .layout
            .iter()
            .flat_map(|layout| layout.controls.iter())
            .filter(|control| control.id.kind != ControlKind::TouchBar)
            .filter_map(|control| {
                let name = control_name(control.id);
                let label = control
                    .surface
                    .or(control.feedback_surface)
                    .and_then(|surface| self.state.surfaces.get(&surface))
                    .and_then(|surface| surface.content.label.clone())
                    .filter(|label| !label.trim().is_empty());
                let matches = contains(&name, &text)
                    || label.as_deref().is_some_and(|label| contains(label, &text));
                matches.then(|| Item {
                    group: Group::Controls,
                    hint: Some(
                        label
                            .unwrap_or_else(|| self.control_action(control.id))
                            .into(),
                    ),
                    name: name.into(),
                    tile: Tile::Icon(control_icon(control.id.kind)),
                    target: Target::Control(control.id),
                })
            })
            .collect();
        add(Group::Controls, controls);

        // The pages of each profile. The active profile has the current
        // names of its pages.
        let active = self.state.profile.as_deref();
        let profiles = self
            .state
            .profiles
            .iter()
            .filter(|p| Some(p.id.as_str()) == active)
            .chain(
                self.state
                    .profiles
                    .iter()
                    .filter(|p| Some(p.id.as_str()) != active),
            );
        let pages = profiles
            .flat_map(|profile| {
                let names = if Some(profile.id.as_str()) == active {
                    &self.state.pages
                } else {
                    &profile.pages
                };
                (0..names.len()).map(move |page| (profile, page_title(names, page), page))
            })
            .filter(|(_, name, _)| contains(name, &text))
            .map(|(profile, name, page)| Item {
                group: Group::Pages,
                name: name.into(),
                hint: Some(page_hint(&profile.name, page).into()),
                tile: Tile::Icon(IconName::PanelTop),
                target: Target::Page {
                    profile: profile.id.clone(),
                    page,
                },
            })
            .collect();
        add(Group::Pages, pages);

        let profiles = self
            .state
            .profiles
            .iter()
            .filter(|profile| contains(&profile.name, &text))
            .map(|profile| Item {
                group: Group::Profiles,
                name: profile.name.clone().into(),
                hint: (Some(profile.id.as_str()) == active).then(|| "On device".into()),
                tile: Tile::Icon(IconName::Layers),
                target: Target::Profile(profile.id.clone()),
            })
            .collect();
        add(Group::Profiles, profiles);

        let commands = self
            .palette
            .commands
            .iter()
            .enumerate()
            .filter(|(_, command)| contains(&command.title, &text))
            .map(|(index, command)| Item {
                group: Group::Commands,
                name: command.title.clone(),
                hint: command.keys.clone(),
                tile: Tile::Icon(command.icon),
                target: Target::Command(index),
            })
            .collect();
        add(Group::Commands, commands);
        items
    }

    /// The hint of a control with no label: the name of the action of its
    /// first gesture with an action, or "Empty".
    fn control_action(&self, control: ControlId) -> String {
        gestures(control.kind)
            .iter()
            .find_map(|(gesture, _)| {
                let binding = self
                    .state
                    .bindings
                    .values()
                    .find(|b| b.binding.control == control && b.binding.gesture == *gesture)?;
                let action = binding.action.as_ref()?;
                Some(
                    self.kind(&action.kind)
                        .map_or_else(|| "Not available".to_string(), |k| k.name.clone()),
                )
            })
            .unwrap_or_else(|| "Empty".into())
    }

    /// The control that an action from the palette goes to: the selected
    /// control, as for a click in the library. The page as the selection
    /// has no selected control (section 11).
    fn palette_control(&self) -> Option<ControlId> {
        self.framed_control().filter(|_| !self.state.page_selected)
    }

    /// Move the selected row by `step` rows, and scroll the list so that
    /// the row is visible. The first row of a group shows its header also.
    fn move_palette_selection(&mut self, step: isize, cx: &mut Context<Self>) {
        let items = self.palette_items(cx);
        if items.is_empty() {
            return;
        }
        let selected = self
            .palette
            .selected
            .saturating_add_signed(step)
            .min(items.len() - 1);
        self.palette.selected = selected;
        // The children of the list: a header before the first row of each
        // group, then the rows.
        let groups_before = items[..=selected]
            .iter()
            .enumerate()
            .filter(|(n, item)| *n == 0 || items[n - 1].group != item.group)
            .count();
        let first_of_group = selected == 0 || items[selected - 1].group != items[selected].group;
        let child = selected + groups_before - usize::from(first_of_group);
        self.palette.scroll.scroll_to_item(child);
        cx.notify();
    }

    /// Do the main operation of the selected row, and close the palette.
    /// With `inspector`, also select the result and show the Inspector tab.
    fn run_palette(&mut self, inspector: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self
            .palette_items(cx)
            .into_iter()
            .nth(self.palette.selected)
        else {
            return;
        };
        let command = match &item.target {
            Target::Command(index) => self
                .palette
                .commands
                .get(*index)
                .map(|c| c.action.boxed_clone()),
            _ => None,
        };
        self.close_palette(window, cx);
        match item.target {
            Target::Kind(kind) => {
                self.assign_kind(&kind, window, cx);
                if inspector && self.palette_control().is_some() {
                    self.show_inspector(window, cx);
                }
            }
            Target::Control(control) => {
                self.select_control(control, window, cx);
                if inspector {
                    self.show_inspector(window, cx);
                }
            }
            Target::Page { profile, page } => {
                if self.state.profile.as_ref() == Some(&profile) {
                    self.activate_page(page, window, cx);
                } else {
                    // The engine does the commands in their order: the page
                    // is a page of the new active profile.
                    (self.send)(Command::SelectProfile { profile });
                    (self.send)(Command::SelectPage { page });
                }
                if inspector {
                    self.select_page(window, cx);
                    self.show_inspector(window, cx);
                }
            }
            Target::Profile(profile) => {
                if self.state.profile.as_ref() != Some(&profile) {
                    (self.send)(Command::SelectProfile { profile });
                }
            }
            Target::Command(_) => {
                // The action goes to the focus. The focus is in the window
                // again, which has the handlers (`docs/kb/gpui.md`, "Popover
                // and modal layer").
                if let Some(action) = command {
                    window.dispatch_action(action, cx);
                }
            }
        }
    }

    /// Escape and Return in the palette.
    fn palette_key(&mut self, key: ModalKey, window: &mut Window, cx: &mut Context<Self>) {
        match key {
            ModalKey::Escape => self.close_palette(window, cx),
            ModalKey::Return => self.run_palette(false, window, cx),
        }
    }

    /// The text of the main operation of `item` in the footer.
    fn main_operation(&self, item: &Item) -> String {
        match item.target {
            Target::Kind(_) => match self.palette_control() {
                Some(control) => format!("Assign to {}", control_name(control)),
                None => "Assign".into(),
            },
            Target::Control(_) | Target::Page { .. } | Target::Profile(_) => "Select".into(),
            Target::Command(_) => "Do the command".into(),
        }
    }

    /// The palette, when it is open.
    pub(super) fn command_palette(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.palette.open {
            return None;
        }
        let view = cx.entity();
        let close = view.clone();
        let modal = Modal::new("palette", &self.palette.focus, LABEL)
            .width(theme::PALETTE_WIDTH)
            .top(theme::PALETTE_TOP)
            .background(theme::BG_FIELD)
            .on_key(move |key, window, cx| {
                view.update(cx, |view, cx| view.palette_key(key, window, cx))
            })
            .close_on_click_out(move |window, cx| {
                close.update(cx, |view, cx| view.close_palette(window, cx))
            });
        let items = self.palette_items(cx);
        let selected = self.palette.selected.min(items.len().saturating_sub(1));
        let content = div()
            .id("palette-content")
            .key_context(PALETTE)
            .on_action(
                cx.listener(|view, _: &SelectPrevious, _, cx| view.move_palette_selection(-1, cx)),
            )
            .on_action(
                cx.listener(|view, _: &SelectNext, _, cx| view.move_palette_selection(1, cx)),
            )
            .on_action(cx.listener(|view, _: &OpenInInspector, window, cx| {
                view.run_palette(true, window, cx)
            }))
            // The card has the border: the content fills the rest.
            .h(theme::PALETTE_HEIGHT - theme::BORDER_OVERLAY_WIDTH * 2.)
            .flex()
            .flex_col()
            .child(self.palette_search_row())
            .child(self.palette_results(&items, selected, cx))
            .child(self.palette_footer(items.get(selected)));
        Some(modal.render(content, window))
    }

    /// The search icon, the field, and the chip of the selected control.
    fn palette_search_row(&self) -> Div {
        div()
            .flex_shrink_0()
            .h(theme::HEIGHT_PALETTE_SEARCH)
            .px(theme::SPACE_18)
            .flex()
            .items_center()
            .gap(theme::SPACE_12)
            .border_b_1()
            .border_color(theme::BORDER_FIELD)
            .child(
                Icon::new(IconName::Search)
                    .size(theme::ICON_PALETTE_SEARCH)
                    .text_color(theme::TEXT_SECONDARY),
            )
            .child(
                div()
                    .id("palette-field")
                    .debug_selector(|| "palette-field".into())
                    .flex_1()
                    .min_w_0()
                    .child(
                        Input::new(&self.palette.query)
                            .aria_label(LABEL)
                            .appearance(false)
                            .h(theme::HEIGHT_PALETTE_FIELD)
                            .px_0()
                            .py_0()
                            .text_size(theme::TEXT_SHEET_TITLE),
                    ),
            )
            .children(self.palette_control().map(|control| {
                let text = format!("{} selected", control_name(control));
                div()
                    .id("palette-context")
                    .debug_selector(|| "palette-context".into())
                    .readable_text(text.clone())
                    .flex_shrink_0()
                    .py(theme::SPACE_3)
                    .px(theme::SPACE_8)
                    .rounded(theme::RADIUS_CONTROL)
                    .bg(theme::BG_CONTEXT_CHIP)
                    .text_size(theme::TEXT_HELP)
                    .text_color(theme::TEXT_SECONDARY)
                    .child(text)
            }))
    }

    /// The groups and their rows, or the text with no result.
    fn palette_results(&self, items: &[Item], selected: usize, cx: &mut Context<Self>) -> Div {
        let text = self.palette.query.read(cx).value().to_string();
        // The headers and the rows are the children of the scroll element,
        // so that `scroll_to_item` finds a row by its position.
        let mut children: Vec<AnyElement> = Vec::new();
        if items.is_empty() {
            children.push(
                div()
                    .id("palette-no-result")
                    .debug_selector(|| "palette-no-result".into())
                    .readable_text(no_result(&text))
                    .pt(theme::EMPTY_STATE_TOP)
                    .px(theme::SECTION_PADDING_SIDE)
                    .text_center()
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::TEXT_TERTIARY)
                    .child(no_result(&text))
                    .into_any_element(),
            );
        } else {
            for (number, item) in items.iter().enumerate() {
                if number == 0 || items[number - 1].group != item.group {
                    let group = item.group;
                    children.push(
                        controls::section_title(group.title())
                            .pt(theme::SPACE_8)
                            .px(theme::SPACE_10)
                            .pb(theme::SPACE_4)
                            .debug_selector(move || format!("palette-group-{}", group.title()))
                            .into_any_element(),
                    );
                }
                children.push(self.palette_row(number, item, number == selected, cx));
            }
        }
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .id("palette-results")
                    .debug_selector(|| "palette-results".into())
                    .size_full()
                    .track_scroll(&self.palette.scroll)
                    .overflow_y_scroll()
                    .py(theme::SPACE_6)
                    .px(theme::SPACE_8)
                    .flex()
                    .flex_col()
                    .children(children),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .child(Scrollbar::vertical(&self.palette.scroll)),
            )
    }

    /// One row: the tile, the name, and the hint at the end.
    fn palette_row(
        &self,
        number: usize,
        item: &Item,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let tile = match &item.tile {
            Tile::Kind(label) => label.tile(TileSize::Library),
            Tile::Icon(icon) => div()
                .flex_shrink_0()
                .size(theme::PALETTE_TILE)
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme::RADIUS_PALETTE_TILE)
                .bg(theme::BG_TILE)
                .text_color(theme::TEXT_ON_ACCENT)
                .child(Icon::new(*icon).size(theme::ICON_PALETTE_TILE)),
        };
        div()
            .id(SharedString::from(format!("palette-row-{number}")))
            .debug_selector(move || format!("palette-row-{number}"))
            .accessible(Role::Button, item.name.clone())
            .when_some(item.hint.clone(), |d, hint| d.aria_description(hint))
            .aria_selected(selected)
            .flex_shrink_0()
            .h(theme::HEIGHT_PALETTE_ROW)
            .px(theme::SPACE_10)
            .flex()
            .items_center()
            .gap(theme::SPACE_10)
            .rounded(theme::RADIUS_PALETTE_ROW)
            .text_size(theme::TEXT_BODY)
            .when(selected, |d| {
                d.bg(theme::ACCENT).text_color(theme::TEXT_ON_ACCENT)
            })
            .child(tile.when(selected, |tile| tile.bg(theme::TILE_ON_ACCENT)))
            .child(div().flex_1().min_w_0().truncate().child(item.name.clone()))
            .children(item.hint.clone().map(|hint| {
                div()
                    .flex_shrink_0()
                    .max_w(theme::WIDTH_TOOLBAR_SEARCH)
                    .truncate()
                    .text_size(theme::TEXT_LABEL)
                    .text_color(if selected {
                        theme::TEXT_ON_ACCENT
                    } else {
                        theme::TEXT_SECONDARY
                    })
                    .child(hint)
            }))
            // The pointer selects the row under it, as in the menus of
            // macOS. Only a move of the pointer does this: a scroll with the
            // keys does not.
            .on_mouse_move(cx.listener(move |view, _, _, cx| {
                if view.palette.selected != number {
                    view.palette.selected = number;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |view, _, window, cx| {
                view.palette.selected = number;
                view.run_palette(false, window, cx);
            }))
            .into_any_element()
    }

    /// The keys of the selected row, and "esc Close" at the end.
    fn palette_footer(&self, item: Option<&Item>) -> Div {
        let hint = |id: &'static str, key: &'static str, text: String| {
            div()
                .debug_selector(move || id.to_string())
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(theme::SPACE_4)
                .child(
                    div()
                        .text_color(theme::TEXT_PRIMARY)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(key),
                )
                .child(text)
        };
        let inspector = item.is_some_and(|item| match item.target {
            Target::Kind(_) => self.palette_control().is_some(),
            Target::Control(_) | Target::Page { .. } => true,
            Target::Profile(_) | Target::Command(_) => false,
        });
        div()
            .flex_shrink_0()
            .h(theme::HEIGHT_PALETTE_FOOTER)
            .px(theme::SPACE_18)
            .flex()
            .items_center()
            .gap(theme::SPACE_16)
            .border_t_1()
            .border_color(theme::BORDER_FIELD)
            .text_size(theme::TEXT_HELP)
            .text_color(theme::TEXT_SECONDARY)
            .children(item.map(|item| hint("palette-main", "↵", self.main_operation(item))))
            .when(inspector, |d| {
                d.child(hint(
                    "palette-inspector",
                    "⌘↵",
                    "Open in the inspector".into(),
                ))
            })
            .child(div().flex_1())
            .child(hint("palette-close", "esc", "Close".into()))
    }

    /// The search field of the toolbar (section 5): a button that looks like
    /// a search field. A click opens the palette.
    pub(super) fn toolbar_search(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("toolbar-search")
            .debug_selector(|| "toolbar-search".into())
            .accessible(Role::Button, LABEL)
            .flex_shrink_0()
            .w(theme::WIDTH_TOOLBAR_SEARCH)
            .h(theme::HEIGHT_CONTROL)
            // The gap of the toolbar is 6, and the field is 8 from the pill.
            .ml(theme::SPACE_2)
            .px(theme::SPACE_8)
            .flex()
            .items_center()
            .gap(theme::SPACE_8)
            .rounded(theme::RADIUS_TRACK)
            .bg(theme::BG_FIELD)
            .border_1()
            .border_color(theme::BORDER_FIELD)
            .text_color(theme::TEXT_TERTIARY)
            .cursor_pointer()
            .hover(|d| d.border_color(theme::BORDER_CONTROL))
            .child(Icon::new(IconName::Search).size(theme::ICON_SEARCH))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(theme::TEXT_CONTROL)
                    .child("Search"),
            )
            .children(crate::settings_window::shortcuts::keys(&Search, cx).map(controls::key_hint))
            .map(|d| controls::keyboard_focus(d, controls::Ring::Outside))
            .on_click(cx.listener(|view, _, window, cx| view.open_palette(window, cx)))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_texts_of_the_palette() {
        assert!(contains("Hide Sidebar", "side"));
        assert!(contains("Hide Sidebar", " SIDE "));
        assert!(contains("Hide Sidebar", ""));
        assert!(!contains("Hide Sidebar", "panel"));
        assert_eq!(no_result("xyz "), "No result for “xyz”.");
        assert_eq!(page_hint("Work", 0), "Work · page 1");
    }
}
