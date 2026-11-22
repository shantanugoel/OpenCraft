//! The font family of a label (decision 0043, section 1): the row "Font" of
//! "Appearance", and the font picker popover.
//!
//! The engine gives the installed families [`label::font_families`], each
//! with the name that the profile stores and the name for the interface. The
//! window sends the stored name, thus a profile loads on a computer with a
//! different language.
//!
//! The popover:
//!
//! - The row "System font", and then the installed families, sorted by their
//!   display name. The current family has a check mark.
//! - The search field, with the cursor when the popover opens. It finds a
//!   family by the display name or by the stored name.
//! - Up and Down move the highlight, Return selects the row, Escape and a
//!   click out of the popover close it.
//!
//! A family that the profile has and the computer does not have draws in the
//! system font, with no error. The row then has the hint "Not installed: the
//! system font shows.", and the profile keeps the name.

use super::*;
use gpui_kit::{
    anchored, assets::IconName, canvas, component::Icon, deferred, list, point, Bounds,
    KeyDownEvent, ListAlignment, ListState, MouseButton, Pixels,
};
use opencraft_engine::{
    label::{self, FontFamily},
    Auto, LabelChange, SurfaceContent,
};
use std::cell::{Cell, Ref, RefCell};

/// The hint of a family that the computer does not have. The row "Font"
/// shows the name, and the title draws in the system font.
pub(super) const MISSING_FONT: &str = "Not installed: the system font shows.";

/// A row of the list: the system font, or one installed family.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Row {
    System,
    Family(FontFamily),
}

/// The state of the font picker.
pub(super) struct FontPicker {
    /// The popover is open.
    pub(super) open: bool,
    search: Entity<InputState>,
    _search_subscription: Subscription,
    /// The installed families of the last background scan. A font that the
    /// user installs while OpenCraft runs shows after the next scan.
    families: RefCell<Vec<FontFamily>>,
    /// The list came from a scan, or from a test: a row can use it.
    loaded: Cell<bool>,
    /// A background scan runs now.
    loading: Cell<bool>,
    /// A scan that started earlier than this number is ignored.
    generation: Cell<u64>,
    /// A test gave the list: do not ask the engine.
    #[cfg(test)]
    fixed: Cell<bool>,
    /// The rows that the search field shows.
    shown: Vec<Row>,
    /// The highlighted row.
    highlight: usize,
    list: ListState,
    /// The bounds of the button in the last frame, for the place of the
    /// popover.
    button: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// The focus of the focus trap of the popover (`controls::popover_focus`).
    focus: FocusHandle,
}

/// The place of the popover in the window. The trailing edge lines up with
/// the trailing edge of the button. The popover is below the button when it
/// fits there, else above it.
struct Place {
    origin: gpui_kit::Point<Pixels>,
    height: Pixels,
}

fn place(button: Bounds<Pixels>, viewport: gpui_kit::Size<Pixels>) -> Place {
    let margin = theme::ICON_PICKER_WINDOW_MARGIN;
    let width = theme::LIST_PICKER_WIDTH;
    let gap = theme::SPACE_4;
    let below_space = viewport.height - margin - (button.bottom() + gap);
    let above_space = button.top() - gap - margin;
    let full = theme::LIST_PICKER_HEIGHT;
    let below = below_space >= full || (above_space < full && below_space >= above_space);
    let height = full
        .min(if below { below_space } else { above_space })
        .max(theme::LIST_PICKER_MIN_HEIGHT);
    let left = (button.right() - width)
        .max(margin)
        .min((viewport.width - margin - width).max(margin));
    let top = if below {
        button.bottom() + gap
    } else {
        button.top() - gap - height
    };
    Place {
        origin: point(left, top),
        height,
    }
}

/// The text of the popup button: "System font", the display name of the
/// family when the list has it, or the stored name. Decision 0043, section
/// 1: the picker shows the display name and sends the stored name.
fn font_button_value(families: &[FontFamily], current: Option<&str>) -> String {
    let Some(stored) = current else {
        return "System font".into();
    };
    families
        .iter()
        .find(|known| known.name == stored)
        .map(|known| known.display.clone())
        .unwrap_or_else(|| stored.to_string())
}

/// The rows for the search text `query`: the system font, and the families
/// whose display name or stored name has the text, with no difference
/// between capital and small letters.
fn rows(families: &[FontFamily], query: &str) -> Vec<Row> {
    let query = query.trim().to_lowercase();
    let mut rows = Vec::new();
    if query.is_empty() || "system font".contains(&query) {
        rows.push(Row::System);
    }
    rows.extend(
        families
            .iter()
            .filter(|family| {
                query.is_empty()
                    || family.display.to_lowercase().contains(&query)
                    || family.name.to_lowercase().contains(&query)
            })
            .cloned()
            .map(Row::Family),
    );
    rows
}

impl FontPicker {
    pub(super) fn new(window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search fonts"));
        let subscription = cx.subscribe_in(&search, window, |view, _, event, _, cx| {
            if matches!(event, TextEvent::Change) {
                view.refresh_font_rows(cx);
                view.inspector.font_picker.highlight = 0;
                view.inspector.font_picker.list.scroll_to_reveal_item(0);
                cx.notify();
            }
        });
        Self {
            open: false,
            search,
            _search_subscription: subscription,
            families: RefCell::new(Vec::new()),
            loaded: Cell::new(false),
            loading: Cell::new(false),
            generation: Cell::new(0),
            #[cfg(test)]
            fixed: Cell::new(false),
            shown: Vec::new(),
            highlight: 0,
            list: ListState::new(0, ListAlignment::Top, theme::HEIGHT_MENU_ROW * 4.),
            button: Rc::new(Cell::new(None)),
            focus: cx.focus_handle(),
        }
    }

    /// The installed families of the last scan. The method does not scan:
    /// the shell scans on a background thread (`DeviceView::start_font_scan`),
    /// thus the UI thread never waits for Core Text.
    pub(super) fn families(&self) -> Ref<'_, Vec<FontFamily>> {
        self.families.borrow()
    }

    /// The list came from a scan, or from a test.
    pub(super) fn loaded(&self) -> bool {
        self.loaded.get()
    }

    /// Start a scan unless one runs, and give its generation. `None` if a
    /// scan runs now, or a test gave the list.
    pub(super) fn start_scan(&self) -> Option<u64> {
        #[cfg(test)]
        if self.fixed.get() {
            return None;
        }
        if self.loading.get() {
            return None;
        }
        self.loading.set(true);
        let generation = self.generation.get() + 1;
        self.generation.set(generation);
        Some(generation)
    }

    /// Use the families of the scan `generation`. An earlier scan gives
    /// `false`, and its list is dropped.
    pub(super) fn finish_scan(&self, generation: u64, families: Vec<FontFamily>) -> bool {
        if self.generation.get() != generation {
            return false;
        }
        self.loading.set(false);
        *self.families.borrow_mut() = families;
        self.loaded.set(true);
        true
    }

    /// Use `families` and do not ask the engine. A window test calls this
    /// before it opens the picker, thus the test does not need the fonts of
    /// the computer.
    #[cfg(test)]
    pub(super) fn use_fixed_families(&self, families: Vec<FontFamily>) {
        *self.families.borrow_mut() = families;
        self.loaded.set(true);
        self.fixed.set(true);
    }

    /// The families that the open popover shows, without the system font.
    #[cfg(test)]
    pub(super) fn shown_families(&self) -> Vec<String> {
        self.shown
            .iter()
            .filter_map(|row| match row {
                Row::Family(family) => Some(family.display.clone()),
                Row::System => None,
            })
            .collect()
    }
}

impl DeviceView {
    /// The row "Font" of "Appearance": the popup button of the family, the
    /// popover, and the hint of a family that is not installed.
    pub(super) fn font_row(
        &self,
        content: &SurfaceContent,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let picker = &self.inspector.font_picker;
        let open = picker.open;
        let bounds = picker.button.clone();
        // The family that shows: the own family of the surface, or else the
        // engine default until the default title style of the profile comes
        // (decision 0043).
        let style = content
            .label_style
            .resolve(&self.state.default_label_style());
        let current = style.font.clone();
        // A named family needs the list for its display name and for the
        // hint. Make the list one time on a background thread, not in this
        // frame.
        if current.is_some() && !picker.loaded() {
            self.start_font_scan(window, cx);
        }
        let families = picker.families();
        let value = font_button_value(&families, current.as_deref());
        let missing = current
            .as_deref()
            .filter(|family| !families.iter().any(|known| &known.name == family));
        drop(families);
        let button = div()
            .flex_1()
            .min_w_0()
            .flex()
            .child(
                controls::popup_button(
                    "label-font",
                    "Font",
                    value.clone(),
                    theme::HEIGHT_CONTROL,
                    open,
                    div().truncate().child(value.clone()),
                )
                .on_click(cx.listener(move |view, _, window, cx| {
                    if view.inspector.font_picker.open {
                        view.close_font_picker(window, cx);
                    } else {
                        view.open_font_picker(window, cx);
                    }
                })),
            )
            .child(
                canvas(
                    move |bounds_now, _, _| bounds.set(Some(bounds_now)),
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .when(open, |row| row.child(self.font_popover(window, cx)));
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(theme::SPACE_4)
            // The button fills the value column, as the title field and the
            // segments do: a long family name has room and ends with "…".
            .child(div().relative().flex().child(button))
            .when_some(missing, |row, _family| {
                // The button above shows the name of the family, thus the
                // hint is short (decision 0043, section 1).
                row.child(
                    form::help(MISSING_FONT)
                        .min_w_0()
                        .id("label-font-missing")
                        .debug_selector(|| "label-font-missing".into())
                        // The hint is a text that VoiceOver must read: it
                        // says that the family of the profile is not
                        // installed.
                        .readable_text(MISSING_FONT),
                )
            })
            .into_any_element()
    }

    /// Start the scan of the installed families on the background executor.
    /// The open popover shows the last list until the scan finishes, and a
    /// first open shows "Loading the fonts…".
    fn start_font_scan(&self, window: &Window, cx: &mut Context<Self>) {
        let Some(generation) = self.inspector.font_picker.start_scan() else {
            return;
        };
        cx.spawn_in(window, async move |view, cx| {
            let families = cx
                .background_executor()
                .spawn(async { label::font_families() })
                .await;
            let _ = view.update_in(cx, |view, _, cx| {
                if !view.inspector.font_picker.finish_scan(generation, families) {
                    return;
                }
                view.refresh_font_rows(cx);
                if view.inspector.font_picker.open {
                    view.highlight_current_font();
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Open the font picker: the families of now, an empty search field with
    /// the cursor, and the highlight on the current family.
    pub(super) fn open_font_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_kinds();
        // A font that the user installed while OpenCraft runs shows after
        // this scan. The UI thread does not wait for it.
        self.start_font_scan(window, cx);
        let picker = &mut self.inspector.font_picker;
        picker.open = true;
        let search = picker.search.clone();
        search.update(cx, |search, cx| {
            search.set_value("", window, cx);
            search.focus(window, cx);
        });
        self.refresh_font_rows(cx);
        self.highlight_current_font();
        cx.notify();
    }

    /// Close the font picker, and give the focus back to the window.
    fn close_font_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.inspector.font_picker.open {
            return;
        }
        self.inspector.font_picker.open = false;
        self.focus_workspace(window, cx);
        cx.notify();
    }

    /// Make the rows again for the search text, and show the start of the
    /// list.
    fn refresh_font_rows(&mut self, cx: &Context<Self>) {
        let shown = if self.inspector.font_picker.loaded() {
            let families = self.inspector.font_picker.families();
            let query = self
                .inspector
                .font_picker
                .search
                .read(cx)
                .value()
                .to_string();
            rows(&families, &query)
        } else {
            // The first scan of the fonts is not ready. The empty state says
            // so, and the rows come when the scan finishes.
            Vec::new()
        };
        let picker = &mut self.inspector.font_picker;
        if picker.shown != shown {
            picker.list.reset(shown.len());
            picker.shown = shown;
        }
        if picker.highlight >= picker.shown.len() {
            picker.highlight = 0;
        }
    }

    /// The highlight on the family that shows: the system font, or the own
    /// family of the surface.
    fn highlight_current_font(&mut self) {
        let Some(surface) = self.state.edit_surface() else {
            return;
        };
        let content = self.edited_face(surface);
        let style = content
            .label_style
            .resolve(&self.state.default_label_style());
        let picker = &mut self.inspector.font_picker;
        let index = match &style.font {
            None => 0,
            Some(family) => picker
                .shown
                .iter()
                .position(|row| match row {
                    Row::System => false,
                    Row::Family(known) => &known.name == family,
                })
                .unwrap_or(0),
        };
        picker.highlight = index;
        picker.list.scroll_to_reveal_item(index);
    }

    fn move_font_highlight(&mut self, delta: isize) {
        let count = self.inspector.font_picker.shown.len();
        if count == 0 {
            return;
        }
        let current = self.inspector.font_picker.highlight as isize;
        let next = (current + delta).rem_euclid(count as isize) as usize;
        self.inspector.font_picker.highlight = next;
        self.inspector.font_picker.list.scroll_to_reveal_item(next);
    }

    /// Use the highlighted row on the selected surface.
    fn choose_highlighted_font(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let index = self.inspector.font_picker.highlight;
        let Some(row) = self.inspector.font_picker.shown.get(index).cloned() else {
            return;
        };
        self.choose_font(row, window, cx);
    }

    /// Use the row `row` on the selected surface. The profile stores the
    /// name of the family, and not the name for the interface.
    fn choose_font(&mut self, row: Row, window: &mut Window, cx: &mut Context<Self>) {
        // A click on the checked row changes nothing. Without this test the
        // engine records an undo step and stores the automatic font, which
        // stops an inheritance of the profile default (review 2, M4).
        let current = self.state.edit_surface().and_then(|surface| {
            self.edited_face(surface)
                .label_style
                .resolve(&self.state.default_label_style())
                .font
        });
        let checked = match &row {
            Row::System => current.is_none(),
            Row::Family(family) => current.as_deref() == Some(family.name.as_str()),
        };
        if checked {
            self.close_font_picker(window, cx);
            return;
        }
        let change = match row {
            Row::System => LabelChange::Font(Some(Auto::Auto)),
            Row::Family(family) => LabelChange::Font(Some(Auto::Value(family.name))),
        };
        self.set_label_style(change);
        self.close_font_picker(window, cx);
    }

    /// The popover of the font picker, next to the button.
    fn font_popover(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let viewport = window.viewport_size();
        let Some(button) = self.inspector.font_picker.button.get() else {
            // The first frame has no bounds of the button yet.
            window.request_animation_frame();
            return div().into_any_element();
        };
        let place = place(button, viewport);
        let close = cx.listener(|view, _: &gpui_kit::MouseDownEvent, window, cx| {
            view.close_font_picker(window, cx);
        });
        div()
            .child(
                deferred(
                    anchored().position(point(px(0.), px(0.))).child(
                        div()
                            .id("font-picker-backdrop")
                            .w(viewport.width)
                            .h(viewport.height)
                            .occlude()
                            .on_mouse_down(MouseButton::Left, close),
                    ),
                )
                .with_priority(1),
            )
            .child(
                deferred(
                    anchored().position(place.origin).child(
                        div()
                            .id("font-picker")
                            .debug_selector(|| "font-picker".into())
                            .accessible(Role::Dialog, "Choose a font")
                            .occlude()
                            .relative()
                            .w(theme::LIST_PICKER_WIDTH)
                            .h(place.height)
                            .capture_key_down(cx.listener(
                                |view, event: &KeyDownEvent, window, cx| {
                                    if event.keystroke.modifiers.modified() {
                                        return;
                                    }
                                    match event.keystroke.key.as_str() {
                                        "escape" => {
                                            cx.stop_propagation();
                                            view.close_font_picker(window, cx);
                                        }
                                        "up" => {
                                            cx.stop_propagation();
                                            view.move_font_highlight(-1);
                                            cx.notify();
                                        }
                                        "down" => {
                                            cx.stop_propagation();
                                            view.move_font_highlight(1);
                                            cx.notify();
                                        }
                                        "enter" => {
                                            cx.stop_propagation();
                                            view.choose_highlighted_font(window, cx);
                                        }
                                        _ => {}
                                    }
                                },
                            ))
                            .child(
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    .overflow_hidden()
                                    .rounded(theme::RADIUS_POPOVER)
                                    .bg(theme::bg_field())
                                    .border_1()
                                    .border_color(theme::border_overlay())
                                    .shadow(theme::popover_shadow())
                                    .text_size(theme::TEXT_CONTROL)
                                    .font_weight(gpui_kit::FontWeight::NORMAL)
                                    .text_color(theme::text_primary())
                                    .child(self.font_picker_search(window, cx))
                                    .child(self.font_list(cx)),
                            )
                            .map(|popover| {
                                controls::popover_focus(
                                    popover,
                                    "font-picker-trap".into(),
                                    &self.inspector.font_picker.focus,
                                )
                            }),
                    ),
                )
                .with_priority(2),
            )
            .into_any_element()
    }

    fn font_picker_search(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        div()
            .flex_shrink_0()
            .p(theme::SPACE_12)
            .border_b_1()
            .border_color(theme::border_field())
            .child(
                controls::search_field(
                    "font-search",
                    "Search fonts",
                    &self.inspector.font_picker.search,
                    theme::HEIGHT_SEARCH_FIELD,
                    window,
                    cx,
                )
                .bg(theme::bg_bar()),
            )
    }

    fn font_list(&self, cx: &mut Context<Self>) -> Div {
        let frame = div().flex_1().min_h_0().relative();
        if self.inspector.font_picker.shown.is_empty() {
            let text = self.font_empty_text(cx);
            return frame.child(
                div()
                    .id("font-empty")
                    .debug_selector(|| "font-empty".into())
                    .pt(theme::EMPTY_STATE_TOP)
                    .px(theme::SPACE_16)
                    .text_center()
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::text_tertiary())
                    .readable_text(text.clone())
                    .child(text),
            );
        }
        let shown = self.inspector.font_picker.shown.clone();
        let view = cx.entity();
        frame
            .child(
                list(
                    self.inspector.font_picker.list.clone(),
                    move |index, _, cx| {
                        let row = shown[index].clone();
                        view.update(cx, |view, cx| view.font_picker_row(index, &row, cx))
                    },
                )
                .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .child(Scrollbar::vertical(&self.inspector.font_picker.list)),
            )
    }

    fn font_empty_text(&self, cx: &Context<Self>) -> String {
        if !self.inspector.font_picker.loaded() {
            return "Loading the fonts…".into();
        }
        let query = self.inspector.font_picker.search.read(cx).value();
        let query = query.trim();
        if query.is_empty() {
            return "No font is installed.".into();
        }
        format!("No font matches “{query}”.")
    }

    fn font_picker_row(&self, index: usize, row: &Row, cx: &mut Context<Self>) -> AnyElement {
        // The family that shows: the own family of the surface, or else the
        // engine default.
        let current = self.state.edit_surface().map(|surface| {
            self.edited_face(surface)
                .label_style
                .resolve(&self.state.default_label_style())
                .font
        });
        let checked = match row {
            Row::System => current.flatten().is_none(),
            Row::Family(family) => current.flatten().as_deref() == Some(family.name.as_str()),
        };
        let name = match row {
            Row::System => "System font".to_string(),
            Row::Family(family) => family.display.clone(),
        };
        let highlighted = self.inspector.font_picker.highlight == index;
        let row_id = format!("font-row-{index}");
        let selector = row_id.clone();
        let row = row.clone();
        div()
            .id(SharedString::from(row_id))
            .debug_selector(move || selector)
            .w_full()
            .h(theme::HEIGHT_MENU_ROW)
            .mx(theme::SPACE_6)
            .px(theme::SPACE_8)
            .flex()
            .items_center()
            .gap(theme::SPACE_8)
            .rounded(theme::RADIUS_CONTROL)
            .cursor_pointer()
            .when(highlighted, |d| d.bg(theme::menu_highlight()))
            .accessible(Role::MenuItem, name.clone())
            .child(
                div()
                    .flex_shrink_0()
                    .w(theme::ICON_SMALL)
                    .when(checked, |d| {
                        d.child(Icon::new(IconName::Check).size(theme::ICON_SMALL))
                    }),
            )
            .child(div().flex_1().min_w_0().truncate().child(name))
            .on_mouse_move(cx.listener(move |view, _, _, cx| {
                if view.inspector.font_picker.highlight != index {
                    view.inspector.font_picker.highlight = index;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |view, _, window, cx| {
                view.choose_font(row.clone(), window, cx);
            }))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_button_shows_the_display_name_of_the_family() {
        let families = [
            FontFamily {
                name: "Hiragino Sans GB".into(),
                display: "ヒラギノ角ゴ 簡体中文".into(),
            },
            FontFamily {
                name: "Menlo".into(),
                display: "Menlo".into(),
            },
        ];
        assert_eq!(font_button_value(&families, None), "System font");
        assert_eq!(
            font_button_value(&families, Some("Hiragino Sans GB")),
            "ヒラギノ角ゴ 簡体中文"
        );
        assert_eq!(font_button_value(&families, Some("Menlo")), "Menlo");
        // A family that is not installed keeps its stored name.
        assert_eq!(
            font_button_value(&families, Some("Missing Family")),
            "Missing Family"
        );
    }
}
