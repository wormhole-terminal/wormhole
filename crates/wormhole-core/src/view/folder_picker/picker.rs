//! The folder picker as data: what is typed, the folder that text points at, what was read from it and
//! which row is selected. The window reads a folder (off the UI thread) and hands the answer back; nothing
//! here touches the disk.

use std::io;
use std::path::{Path, PathBuf};

use super::collate::collate;
use super::insert::inserted;
use super::rows::{Row, rows};
use super::target::{Target, resolve};
use crate::storage::files::Folders;

/// What a read of the target folder came to.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Listing {
    /// The folder it is the listing of.
    folder: PathBuf,
    /// Its subfolders, in the order `ls` gives.
    names: Vec<String>,
    /// There are more than were read (the read's caps).
    cut: bool,
    /// The folder could not be read: missing, not a folder, or refused.
    unreadable: bool,
}

/// The picker's state.
#[derive(Clone, Debug)]
pub struct Picker {
    cwd: PathBuf,
    home: PathBuf,
    typed: String,
    target: Target,
    listing: Option<Listing>,
    rows: Vec<Row>,
    selected: usize,
}

impl Picker {
    /// A picker with nothing typed, over `cwd`'s subfolders. `home` is what a typed `~/` means.
    pub fn new(cwd: PathBuf, home: PathBuf) -> Self {
        let target = resolve("", &cwd, &home);
        Self { cwd, home, typed: String::new(), target, listing: None, rows: Vec::new(), selected: 0 }
    }

    /// What is typed in the field.
    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// The folder being listed and what narrows it.
    pub fn target(&self) -> &Target {
        &self.target
    }

    /// The text changed. The selection goes back to the first row. When the folder it points at changed, the
    /// rows are gone until [`set_listing`](Self::set_listing) has the new folder's: ask [`wants`](Self::wants).
    pub fn set_typed(&mut self, typed: &str) {
        if typed == self.typed {
            return;
        }
        self.typed = typed.to_string();
        let target = resolve(typed, &self.cwd, &self.home);
        if target.folder != self.target.folder {
            self.listing = None;
        }
        self.target = target;
        self.refilter();
    }

    /// The folder to read, until its listing is in.
    pub fn wants(&self) -> Option<&Path> {
        self.listing.is_none().then_some(self.target.folder.as_path())
    }

    /// The answer to a read of `folder`. One for a folder that is no longer the target (the text moved on
    /// while it was being read) is dropped.
    pub fn set_listing(&mut self, folder: &Path, read: io::Result<Folders>) {
        if folder != self.target.folder {
            return;
        }
        let (mut names, cut, unreadable) = match read {
            Ok(found) => (found.names, found.cut, false),
            Err(_) => (Vec::new(), false, true),
        };
        names.sort_by(|a, b| collate(a, b));
        self.listing = Some(Listing { folder: folder.to_path_buf(), names, cut, unreadable });
        self.refilter();
    }

    fn refilter(&mut self) {
        self.rows = self.listing.as_ref().map(|l| rows(&l.names, &self.target.filter)).unwrap_or_default();
        self.selected = 0;
    }

    /// Whether the target folder's listing is in: before it, an empty list means "reading", not "no folders".
    pub fn loaded(&self) -> bool {
        self.listing.is_some()
    }

    /// The folder could not be read.
    pub fn unreadable(&self) -> bool {
        self.listing.as_ref().is_some_and(|l| l.unreadable)
    }

    /// The folder has more subfolders than were read.
    pub fn cut(&self) -> bool {
        self.listing.as_ref().is_some_and(|l| l.cut)
    }

    /// The rows, top to bottom.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The selected row's place in [`rows`](Self::rows); none with no rows.
    pub fn selected(&self) -> Option<usize> {
        (!self.rows.is_empty()).then_some(self.selected)
    }

    /// Move the selection `by` rows, wrapping round the ends.
    pub fn step(&mut self, by: isize) {
        let n = self.rows.len() as isize;
        if n > 0 {
            self.selected = (self.selected as isize + by).rem_euclid(n) as usize;
        }
    }

    /// Select row `ix`, if there is one.
    pub fn select(&mut self, ix: usize) {
        if ix < self.rows.len() {
            self.selected = ix;
        }
    }

    fn selected_name(&self) -> Option<&str> {
        self.rows.get(self.selected).map(|row| row.name.as_str())
    }

    /// What Tab makes of the text: the selected folder written out in full with its `/`, so the list is now
    /// that folder's. None with nothing selected.
    pub fn completion(&self) -> Option<String> {
        Some(format!("{}{}/", self.target.prefix, self.selected_name()?))
    }

    /// What ↩ puts at the prompt: the selected folder's path in the form typed, escaped for the shell, ending
    /// in `/`. None with nothing selected.
    pub fn choice(&self) -> Option<String> {
        Some(inserted(&self.target.prefix, self.selected_name()?))
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::{Path, PathBuf};

    use super::Picker;
    use crate::storage::files::Folders;

    fn folders(names: &[&str]) -> io::Result<Folders> {
        Ok(Folders { names: names.iter().map(ToString::to_string).collect(), cut: false })
    }

    fn picker() -> Picker {
        let mut picker = Picker::new(PathBuf::from("/work/app"), PathBuf::from("/Users/me"));
        picker.set_listing(Path::new("/work/app"), folders(&["src", ".git", "My Docs", "docs", "Zed"]));
        picker
    }

    fn names(picker: &Picker) -> Vec<&str> {
        picker.rows().iter().map(|r| r.name.as_str()).collect()
    }

    #[test]
    fn it_starts_empty_over_the_current_folder_and_asks_for_it() {
        let picker = Picker::new(PathBuf::from("/work/app"), PathBuf::from("/Users/me"));
        assert_eq!(picker.wants(), Some(Path::new("/work/app")));
        assert!(!picker.loaded());
        assert!(picker.rows().is_empty());
        assert_eq!(picker.selected(), None);
    }

    #[test]
    fn the_rows_are_in_ls_order_without_dot_folders() {
        let picker = picker();
        assert_eq!(picker.wants(), None);
        assert_eq!(names(&picker), ["docs", "My Docs", "src", "Zed"]);
        assert_eq!(picker.selected(), Some(0));
    }

    #[test]
    fn typing_narrows_the_rows_and_selects_the_first() {
        let mut picker = picker();
        picker.step(2);
        picker.set_typed("do");
        assert_eq!(names(&picker), ["docs", "My Docs"]);
        assert_eq!(picker.selected(), Some(0));
        assert_eq!(picker.wants(), None, "the same folder needs no new read");
    }

    #[test]
    fn walking_into_another_folder_asks_for_it_and_shows_nothing_until_it_comes() {
        let mut picker = picker();
        picker.set_typed("src/");
        assert_eq!(picker.wants(), Some(Path::new("/work/app/src/")));
        assert!(picker.rows().is_empty() && !picker.loaded());
        picker.set_listing(Path::new("/work/app/src/"), folders(&["ui", "core"]));
        assert_eq!(names(&picker), ["core", "ui"]);
        assert_eq!(picker.choice().as_deref(), Some("src/core/"));
    }

    #[test]
    fn an_answer_for_a_folder_the_text_has_left_is_dropped() {
        let mut picker = picker();
        picker.set_typed("src/");
        picker.set_listing(Path::new("/work/app"), folders(&["stale"]));
        assert!(!picker.loaded());
        picker.set_typed("../");
        picker.set_listing(Path::new("/work/app/src/"), folders(&["stale"]));
        assert!(!picker.loaded());
    }

    #[test]
    fn the_selection_wraps() {
        let mut picker = picker();
        picker.step(-1);
        assert_eq!(picker.selected(), Some(3));
        picker.step(1);
        assert_eq!(picker.selected(), Some(0));
        picker.step(5);
        assert_eq!(picker.selected(), Some(1));
        picker.select(9);
        assert_eq!(picker.selected(), Some(1), "no such row");
    }

    #[test]
    fn tab_completes_the_selected_folder_and_return_inserts_it_escaped() {
        let mut picker = picker();
        picker.step(1);
        assert_eq!(picker.completion().as_deref(), Some("My Docs/"));
        assert_eq!(picker.choice().as_deref(), Some(r"My\ Docs/"));
    }

    #[test]
    fn the_form_typed_is_the_form_inserted() {
        let mut picker = picker();
        picker.set_typed("~/");
        picker.set_listing(Path::new("/Users/me/"), folders(&["Projects", "Desktop"]));
        assert_eq!(picker.choice().as_deref(), Some("~/Desktop/"));
        picker.set_typed("/usr/");
        picker.set_listing(Path::new("/usr/"), folders(&["bin"]));
        assert_eq!(picker.choice().as_deref(), Some("/usr/bin/"));
        picker.set_typed("../");
        picker.set_listing(Path::new("/work/app/../"), folders(&["other"]));
        assert_eq!(picker.choice().as_deref(), Some("../other/"));
    }

    #[test]
    fn a_folder_that_cant_be_read_has_no_rows_and_says_so() {
        let mut picker = picker();
        picker.set_typed("nope/");
        picker.set_listing(Path::new("/work/app/nope/"), Err(io::ErrorKind::NotFound.into()));
        assert!(picker.loaded() && picker.unreadable());
        assert!(picker.rows().is_empty());
        assert_eq!(picker.choice(), None);
        assert_eq!(picker.completion(), None);
    }

    #[test]
    fn it_reads_what_is_on_the_disk() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["beta", "Alpha", "with space", ".hidden"] {
            std::fs::create_dir(dir.path().join(name)).unwrap();
        }
        std::fs::write(dir.path().join("file.txt"), "x").unwrap();
        let mut picker = Picker::new(dir.path().to_path_buf(), PathBuf::from("/Users/me"));
        let folder = picker.wants().unwrap().to_path_buf();
        picker.set_listing(&folder, crate::storage::files::list_folders(&folder, 100));
        assert_eq!(names(&picker), ["Alpha", "beta", "with space"]);
        picker.set_typed(".");
        assert_eq!(names(&picker), [".hidden"]);
    }
}
