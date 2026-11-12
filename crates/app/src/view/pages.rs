//! The changes of the pages and the profiles that the sidebar starts
//! (decision 0029, sections 7.3 and 7.5).
//!
//! The sidebar shows the pages and the profiles of the engine, and its
//! clicks and menus send commands only. The window shows the active page of
//! the engine: when an action on the device changes the page, the sidebar
//! follows. The commands are in decisions 0016 and 0018.

use super::*;
use gpui_kit::{PromptButton, PromptLevel};
use opencraft_engine::ProfileEntry;

/// A row of the sidebar that has a name. A profile row has the identity of
/// the profile, and a page row has the number of the page in the active
/// profile, from 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Row {
    Profile(String),
    Page(usize),
}

/// The edit of a name in a row of the sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum NameEdit {
    /// The name of a new profile, in a new row below the profiles.
    NewProfile,
    /// A new name for the profile or the page of the row.
    Rename(Row),
}

/// The state of the sidebar rows. Only the engine events change the pages
/// and the profiles.
pub(super) struct Lists {
    pub(super) edit: Option<NameEdit>,
    /// The text field of the edit.
    pub(super) name: Entity<InputState>,
    /// The row of the right-click menu. A menu item does its work on it.
    pub(super) menu_row: Option<Row>,
    /// True when the user closed the pages of the active profile with its
    /// disclosure triangle.
    pub(super) pages_hidden: bool,
    _subscription: Subscription,
}

impl Lists {
    pub(super) fn new(window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx));
        // A click out of the field keeps the name. The row handles Return
        // and Escape before the field (`DeviceView::name_field`).
        let subscription = cx.subscribe_in(&name, window, |view, _, event, window, cx| {
            if matches!(event, TextEvent::Blur) {
                view.save_name_on_blur(window, cx);
            }
        });
        Self {
            edit: None,
            name,
            menu_row: None,
            pages_hidden: false,
            _subscription: subscription,
        }
    }
}

/// The name of a page for the user. A page with no name has its number.
pub(super) fn page_title(names: &[String], page: usize) -> String {
    match names.get(page) {
        Some(name) if !name.trim().is_empty() => name.clone(),
        _ => format!("Page {}", page + 1),
    }
}

/// `base`, or `base N` with the lowest N from 2 that no name in `names` has.
fn free_name<'a>(base: &str, names: impl Iterator<Item = &'a str> + Clone) -> String {
    let free = |name: &str| !names.clone().any(|n| n == name);
    if free(base) {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|name| free(name))
        .expect("a free name exists")
}

/// An item of the right-click menu of a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MenuItem {
    Rename,
    Duplicate,
    MoveUp,
    MoveDown,
    Separator,
    Remove,
}

impl MenuItem {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Rename => "Rename",
            Self::Duplicate => "Duplicate",
            Self::MoveUp => "Move Up",
            Self::MoveDown => "Move Down",
            Self::Separator => "",
            Self::Remove => "Remove…",
        }
    }
}

impl DeviceView {
    /// Apply the part of an engine event that the sidebar uses. An edit or a
    /// menu of a page stops when the names of the pages change: its page
    /// number can be a different page now. An edit or a menu of a profile
    /// stops when the profile goes. The result is true when an edit
    /// stopped.
    pub(super) fn update_lists(&mut self, event: &Event) -> bool {
        let state = &self.state;
        let gone = |row: &Row| match (row, event) {
            (Row::Page(_), Event::Pages { names, .. }) => *names != state.pages,
            (Row::Page(_), Event::Profiles { active, .. }) => {
                state.profile.as_ref() != Some(active)
            }
            (Row::Profile(id), Event::Profiles { profiles, .. }) => {
                !profiles.iter().any(|p| &p.id == id)
            }
            _ => false,
        };
        if self.lists.menu_row.as_ref().is_some_and(gone) {
            self.lists.menu_row = None;
        }
        let stopped = matches!(&self.lists.edit, Some(NameEdit::Rename(row)) if gone(row));
        if stopped {
            self.lists.edit = None;
        }
        // A new active profile shows its pages.
        if matches!(event, Event::Profiles { active, .. } if state.profile.as_ref() != Some(active))
        {
            self.lists.pages_hidden = false;
        }
        stopped
    }

    pub(super) fn profile(&self, id: &str) -> Option<&ProfileEntry> {
        self.state.profiles.iter().find(|p| p.id == id)
    }

    /// The items of the right-click menu of `row`, and if each one is
    /// enabled.
    pub(super) fn menu_items(&self, row: &Row) -> Vec<(MenuItem, bool)> {
        match row {
            Row::Profile(_) => vec![
                (MenuItem::Rename, true),
                (MenuItem::Duplicate, true),
                (MenuItem::Separator, true),
                (MenuItem::Remove, self.state.profiles.len() > 1),
            ],
            Row::Page(page) => {
                let count = self.state.pages.len();
                vec![
                    (MenuItem::Rename, true),
                    (MenuItem::Duplicate, true),
                    (MenuItem::MoveUp, *page > 0),
                    (MenuItem::MoveDown, page + 1 < count),
                    (MenuItem::Separator, true),
                    (MenuItem::Remove, count > 1),
                ]
            }
        }
    }

    /// Do the work of a menu item on the row of the right-click menu. An
    /// item that is not enabled does nothing.
    pub(super) fn menu_choice(
        &mut self,
        item: MenuItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(row) = self.lists.menu_row.take() else {
            return;
        };
        if !self.menu_items(&row).contains(&(item, true)) {
            return;
        }
        match (item, &row) {
            (MenuItem::Rename, _) => self.start_rename(row, window, cx),
            (MenuItem::Duplicate, Row::Page(page)) => {
                (self.send)(Command::AddPage {
                    copy_of: Some(*page),
                });
            }
            (MenuItem::Duplicate, Row::Profile(id)) => self.duplicate_profile(id),
            (MenuItem::MoveUp, Row::Page(page)) => (self.send)(Command::MovePage {
                page: *page,
                to: page - 1,
            }),
            (MenuItem::MoveDown, Row::Page(page)) => (self.send)(Command::MovePage {
                page: *page,
                to: page + 1,
            }),
            (MenuItem::Remove, _) => self.confirm_remove(row, window, cx),
            _ => {}
        }
        cx.notify();
    }

    /// Add a copy of a profile with the name "*name* copy". The engine makes
    /// the copy the active profile.
    fn duplicate_profile(&self, id: &str) {
        let Some(profile) = self.profile(id) else {
            return;
        };
        let name = free_name(
            &format!("{} copy", profile.name),
            self.state.profiles.iter().map(|p| p.name.as_str()),
        );
        (self.send)(Command::AddProfile {
            name,
            copy_of: Some(id.to_string()),
        });
    }

    /// The name of a row now.
    fn row_name(&self, row: &Row) -> Option<String> {
        match row {
            Row::Profile(id) => self.profile(id).map(|p| p.name.clone()),
            Row::Page(page) => {
                (*page < self.state.pages.len()).then(|| page_title(&self.state.pages, *page))
            }
        }
    }

    /// Start the edit of the name of `row` in the row. The old name is
    /// selected, so typed text replaces it.
    pub(super) fn start_rename(&mut self, row: Row, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.row_name(&row) else {
            return;
        };
        self.lists.edit = Some(NameEdit::Rename(row));
        self.edit_name(name, window, cx);
    }

    /// Add a row for a new profile, and start the edit of its name.
    pub(super) fn start_new_profile(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = free_name(
            "New profile",
            self.state.profiles.iter().map(|p| p.name.as_str()),
        );
        self.lists.edit = Some(NameEdit::NewProfile);
        self.edit_name(name, window, cx);
    }

    fn edit_name(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        // The field is not in the window yet, so an action cannot reach it.
        self.lists.name.update(cx, |input, cx| {
            input.set_value(text, window, cx);
            input.select_all(window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    /// Keep the name of the edit. An empty name is not possible: then the
    /// edit stays open.
    pub(super) fn save_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.lists.name.read(cx).value().trim().to_string();
        if self.lists.edit.is_none() || name.is_empty() {
            return;
        }
        match self.lists.edit.take() {
            Some(NameEdit::NewProfile) => (self.send)(Command::AddProfile {
                name,
                copy_of: None,
            }),
            // The same name sends nothing.
            Some(NameEdit::Rename(Row::Profile(id)))
                if self.profile(&id).is_some_and(|p| p.name != name) =>
            {
                (self.send)(Command::RenameProfile { profile: id, name });
            }
            Some(NameEdit::Rename(Row::Page(page)))
                if self.state.pages.get(page).is_some_and(|p| *p != name) =>
            {
                (self.send)(Command::RenamePage { page, name });
            }
            _ => {}
        }
        self.focus_workspace(window, cx);
        cx.notify();
    }

    /// A click out of the field keeps the name. With an empty name, the
    /// edit stops and nothing changes.
    fn save_name_on_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.lists.name.read(cx).value().trim().is_empty() {
            self.cancel_name(window, cx);
        } else {
            self.save_name(window, cx);
        }
    }

    /// Stop the edit with no change.
    pub(super) fn cancel_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.lists.edit.take().is_some() {
            self.focus_workspace(window, cx);
        }
        cx.notify();
    }

    /// Ask the user to confirm the removal of `row`. The question names the
    /// profile or the page. The removal occurs only if the row is the same
    /// row when the user answers.
    fn confirm_remove(&mut self, row: Row, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.row_name(&row) else {
            return;
        };
        let (message, detail) = match &row {
            Row::Profile(_) => (
                format!("Remove the profile “{name}”?"),
                "OpenCraft does not delete it. It moves the profile to the folder “removed” \
                 in the profiles folder.",
            ),
            Row::Page(_) => (
                format!("Remove the page “{name}”?"),
                "The page, its key faces, and its bindings go away.",
            ),
        };
        let pages = self.state.pages.clone();
        let answer = window.prompt(
            PromptLevel::Warning,
            &message,
            Some(detail),
            &[PromptButton::ok("Remove"), PromptButton::cancel("Cancel")],
            cx,
        );
        cx.spawn_in(window, async move |view, cx| {
            if answer.await != Ok(0) {
                return;
            }
            let _ = view.update(cx, |view, _| match row {
                Row::Profile(profile) => {
                    if view.profile(&profile).is_some() {
                        (view.send)(Command::RemoveProfile { profile });
                    }
                }
                // A change of the pages during the question can give the
                // number to a different page.
                Row::Page(page) => {
                    if view.state.pages == pages {
                        (view.send)(Command::RemovePage { page });
                    }
                }
            });
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_free_name_gets_the_lowest_free_number() {
        let names = ["Work", "Work copy", "Work copy 2"];
        assert_eq!(free_name("Home", names.iter().copied()), "Home");
        assert_eq!(free_name("Work copy", names.iter().copied()), "Work copy 3");
    }

    #[test]
    fn a_page_with_no_name_has_its_number() {
        let names = ["Main".to_string(), " ".to_string()];
        assert_eq!(page_title(&names, 0), "Main");
        assert_eq!(page_title(&names, 1), "Page 2");
    }
}
