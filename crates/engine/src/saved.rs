//! The saved document: a value that the engine keeps in memory and writes to
//! a file. The active profile and the device settings each use it.
//!
//! The module owns these rules:
//!
//! - **Edit marks the change.** The value can be read through `Deref`. It can
//!   be changed only through [`SavedDocument::edit`] or
//!   [`SavedDocument::edit_if`]. Thus no edit can forget the mark, and the
//!   caller keeps no time of change. The engine does not find a change with a
//!   compare of the whole value.
//! - **Delay.** The write waits until no edit came for [`SAVE_DELAY`]. The
//!   caller gives the time only to the calls that write
//!   ([`SavedDocument::save_if_due`] and [`SavedDocument::flush`]). The module
//!   keeps the time of the last such call, and an edit counts from that time.
//!   The engine calls `save_if_due` at the end of each poll, thus the time of
//!   an edit is early by less than one poll.
//! - **Retry.** A failed write stays pending. The module tries again after
//!   the delay.
//! - **One problem message.** [`SavedDocument::save_if_due`] and
//!   [`SavedDocument::flush`] give the error of a failed write one time, until
//!   a write is successful. The engine tells the shell, and the shell does not
//!   get the same problem for each try.
//! - **Refusal.** A document that is not writable is never written. The
//!   engine marks a document as not writable when the file cannot be read,
//!   because the write would replace the file (decision 0009).
//! - **Save at stop.** [`SavedDocument::flush`] writes the pending change at
//!   once (decision 0022). [`SavedDocument::save_before_leaving`] also writes
//!   a document that has no file yet.
//!
//! The module does no I/O itself. A [`SavedFile`] does the I/O.

use std::ops::Deref;
use std::time::{Duration, Instant};

use crate::Result;

/// The time after the last edit before the engine writes the file.
pub const SAVE_DELAY: Duration = Duration::from_millis(500);

/// The file of a saved document.
pub trait SavedFile<T> {
    /// True if the file exists.
    fn exists(&self) -> bool;

    /// Write `value` to the file.
    fn write(&self, value: &T) -> Result<()>;
}

/// What is not in the file yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pending {
    /// The file has the value.
    Nothing,
    /// The edit was seen at this time. The write is due after
    /// [`SAVE_DELAY`].
    Since(Instant),
}

/// A value with its rules for the file. See the module description.
#[derive(Debug)]
pub struct SavedDocument<T> {
    value: T,
    /// False if the engine must not write the file.
    writable: bool,
    pending: Pending,
    /// The time of the last call that wrote or could write. Before the first
    /// call, the time of the start.
    last_call: Instant,
    /// True after a failed write, until a write is successful. Then the
    /// engine told the shell about the problem.
    failed: bool,
}

impl<T> Deref for SavedDocument<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> SavedDocument<T> {
    /// A document that has no change. `writable` is false if the engine must
    /// not write the file. `now` is the time of the start: an edit before the
    /// first write call counts from it.
    pub fn new(value: T, writable: bool, now: Instant) -> Self {
        Self {
            value,
            writable,
            pending: Pending::Nothing,
            last_call: now,
            failed: false,
        }
    }

    /// Put in a value that came from the file, or a value that the engine
    /// uses instead of the file. The document has no change after this.
    pub fn load(&mut self, value: T, writable: bool) {
        let last_call = self.last_call;
        *self = Self::new(value, writable, last_call);
        self.last_call = last_call;
    }

    /// Change the value, and mark the document as changed.
    pub fn edit(&mut self) -> &mut T {
        self.mark();
        &mut self.value
    }

    fn mark(&mut self) {
        self.pending = Pending::Since(self.last_call);
    }

    /// Change the value with `change`. The document is marked as changed
    /// only if `change` returns true. Return what `change` returns.
    pub fn edit_if(&mut self, change: impl FnOnce(&mut T) -> bool) -> bool {
        let changed = change(&mut self.value);
        if changed {
            self.mark();
        }
        changed
    }

    /// True if the engine may write the file.
    pub fn is_writable(&self) -> bool {
        self.writable
    }

    /// Let the engine write the file again. The caller does this when the
    /// user changes the value on purpose, after a file that the engine could
    /// not read.
    pub fn allow_writes(&mut self) {
        self.writable = true;
    }

    /// True if a change is not in the file.
    pub fn is_changed(&self) -> bool {
        self.pending != Pending::Nothing
    }

    /// True if the document has no change, and the engine may write, and the
    /// file is absent: the value is only in memory, for example a profile
    /// that the engine made.
    pub fn is_unwritten(&self, file: &impl SavedFile<T>) -> bool {
        self.writable && !self.is_changed() && !file.exists()
    }

    /// Write the file if the last edit is older than [`SAVE_DELAY`]. `now` is
    /// the time of the caller. Return the error of a failed write, but only
    /// the first one of a series of failures.
    pub fn save_if_due(&mut self, now: Instant, file: &impl SavedFile<T>) -> Option<crate::Error> {
        self.last_call = now;
        let Pending::Since(since) = self.pending else {
            return None;
        };
        if now.saturating_duration_since(since) < SAVE_DELAY {
            return None;
        }
        self.flush(now, file)
    }

    /// Write the file now if a change is not in it. Use this at the stop
    /// (decision 0022). Return the error of a failed write, but only the
    /// first one of a series of failures. A failed write stays pending, and
    /// [`Self::save_if_due`] tries again after [`SAVE_DELAY`].
    pub fn flush(&mut self, now: Instant, file: &impl SavedFile<T>) -> Option<crate::Error> {
        self.last_call = now;
        if !self.is_changed() {
            return None;
        }
        if !self.writable {
            self.pending = Pending::Nothing;
            return None;
        }
        match file.write(&self.value) {
            Ok(()) => {
                self.pending = Pending::Nothing;
                self.failed = false;
                None
            }
            Err(e) => {
                self.pending = Pending::Since(now);
                if self.failed {
                    None
                } else {
                    self.failed = true;
                    Some(e)
                }
            }
        }
    }

    /// Write the file before the engine leaves the document: if a change is
    /// not in the file, or if the file is absent. Return the error of a
    /// failed write every time, because the caller stays on the document.
    pub fn save_before_leaving(&mut self, file: &impl SavedFile<T>) -> Result<()> {
        if !self.writable || (!self.is_changed() && file.exists()) {
            return Ok(());
        }
        file.write(&self.value)?;
        self.pending = Pending::Nothing;
        self.failed = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;
    use std::cell::{Cell, RefCell};
    use std::fs;
    use std::io;
    use std::path::PathBuf;

    /// A file in a temporary folder. `fail` makes each write fail.
    struct TextFile {
        path: PathBuf,
        fail: Cell<bool>,
        writes: Cell<u32>,
    }

    impl TextFile {
        fn new(dir: &tempfile::TempDir) -> Self {
            Self {
                path: dir.path().join("document.txt"),
                fail: Cell::new(false),
                writes: Cell::new(0),
            }
        }

        fn text(&self) -> Option<String> {
            fs::read_to_string(&self.path).ok()
        }
    }

    impl SavedFile<String> for TextFile {
        fn exists(&self) -> bool {
            self.path.exists()
        }

        fn write(&self, value: &String) -> Result<()> {
            if self.fail.get() {
                return Err(Error::Io {
                    action: "cannot write",
                    path: self.path.clone(),
                    source: io::Error::other("the disk is full"),
                });
            }
            self.writes.set(self.writes.get() + 1);
            fs::write(&self.path, value).map_err(|source| Error::Io {
                action: "cannot write",
                path: self.path.clone(),
                source,
            })
        }
    }

    /// The test clock: the caller moves it.
    struct Time {
        start: Instant,
        now: RefCell<Duration>,
    }

    impl Time {
        fn new() -> Self {
            Self {
                start: Instant::now(),
                now: RefCell::new(Duration::ZERO),
            }
        }

        fn now(&self) -> Instant {
            self.start + *self.now.borrow()
        }

        fn pass(&self, time: Duration) {
            *self.now.borrow_mut() += time;
        }
    }

    fn document(time: &Time, writable: bool) -> SavedDocument<String> {
        SavedDocument::new("start".to_string(), writable, time.now())
    }

    #[test]
    fn a_new_document_has_no_change_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        let time = Time::new();
        let mut doc = document(&time, true);
        assert!(!doc.is_changed());
        time.pass(SAVE_DELAY * 4);
        assert!(doc.save_if_due(time.now(), &file).is_none());
        assert_eq!(file.writes.get(), 0);
    }

    #[test]
    fn an_edit_is_written_after_the_delay_and_not_before() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        let time = Time::new();
        let mut doc = document(&time, true);
        doc.edit().push_str(" one");
        assert!(doc.is_changed());
        assert!(doc.save_if_due(time.now(), &file).is_none());
        time.pass(SAVE_DELAY - Duration::from_millis(1));
        assert!(doc.save_if_due(time.now(), &file).is_none());
        assert_eq!(file.text(), None);
        time.pass(Duration::from_millis(1));
        assert!(doc.save_if_due(time.now(), &file).is_none());
        assert_eq!(file.text().as_deref(), Some("start one"));
        assert!(!doc.is_changed());
    }

    #[test]
    fn each_edit_makes_the_delay_start_again() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        let time = Time::new();
        let mut doc = document(&time, true);
        for _ in 0..10 {
            doc.edit().push('x');
            doc.save_if_due(time.now(), &file);
            time.pass(SAVE_DELAY / 5);
            doc.save_if_due(time.now(), &file);
        }
        assert_eq!(file.writes.get(), 0, "no write during the edits");
        time.pass(SAVE_DELAY);
        doc.save_if_due(time.now(), &file);
        assert_eq!(file.writes.get(), 1, "one write for all edits");
        assert_eq!(file.text().as_deref(), Some("startxxxxxxxxxx"));
    }

    #[test]
    fn an_edit_counts_from_the_last_call_that_knew_the_time() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        let time = Time::new();
        let mut doc = document(&time, true);
        doc.save_if_due(time.now(), &file);
        // The edit comes between two calls. The caller gives no time.
        time.pass(Duration::from_millis(10));
        doc.edit().push_str(" one");
        time.pass(SAVE_DELAY - Duration::from_millis(10));
        doc.save_if_due(time.now(), &file);
        assert_eq!(file.writes.get(), 1, "the delay counted from the last call");
    }

    #[test]
    fn edit_if_marks_the_change_only_when_the_closure_says_so() {
        let time = Time::new();
        let mut doc = document(&time, true);
        assert!(!doc.edit_if(|_| false));
        assert!(!doc.is_changed());
        assert!(doc.edit_if(|value| {
            value.push('!');
            true
        }));
        assert!(doc.is_changed());
        assert_eq!(&*doc, "start!");
    }

    #[test]
    fn a_failed_write_is_tried_again_and_the_problem_comes_one_time() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        file.fail.set(true);
        let time = Time::new();
        let mut doc = document(&time, true);
        doc.edit().push_str(" one");
        doc.save_if_due(time.now(), &file);
        time.pass(SAVE_DELAY);
        let first = doc.save_if_due(time.now(), &file);
        assert!(first.is_some(), "the first failure gives the error");
        assert!(doc.is_changed(), "the change stays pending");
        // The retry waits for the delay.
        time.pass(SAVE_DELAY - Duration::from_millis(1));
        assert!(doc.save_if_due(time.now(), &file).is_none());
        time.pass(Duration::from_millis(1));
        assert!(
            doc.save_if_due(time.now(), &file).is_none(),
            "the second failure gives no error"
        );
        assert!(doc.is_changed());
        // The disk works again.
        file.fail.set(false);
        time.pass(SAVE_DELAY);
        assert!(doc.save_if_due(time.now(), &file).is_none());
        assert_eq!(file.text().as_deref(), Some("start one"));
        assert!(!doc.is_changed());
        // A new series of failures gives the error again.
        file.fail.set(true);
        doc.edit().push('!');
        doc.save_if_due(time.now(), &file);
        time.pass(SAVE_DELAY);
        assert!(doc.save_if_due(time.now(), &file).is_some());
    }

    #[test]
    fn a_document_that_is_not_writable_never_writes() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        fs::write(&file.path, "the old file").unwrap();
        let time = Time::new();
        let mut doc = document(&time, false);
        doc.edit().push_str(" one");
        doc.save_if_due(time.now(), &file);
        time.pass(SAVE_DELAY);
        assert!(doc.save_if_due(time.now(), &file).is_none());
        assert!(doc.flush(time.now(), &file).is_none());
        assert!(doc.save_before_leaving(&file).is_ok());
        assert_eq!(file.text().as_deref(), Some("the old file"));
        assert_eq!(file.writes.get(), 0);
        assert!(!doc.is_changed(), "the refused change is let go");
    }

    #[test]
    fn allow_writes_lets_the_next_edit_write_over_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        fs::write(&file.path, "the old file").unwrap();
        let time = Time::new();
        let mut doc = document(&time, false);
        doc.allow_writes();
        doc.edit().push_str(" one");
        assert!(doc.flush(time.now(), &file).is_none());
        assert_eq!(file.text().as_deref(), Some("start one"));
    }

    #[test]
    fn flush_writes_the_pending_change_at_once_and_only_a_change() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        let time = Time::new();
        let mut doc = document(&time, true);
        assert!(doc.flush(time.now(), &file).is_none());
        assert_eq!(file.writes.get(), 0, "no change, no write at the stop");
        doc.edit().push_str(" one");
        assert!(doc.flush(time.now(), &file).is_none());
        assert_eq!(file.text().as_deref(), Some("start one"));
        assert!(doc.flush(time.now(), &file).is_none());
        assert_eq!(file.writes.get(), 1);
    }

    #[test]
    fn flush_gives_the_error_of_the_first_failure_only() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        file.fail.set(true);
        let time = Time::new();
        let mut doc = document(&time, true);
        doc.edit().push('x');
        assert!(doc.flush(time.now(), &file).is_some());
        assert!(doc.flush(time.now(), &file).is_none());
        assert!(doc.is_changed());
    }

    #[test]
    fn the_file_is_written_when_it_is_absent() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        let time = Time::new();
        let mut doc = document(&time, true);
        assert!(doc.is_unwritten(&file));
        assert!(doc.save_before_leaving(&file).is_ok());
        assert_eq!(file.text().as_deref(), Some("start"));
        assert!(!doc.is_unwritten(&file));
        // With the file and no change, leaving writes nothing.
        assert!(doc.save_before_leaving(&file).is_ok());
        assert_eq!(file.writes.get(), 1);
        // A document that is not writable is not unwritten.
        let time = Time::new();
        assert!(!document(&time, false).is_unwritten(&TextFile::new(&dir)));
    }

    #[test]
    fn save_before_leaving_writes_a_change_and_reports_each_failure() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        let time = Time::new();
        let mut doc = document(&time, true);
        doc.save_before_leaving(&file).unwrap();
        doc.edit().push_str(" one");
        file.fail.set(true);
        assert!(doc.save_before_leaving(&file).is_err());
        assert!(doc.save_before_leaving(&file).is_err());
        assert!(doc.is_changed());
        file.fail.set(false);
        assert!(doc.save_before_leaving(&file).is_ok());
        assert_eq!(file.text().as_deref(), Some("start one"));
    }

    #[test]
    fn load_puts_in_a_value_with_no_change() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        file.fail.set(true);
        let time = Time::new();
        let mut doc = document(&time, true);
        doc.edit().push('x');
        assert!(doc.flush(time.now(), &file).is_some());
        doc.load("other".to_string(), true);
        assert!(!doc.is_changed());
        assert_eq!(&*doc, "other");
        // The series of failures ended: a new failure gives the error.
        doc.edit().push('y');
        assert!(doc.flush(time.now(), &file).is_some());
    }

    #[test]
    fn a_save_at_the_stop_works_for_a_change_that_no_tick_saw() {
        let dir = tempfile::tempdir().unwrap();
        let file = TextFile::new(&dir);
        let time = Time::new();
        let mut doc = document(&time, true);
        doc.edit().push_str(" late");
        // No call of `save_if_due` came. The stop still writes.
        assert!(doc.flush(time.now(), &file).is_none());
        assert_eq!(file.text().as_deref(), Some("start late"));
    }
}
