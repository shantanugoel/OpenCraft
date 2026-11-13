//! The undo and redo history of the active profile.
//!
//! The engine writes the profile (architecture rule 9), thus the engine
//! keeps the history. A step keeps a copy of the pages and the folders of
//! the profile, and the location: the active page and the open folders. An
//! undo puts the copy back, and keeps the pages and the folders that it
//! replaced for the redo. Thus a new command that changes the pages or the
//! folders needs no inverse command: it needs only a name. See
//! [`crate::engine::Command`] and
//! `docs/decisions/0030-undo-and-redo-in-the-engine.md`. Decision 0036 adds
//! the folders and the open folders to the step.

use crate::action::BindingId;
use crate::profile::{Folder, FolderId, Page};
use opencraft_core::SurfaceId;

/// The largest number of steps in each list. The oldest step goes away
/// first. A step keeps a copy of all pages, which is a few kilobytes for a
/// typical profile: see decision 0030.
pub const HISTORY_LIMIT: usize = 100;

/// What a change of a text field changes. In one text edit, a change with
/// the same target as the last step joins that step, so that one edit of a
/// field is one step and not one step for each character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    /// The label of a surface on a page.
    Label { page: usize, surface: SurfaceId },
    /// The name of a page.
    PageName { page: usize },
    /// The name of a folder.
    FolderName { folder: FolderId },
    /// The action of a binding, for example a text setting of it.
    Binding(BindingId),
}

/// One step: its name for the user, and the profile at one side of it.
#[derive(Debug, Clone)]
struct Step {
    name: String,
    /// In the undo list: the pages before the change. In the redo list:
    /// the pages after the change.
    pages: Vec<Page>,
    /// The folders at the same side as `pages`.
    folders: Vec<Folder>,
    /// The active page before the change. An undo shows this page.
    undo_page: usize,
    /// The active page after the change. A redo shows this page.
    redo_page: usize,
    /// The open folders before the change.
    undo_open: Vec<FolderId>,
    /// The open folders after the change.
    redo_open: Vec<FolderId>,
}

/// A copy of the pages and the folders of a profile.
#[derive(Debug, Clone)]
pub(crate) struct SavedProfile {
    pub pages: Vec<Page>,
    pub folders: Vec<Folder>,
}

/// The active page and the open folders.
#[derive(Debug, Clone)]
pub(crate) struct Location {
    pub page: usize,
    pub open: Vec<FolderId>,
}

/// The two lists of steps, and the state of a text edit.
#[derive(Debug, Default)]
pub(crate) struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// True from [`Self::start_text_edit`] to [`Self::end_text_edit`].
    text_edit: bool,
    /// The target of the last step, if a change can join it: the step came
    /// in the open text edit, and nothing came after it.
    open: Option<Target>,
}

impl History {
    /// The name of the step that an undo undoes, and of the step that a
    /// redo does again.
    pub fn names(&self) -> (Option<String>, Option<String>) {
        let name = |list: &[Step]| list.last().map(|s| s.name.clone());
        (name(&self.undo), name(&self.redo))
    }

    /// True if a change of `target` joins the last step.
    pub fn joins(&self, target: Option<Target>) -> bool {
        self.text_edit && target.is_some() && self.open == target
    }

    /// A change joined the last step. The pages are now `pages` and the
    /// folders are now `folders`. The redo of the step shows `active` and
    /// `open_folders`. If the step changes nothing now, for example a letter
    /// that the user typed and removed, the step goes away.
    pub fn joined(
        &mut self,
        pages: &[Page],
        folders: &[Folder],
        active: usize,
        open_folders: &[FolderId],
    ) {
        let Some(last) = self.undo.last_mut() else {
            return;
        };
        last.redo_page = active;
        last.redo_open = open_folders.to_vec();
        if last.pages == pages && last.folders == folders {
            self.undo.pop();
            self.open = None;
        }
    }

    /// Add a step after a change. `before` is the profile before the
    /// change. `undo_place` is the location before it, and `redo_place`
    /// is the location after it. The redo list becomes empty.
    pub fn push(
        &mut self,
        name: String,
        target: Option<Target>,
        before: SavedProfile,
        undo_place: Location,
        redo_place: Location,
    ) {
        self.redo.clear();
        self.undo.push(Step {
            name,
            pages: before.pages,
            folders: before.folders,
            undo_page: undo_place.page,
            redo_page: redo_place.page,
            undo_open: undo_place.open,
            redo_open: redo_place.open,
        });
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.open = if self.text_edit { target } else { None };
    }

    /// Undo the last step. `swap` puts the pages, the folders, and the
    /// location of the step into the profile, and returns the pages and the
    /// folders that it replaced. Return false if the undo list is empty.
    pub fn undo(&mut self, swap: impl FnOnce(SavedProfile, Location) -> SavedProfile) -> bool {
        self.open = None;
        let Some(mut step) = self.undo.pop() else {
            return false;
        };
        let saved = swap(
            SavedProfile {
                pages: step.pages,
                folders: step.folders,
            },
            Location {
                page: step.undo_page,
                open: step.undo_open.clone(),
            },
        );
        step.pages = saved.pages;
        step.folders = saved.folders;
        self.redo.push(step);
        true
    }

    /// Do the last undone step again. See [`Self::undo`].
    pub fn redo(&mut self, swap: impl FnOnce(SavedProfile, Location) -> SavedProfile) -> bool {
        self.open = None;
        let Some(mut step) = self.redo.pop() else {
            return false;
        };
        let saved = swap(
            SavedProfile {
                pages: step.pages,
                folders: step.folders,
            },
            Location {
                page: step.redo_page,
                open: step.redo_open.clone(),
            },
        );
        step.pages = saved.pages;
        step.folders = saved.folders;
        self.undo.push(step);
        true
    }

    /// A text field of the shell got the focus. The next change starts a
    /// new step.
    pub fn start_text_edit(&mut self) {
        self.text_edit = true;
        self.open = None;
    }

    /// The text field lost the focus, or the user pressed Return. The next
    /// change starts a new step.
    pub fn end_text_edit(&mut self) {
        self.text_edit = false;
        self.open = None;
    }

    /// Remove all steps, for example when a different profile becomes the
    /// active profile. The state of the text edit stays.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.open = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages(names: &[&str]) -> Vec<Page> {
        names.iter().map(|n| Page::new(*n)).collect()
    }

    /// A fake profile: its pages, its folders, and its location.
    struct Fake {
        pages: Vec<Page>,
        folders: Vec<Folder>,
        active: usize,
        open: Vec<FolderId>,
    }

    impl Fake {
        fn new(pages: Vec<Page>, active: usize) -> Self {
            Self {
                pages,
                folders: Vec::new(),
                active,
                open: Vec::new(),
            }
        }

        fn swap(&mut self) -> impl FnOnce(SavedProfile, Location) -> SavedProfile + '_ {
            |saved, place| {
                self.active = place.page;
                self.open = place.open;
                SavedProfile {
                    pages: std::mem::replace(&mut self.pages, saved.pages),
                    folders: std::mem::replace(&mut self.folders, saved.folders),
                }
            }
        }
    }

    fn push(
        history: &mut History,
        name: &str,
        target: Option<Target>,
        before: Vec<Page>,
        undo: usize,
        redo: usize,
    ) {
        history.push(
            name.into(),
            target,
            SavedProfile {
                pages: before,
                folders: Vec::new(),
            },
            Location {
                page: undo,
                open: Vec::new(),
            },
            Location {
                page: redo,
                open: Vec::new(),
            },
        );
    }

    const NAME: Target = Target::PageName { page: 0 };

    #[test]
    fn undo_and_redo_swap_the_pages_and_the_names_move_between_the_lists() {
        let mut history = History::default();
        let mut fake = Fake::new(pages(&["B"]), 0);
        push(&mut history, "Rename Page", None, pages(&["A"]), 0, 0);
        assert_eq!(history.names(), (Some("Rename Page".into()), None));

        assert!(history.undo(fake.swap()));
        assert_eq!(fake.pages, pages(&["A"]));
        assert_eq!(history.names(), (None, Some("Rename Page".into())));
        assert!(!history.undo(fake.swap()), "the undo list is empty");

        assert!(history.redo(fake.swap()));
        assert_eq!(fake.pages, pages(&["B"]));
        assert_eq!(history.names(), (Some("Rename Page".into()), None));
        assert!(!history.redo(fake.swap()), "the redo list is empty");
    }

    #[test]
    fn undo_shows_the_page_before_and_redo_the_page_after() {
        let mut history = History::default();
        let mut fake = Fake::new(pages(&["A", "B"]), 1);
        push(&mut history, "Add Page", None, pages(&["A"]), 0, 1);
        history.undo(fake.swap());
        assert_eq!(fake.active, 0);
        history.redo(fake.swap());
        assert_eq!(fake.active, 1);
    }

    #[test]
    fn a_new_step_empties_the_redo_list() {
        let mut history = History::default();
        let mut fake = Fake::new(pages(&["B"]), 0);
        push(&mut history, "One", None, pages(&["A"]), 0, 0);
        history.undo(fake.swap());
        push(&mut history, "Two", None, pages(&["A"]), 0, 0);
        assert_eq!(history.names(), (Some("Two".into()), None));
    }

    #[test]
    fn only_a_change_of_the_same_target_in_one_text_edit_joins_the_last_step() {
        let mut history = History::default();
        // No text edit: no change joins.
        push(&mut history, "Rename Page", Some(NAME), pages(&["A"]), 0, 0);
        assert!(!history.joins(Some(NAME)));

        history.start_text_edit();
        assert!(!history.joins(Some(NAME)), "the first change is a new step");
        push(&mut history, "Rename Page", Some(NAME), pages(&["B"]), 0, 0);
        assert!(history.joins(Some(NAME)));
        assert!(!history.joins(Some(Target::PageName { page: 1 })));
        assert!(!history.joins(None));

        // A step with a different target closes the step before it.
        push(&mut history, "Set Fill", None, pages(&["C"]), 0, 0);
        assert!(!history.joins(Some(NAME)));

        push(&mut history, "Rename Page", Some(NAME), pages(&["D"]), 0, 0);
        history.end_text_edit();
        assert!(!history.joins(Some(NAME)));
        history.start_text_edit();
        assert!(!history.joins(Some(NAME)), "a new text edit is a new step");
    }

    #[test]
    fn a_joined_step_with_no_change_goes_away() {
        let mut history = History::default();
        history.start_text_edit();
        push(&mut history, "Rename Page", Some(NAME), pages(&["A"]), 0, 0);
        history.joined(&pages(&["AB"]), &[], 0, &[]);
        assert_eq!(history.names().0.as_deref(), Some("Rename Page"));
        history.joined(&pages(&["A"]), &[], 0, &[]);
        assert_eq!(history.names(), (None, None));
        assert!(!history.joins(Some(NAME)));
    }

    #[test]
    fn the_undo_list_keeps_the_newest_steps_up_to_the_limit() {
        let mut history = History::default();
        for n in 0..HISTORY_LIMIT + 5 {
            push(
                &mut history,
                &format!("Step {n}"),
                None,
                pages(&["A"]),
                0,
                0,
            );
        }
        assert_eq!(history.undo.len(), HISTORY_LIMIT);
        assert_eq!(history.undo[0].name, "Step 5");
        let mut fake = Fake::new(pages(&["A"]), 0);
        let mut count = 0;
        while history.undo(fake.swap()) {
            count += 1;
        }
        assert_eq!(count, HISTORY_LIMIT);
    }

    #[test]
    fn clear_removes_all_steps() {
        let mut history = History::default();
        let mut fake = Fake::new(pages(&["B"]), 0);
        push(&mut history, "One", None, pages(&["A"]), 0, 0);
        push(&mut history, "Two", None, pages(&["A"]), 0, 0);
        history.undo(fake.swap());
        history.clear();
        assert_eq!(history.names(), (None, None));
    }
}
