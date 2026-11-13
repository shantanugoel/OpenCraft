//! The profile settings sheet (decision 0029, section 16): the name, the
//! colour, and the first page of one profile, the export of a profile file,
//! and the removal of the profile.
//!
//! "Profile Settings…" in the right-click menu of a profile row opens the
//! sheet, also for a profile that is not active. Each change goes to the
//! engine at once, as in the inspector. The name goes to the engine when the
//! field loses the focus, and when the sheet closes. "Done", Return, and
//! Escape close the sheet. The frame of the sheet is the shared modal layer
//! of `modal`.
//!
//! The engine writes the files (rule 9): the export and the import are the
//! commands `ExportProfile` and `ImportProfile`. The file format is in
//! decision 0031.

use super::{
    controls::{self, FieldSize},
    modal::{button_row, sheet_row, sheet_section_header, sheet_title, Modal, ModalKey},
    pages::{page_title, Row},
    *,
};
use gpui_kit::component::input::AnyInputState;
use gpui_kit::{assets::IconName, component::Icon, FontWeight};
use opencraft_engine::share::EXTENSION;
use std::path::PathBuf;

/// The state of the sheet.
pub(super) struct ProfileSheet {
    /// The identity of the profile that the sheet shows. `None`: the sheet
    /// is closed.
    pub(super) profile: Option<String>,
    /// The focus of the card of the sheet.
    pub(super) focus: FocusHandle,
    /// The field "Name".
    pub(super) name: Entity<InputState>,
    /// The last name that the sheet sent, so that it sends a name one time.
    sent_name: Option<String>,
    /// True while the menu of "First page" is open.
    pub(super) pages_open: bool,
    _subscription: Subscription,
}

impl ProfileSheet {
    pub(super) fn new(window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let subscription = cx.subscribe_in(&name, window, |view, _, event, _, cx| {
            if matches!(event, TextEvent::Blur) {
                view.save_sheet_name(cx);
            }
        });
        Self {
            profile: None,
            focus: cx.focus_handle(),
            name,
            sent_name: None,
            pages_open: false,
            _subscription: subscription,
        }
    }
}

/// A click handler of a control of the sheet.
type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// The click on a colour circle: send the colour `color` for the profile of
/// `entry`, if it is a new colour.
fn choose_colour(
    entry: &opencraft_engine::ProfileEntry,
    color: Option<Color>,
    cx: &mut Context<DeviceView>,
) -> ClickHandler {
    let id = entry.id.clone();
    let current = entry.color;
    Box::new(cx.listener(move |view, _, _, cx| {
        view.sheet.pages_open = false;
        if current != color {
            (view.send)(Command::SetProfileColor {
                profile: id.clone(),
                color,
            });
        }
        cx.notify();
    }))
}

/// The name of a page count, for example "3 pages" or "1 page".
fn page_count(count: usize) -> String {
    if count == 1 {
        "1 page".into()
    } else {
        format!("{count} pages")
    }
}

/// A file name for the export of the profile `name`: the name with no
/// character that macOS does not accept in a file name, and the extension.
fn file_name(name: &str) -> String {
    let base: String = name
        .trim()
        .chars()
        .map(|c| if matches!(c, '/' | ':') { '-' } else { c })
        .collect();
    let base = base.trim_start_matches('.');
    let base = if base.is_empty() { "Profile" } else { base };
    format!("{base}.{EXTENSION}")
}

/// The folder that the save dialog shows first: the Documents folder of
/// the user, if it exists. macOS keeps the last folder of the dialog.
fn export_folder() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    match home {
        Some(home) if home.join("Documents").is_dir() => home.join("Documents"),
        Some(home) => home,
        None => PathBuf::from("/"),
    }
}

impl DeviceView {
    /// Open the sheet for the profile `id`.
    pub(super) fn open_profile_sheet(
        &mut self,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(name) = self.profile(&id).map(|p| p.name.clone()) else {
            return;
        };
        self.end_text_edit(cx);
        self.sheet.profile = Some(id);
        self.sheet.sent_name = None;
        self.sheet.pages_open = false;
        self.sheet
            .name
            .update(cx, |input, cx| input.set_value(name, window, cx));
        window.focus(&self.sheet.focus, cx);
        cx.notify();
    }

    /// Close the sheet. A new name in the field goes to the engine first.
    pub(super) fn close_profile_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save_sheet_name(cx);
        self.sheet.profile = None;
        self.sheet.pages_open = false;
        self.focus_workspace(window, cx);
        cx.notify();
    }

    /// Send the name in the field, if it is a new name. An empty name is
    /// not possible: the profile keeps its name.
    fn save_sheet_name(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.sheet.profile.clone() else {
            return;
        };
        let name = self.sheet.name.read(cx).value().trim().to_string();
        let current = self.profile(&id).map(|p| p.name.clone());
        if name.is_empty()
            || current.as_deref() == Some(name.as_str())
            || self.sheet.sent_name.as_deref() == Some(name.as_str())
        {
            return;
        }
        self.sheet.sent_name = Some(name.clone());
        (self.send)(Command::RenameProfile { profile: id, name });
    }

    /// Apply the part of an engine event that the sheet uses: the sheet
    /// closes when its profile goes, and an export or an import shows a
    /// message in the status bar.
    pub(super) fn profile_sheet_event(
        &mut self,
        event: &Event,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            Event::Profiles { profiles, .. } => {
                if let Some(id) = &self.sheet.profile {
                    if !profiles.iter().any(|p| &p.id == id) {
                        self.close_profile_sheet(window, cx);
                    }
                }
            }
            Event::ProfileExported { profile, path } => {
                let name = self.profile(profile).map(|p| p.name.clone());
                let file = path
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if let Some(name) = name {
                    self.show_message(format!("“{name}” is in the file “{file}”"), window, cx);
                }
            }
            Event::ProfileImported { profile } => {
                let Some(name) = self.profile(profile).map(|p| p.name.clone()) else {
                    return;
                };
                self.show_message(
                    format!("The profile “{name}” is imported, and it is on the device"),
                    window,
                    cx,
                );
            }
            _ => {}
        }
    }

    /// Ask for a file, and send the export of the profile of the sheet.
    fn export_profile(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.sheet.profile.clone() else {
            return;
        };
        self.save_sheet_name(cx);
        let name = self.sheet.name.read(cx).value().trim().to_string();
        let path = cx.prompt_for_new_path(&export_folder(), Some(&file_name(&name)));
        cx.spawn_in(window, async move |view, cx| {
            let result = path.await;
            let _ = view.update_in(cx, |view, window, cx| {
                match result {
                    Ok(Ok(Some(path))) => (view.send)(Command::ExportProfile { profile: id, path }),
                    Ok(Ok(None)) => {}
                    _ => view
                        .state
                        .problems
                        .push("OpenCraft could not open the file dialog.".into()),
                }
                // The focus comes back to the sheet after the dialog.
                if view.sheet.profile.is_some() {
                    window.focus(&view.sheet.focus, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Escape and Return in the sheet. An open menu gets the two keys
    /// first: Escape closes it, and Return selects its highlighted item
    /// (`view::menu`), and neither closes the sheet.
    fn sheet_key(&mut self, key: ModalKey, window: &mut Window, cx: &mut Context<Self>) {
        if self.sheet.pages_open {
            if key == ModalKey::Escape {
                self.sheet.pages_open = false;
                cx.notify();
            }
            return;
        }
        self.close_profile_sheet(window, cx);
    }

    /// The pages of the profile `id`, and its first page. The engine sends
    /// the current pages of the active profile in `Event::Pages`.
    pub(super) fn sheet_pages(&self, id: &str) -> (Vec<String>, usize) {
        if self.state.profile.as_deref() == Some(id) {
            return (self.state.pages.clone(), self.state.first_page);
        }
        self.profile(id)
            .map(|p| (p.pages.clone(), p.first_page))
            .unwrap_or_default()
    }

    /// The sheet, if it is open.
    pub(super) fn profile_sheet(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let id = self.sheet.profile.clone()?;
        let entry = self.profile(&id)?.clone();
        let (pages, first) = self.sheet_pages(&id);
        let view = cx.entity();
        let modal = Modal::new(
            "profile-sheet",
            &self.sheet.focus,
            format!("Profile settings for {}", entry.name),
        )
        .on_key(move |key, window, cx| view.update(cx, |view, cx| view.sheet_key(key, window, cx)));
        let form = div()
            .flex_shrink_0()
            .px(theme::SPACE_24)
            .flex()
            .flex_col()
            .gap(theme::SPACE_12)
            .child(sheet_row(
                "Name",
                controls::text_field_sized(
                    &AnyInputState::Input(self.sheet.name.clone()),
                    "Name",
                    theme::BG_BAR,
                    FieldSize::sheet(),
                    window,
                    cx,
                )
                .id("profile-sheet-name")
                .debug_selector(|| "profile-sheet-name".into()),
            ))
            .child(sheet_row("Colour", self.colour_circles(&entry, cx)))
            .child(sheet_row(
                "First page",
                self.first_page_popup(&id, &pages, first, window, cx),
            ));
        // "Switch to this profile automatically" is a LATER place between
        // the form and "Share" (decision 0029, section 21). It is one more
        // section in this list.
        let share = div()
            .flex_shrink_0()
            .mt(theme::SPACE_16)
            .px(theme::SPACE_24)
            .pb(theme::SPACE_24)
            .flex()
            .flex_col()
            .gap(theme::SPACE_8)
            .child(sheet_section_header("Share"))
            .child(
                // "Import from VSD Craft" is a LATER place in this row.
                div().flex().items_center().gap(theme::SPACE_8).child(
                    controls::button("profile-export", "Export as a file…").on_click(
                        cx.listener(|view, _, window, cx| view.export_profile(window, cx)),
                    ),
                ),
            );
        let can_remove = self.state.profiles.len() > 1;
        let remove =
            controls::sheet_button("profile-remove", "Remove the profile…", true, can_remove)
                .when(can_remove, |d| {
                    let id = id.clone();
                    d.on_click(cx.listener(move |view, _, window, cx| {
                        view.save_sheet_name(cx);
                        view.confirm_remove(Row::Profile(id.clone()), window, cx)
                    }))
                })
                .into_any_element();
        let done = controls::main_button("profile-done", "Done", theme::SPACE_18)
            .on_click(cx.listener(|view, _, window, cx| view.close_profile_sheet(window, cx)))
            .into_any_element();
        let content = div()
            .flex()
            .flex_col()
            .child(sheet_title(
                "Profile settings",
                format!("Profile “{}” · {}", entry.name, page_count(pages.len())),
            ))
            .child(form)
            .child(share)
            .child(button_row(Some(remove), vec![done]));
        Some(modal.render(content, window))
    }

    /// The colour circles: "None" and the colours of
    /// [`theme::PROFILE_COLOURS`].
    fn colour_circles(
        &self,
        entry: &opencraft_engine::ProfileEntry,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let none = controls::colour_circle(
            "profile-colour-none",
            None,
            entry.color.is_none(),
            "No colour",
        )
        .on_click(choose_colour(entry, None, cx));
        let circles: Vec<_> = theme::PROFILE_COLOURS
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                let color = colour(value);
                controls::colour_circle(
                    format!("profile-colour-{index}"),
                    Some(theme::sample(value)),
                    entry.color == Some(color),
                    format!("Colour #{value:06X}"),
                )
                .on_click(choose_colour(entry, Some(color), cx))
            })
            .collect();
        div()
            .flex()
            .items_center()
            .gap(theme::SPACE_8)
            .child(none)
            .children(circles)
            .into_any_element()
    }

    /// The popup button "First page" with the names of the pages, and its
    /// menu.
    fn first_page_popup(
        &self,
        id: &str,
        pages: &[String],
        first: usize,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let open = self.sheet.pages_open;
        let menu = open.then(|| {
            let rows = (0..pages.len())
                .map(|page| {
                    let id = id.to_string();
                    controls::menu_row(format!("first-page-{page}"))
                        .label(page_title(pages, page))
                        .child(
                            div()
                                .flex_shrink_0()
                                .w(theme::ICON_SMALL)
                                .when(page == first, |d| {
                                    d.child(Icon::new(IconName::Check).size(theme::ICON_SMALL))
                                }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .child(page_title(pages, page)),
                        )
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.sheet.pages_open = false;
                            if page != first {
                                (view.send)(Command::SetFirstPage {
                                    profile: id.clone(),
                                    page,
                                });
                            }
                            cx.notify();
                        }))
                        .into_any_element()
                })
                .collect();
            controls::popup_menu(
                "first-page-menu",
                controls::MenuPlace {
                    button_height: theme::HEIGHT_SHEET_CONTROL,
                    width: theme::WIDTH_SHEET_POPUP,
                    align_end: false,
                },
                rows,
                window,
                cx.listener(|view, _, _, cx| {
                    view.sheet.pages_open = false;
                    cx.notify();
                }),
            )
        });
        div()
            .relative()
            .flex_shrink_0()
            .w(theme::WIDTH_SHEET_POPUP)
            .flex()
            .child(
                controls::popup_button(
                    "first-page",
                    "First page",
                    page_title(pages, first),
                    theme::HEIGHT_SHEET_CONTROL,
                    open,
                    div()
                        .truncate()
                        .font_weight(FontWeight::MEDIUM)
                        .child(page_title(pages, first)),
                )
                .rounded(theme::RADIUS_TRACK)
                .px(theme::SPACE_10)
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.sheet.pages_open = !open;
                    cx.notify();
                })),
            )
            .children(menu)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn a_file_name_has_the_name_of_the_profile_and_the_extension() {
        assert_eq!(file_name("Work"), "Work.opencraft");
        assert_eq!(file_name(" a/b:c "), "a-b-c.opencraft");
        assert_eq!(file_name("..."), "Profile.opencraft");
        assert_eq!(file_name(""), "Profile.opencraft");
    }

    #[test]
    fn a_page_count_has_the_form_of_the_number() {
        assert_eq!(page_count(1), "1 page");
        assert_eq!(page_count(3), "3 pages");
    }

    #[test]
    fn the_export_folder_exists_or_is_the_root() {
        let folder = export_folder();
        assert!(folder.is_dir() || folder == Path::new("/"));
    }
}
