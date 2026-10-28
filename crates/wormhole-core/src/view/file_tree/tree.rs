//! The tree of folders a Files tab shows: which folders were read and what they held, which are open,
//! and the rows that makes. Paths are relative to the tree's root, and the root itself is the empty
//! path. A folder that is read again replaces what the tree knew of it; an open folder that is gone from
//! its parent is dropped with everything under it.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::dir::{Dir, Entry};
use crate::storage::files::listing::EntryKind;

/// Why a folder could not be listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The folder is gone, or is not a folder (any more).
    Gone,
    /// It is there and may not be read.
    Denied,
    /// Anything else the disk said.
    Other,
}

impl Failure {
    /// What an I/O error means for the tab.
    pub fn of(error: &std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory => Self::Gone,
            std::io::ErrorKind::PermissionDenied => Self::Denied,
            _ => Self::Other,
        }
    }
}

/// What the tree knows of a folder it has an answer for; a folder without one is not read yet.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Slot {
    Ready(Dir),
    Failed(Failure),
}

/// The folders read and the folders open.
#[derive(Debug, Default)]
pub struct Tree {
    slots: HashMap<PathBuf, Slot>,
    open: HashSet<PathBuf>,
}

/// What a row is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowKind {
    Folder {
        open: bool,
    },
    File,
    /// A link; it never opens, and what it points at is never looked at.
    Link,
    /// "N more": entries the folder has that are not listed; `at_least` when that is only what was seen.
    More {
        count: usize,
        at_least: bool,
    },
    /// An open folder not answered yet.
    Reading,
    /// An open folder with nothing in it.
    Empty,
    /// An open folder that could not be listed.
    Unreadable(Failure),
}

/// One line of the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The entry's path below the root; for a note, the path of the folder it is about.
    pub path: PathBuf,
    /// How many folders are above it.
    pub depth: usize,
    /// The name as drawn; empty for a note.
    pub label: String,
    pub kind: RowKind,
}

impl Row {
    /// Whether the row stands for something on the disk (the keys and the menu act on it).
    pub fn is_entry(&self) -> bool {
        matches!(self.kind, RowKind::Folder { .. } | RowKind::File | RowKind::Link)
    }

    pub fn is_folder(&self) -> bool {
        matches!(self.kind, RowKind::Folder { .. })
    }
}

impl Tree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets everything: the root changed.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.open.clear();
    }

    /// Whether `path` (a folder below the root) is open.
    pub fn is_open(&self, path: &Path) -> bool {
        self.open.contains(path)
    }

    /// The entry `path` stands for, if the tree has read its folder.
    pub fn entry(&self, path: &Path) -> Option<&super::dir::Entry> {
        let Some(Slot::Ready(dir)) = self.slots.get(path.parent()?) else { return None };
        dir.get(path.file_name()?)
    }

    /// Whether the folder `path` has been read (or failed to be), so its slot holds an answer.
    pub fn answered(&self, path: &Path) -> bool {
        self.slots.contains_key(path)
    }

    /// Opens a folder, or closes it. Whether the folder now needs reading (it was just opened and the
    /// tree has not read it). A path that is not a real folder of a folder already read (a file, a
    /// link, anything unknown) does nothing: this is where a link is never entered.
    pub fn toggle(&mut self, path: &Path) -> bool {
        if !self.entry(path).is_some_and(Entry::is_folder) {
            return false;
        }
        if self.open.remove(path) {
            return false;
        }
        self.open.insert(path.to_path_buf());
        !matches!(self.slots.get(path), Some(Slot::Ready(_)))
    }

    /// Opens `path` and every folder above it that is not open, for a caller that knows where it wants
    /// to be (devctl, a restored state). Folders not read yet are asked for by [`Self::wanted`].
    pub fn open_path(&mut self, path: &Path) {
        for ancestor in path.ancestors().filter(|a| !a.as_os_str().is_empty()) {
            self.open.insert(ancestor.to_path_buf());
        }
    }

    /// The entry `from` was renamed to `to`, in the same folder: the folder's entry takes the new name, and
    /// what the tree knew of the renamed folder (that it is open, what it held, the folders open in it)
    /// moves with it, so it does not close.
    pub fn renamed(&mut self, from: &Path, to: &Path) {
        let (Some(parent), Some(old), Some(new)) = (from.parent(), from.file_name(), to.file_name()) else {
            return;
        };
        if to.parent() != Some(parent) {
            return;
        }
        if let Some(Slot::Ready(dir)) = self.slots.get_mut(parent)
            && let Some(renamed) = dir.renamed(old, new)
        {
            *dir = renamed;
        }
        let moved = |path: &Path| path.strip_prefix(from).ok().map(|rest| to.join(rest));
        let slots = std::mem::take(&mut self.slots);
        self.slots = slots.into_iter().map(|(path, slot)| (moved(&path).unwrap_or(path), slot)).collect();
        let open = std::mem::take(&mut self.open);
        self.open = open.into_iter().map(|path| moved(&path).unwrap_or(path)).collect();
    }

    /// The folders to read now to keep what is shown true: the root and each open folder that shows,
    /// the ones above first. Run again on every refresh, it is also what finds an open folder that
    /// has not been read yet.
    pub fn wanted(&self) -> Vec<PathBuf> {
        let mut found = vec![PathBuf::new()];
        let mut at = 0;
        while let Some(path) = found.get(at).cloned() {
            at += 1;
            let Some(Slot::Ready(dir)) = self.slots.get(&path) else { continue };
            for entry in dir.entries.iter().filter(|entry| entry.is_folder()) {
                let child = path.join(&entry.name);
                if self.open.contains(&child) {
                    found.push(child);
                }
            }
        }
        found
    }

    /// The answer for `path`. Whether it changes what is shown: a folder read again that holds the same
    /// is no change, so a refresh that finds nothing new redraws nothing.
    pub fn set(&mut self, path: &Path, answer: Result<Dir, Failure>) -> bool {
        let slot = match answer {
            Ok(dir) => Slot::Ready(dir),
            Err(failure) => Slot::Failed(failure),
        };
        let shown = self.shows(path);
        let changed = self.slots.get(path) != Some(&slot);
        self.slots.insert(path.to_path_buf(), slot);
        let pruned = self.prune();
        (changed && shown) || pruned
    }

    /// Whether the folder's content is on screen: it is the root, or open under open folders.
    fn shows(&self, path: &Path) -> bool {
        path.ancestors().take_while(|a| !a.as_os_str().is_empty()).all(|a| self.open.contains(a))
    }

    /// Drops the open folders and answers whose entry is gone from a folder that was read. Whether
    /// anything went.
    fn prune(&mut self) -> bool {
        let stale = |tree: &Self, path: &Path| match path.parent().and_then(|parent| tree.slots.get(parent)) {
            Some(Slot::Ready(dir)) => {
                !path.file_name().and_then(|name| dir.get(name)).is_some_and(Entry::is_folder)
            }
            // The parent is unknown, failed or being read: nothing says the child is gone.
            _ => false,
        };
        let gone: Vec<PathBuf> = self
            .slots
            .keys()
            .chain(self.open.iter())
            .filter(|path| !path.as_os_str().is_empty() && stale(self, path))
            .cloned()
            .collect();
        // What was under a folder that went goes with it, whatever its own parent still says.
        let under_gone = |path: &Path| gone.iter().any(|gone| path.starts_with(gone));
        let before = self.slots.len() + self.open.len();
        self.slots.retain(|path, _| !under_gone(path));
        self.open.retain(|path| !under_gone(path));
        self.slots.len() + self.open.len() != before
    }

    /// The rows, top to bottom: each folder's entries, and under an open folder its own. A folder
    /// that is empty, not read yet or can't be read is a note about it, the root's too.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        self.push_folder(Path::new(""), 0, &mut rows);
        rows
    }

    fn push_folder(&self, folder: &Path, depth: usize, rows: &mut Vec<Row>) {
        let note = |kind| Row { path: folder.to_path_buf(), depth, label: String::new(), kind };
        match self.slots.get(folder) {
            None => rows.push(note(RowKind::Reading)),
            Some(Slot::Failed(failure)) => rows.push(note(RowKind::Unreadable(*failure))),
            Some(Slot::Ready(dir)) => {
                if dir.entries.is_empty() && dir.more == 0 {
                    rows.push(note(RowKind::Empty));
                }
                for entry in &dir.entries {
                    let path = folder.join(&entry.name);
                    let kind = match entry.kind {
                        EntryKind::Folder => RowKind::Folder { open: self.open.contains(&path) },
                        EntryKind::File => RowKind::File,
                        EntryKind::Link => RowKind::Link,
                    };
                    let open = matches!(kind, RowKind::Folder { open: true });
                    rows.push(Row { path: path.clone(), depth, label: entry.label.clone(), kind });
                    if open {
                        self.push_folder(&path, depth + 1, rows);
                    }
                }
                if dir.more > 0 {
                    rows.push(note(RowKind::More { count: dir.more, at_least: dir.at_least }));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use super::{Failure, RowKind, Tree};
    use crate::storage::files::listing::{DirEntry, EntryKind, Listing};
    use crate::view::file_tree::dir::{Dir, ENTRY_CAP};

    fn dir(entries: &[(&str, EntryKind)]) -> Dir {
        let entries =
            entries.iter().map(|&(name, kind)| DirEntry { name: OsString::from(name), kind }).collect();
        Dir::from_listing(Listing { entries, truncated: false }, ENTRY_CAP)
    }

    fn labels(tree: &Tree) -> Vec<String> {
        tree.rows().iter().map(|row| format!("{}{}", "  ".repeat(row.depth), row.label)).collect()
    }

    fn sample() -> Tree {
        use EntryKind::{File, Folder};
        let mut tree = Tree::new();
        tree.set(Path::new(""), Ok(dir(&[("src", Folder), ("docs", Folder), ("Cargo.toml", File)])));
        tree
    }

    #[test]
    fn the_root_lists_its_entries_and_nothing_below_a_closed_folder() {
        assert_eq!(labels(&sample()), ["docs", "src", "Cargo.toml"]);
    }

    #[test]
    fn opening_a_folder_asks_for_it_once_and_shows_it_when_it_answers() {
        use EntryKind::{File, Folder};
        let mut tree = sample();
        assert!(tree.toggle(Path::new("src")), "not read yet: it needs reading");
        // A line for the folder that is being read: no name, one level in.
        assert_eq!(labels(&tree), ["docs", "src", "  ", "Cargo.toml"]);
        assert!(tree.set(Path::new("src"), Ok(dir(&[("main.rs", File), ("bin", Folder)]))));
        assert_eq!(labels(&tree), ["docs", "src", "  bin", "  main.rs", "Cargo.toml"]);
        assert!(!tree.toggle(Path::new("src")), "closing needs no read");
        assert_eq!(labels(&tree), ["docs", "src", "Cargo.toml"]);
        assert!(!tree.toggle(Path::new("src")), "open again: already read");
    }

    #[test]
    fn a_link_and_a_file_never_open() {
        use EntryKind::{File, Link};
        let mut tree = Tree::new();
        tree.set(Path::new(""), Ok(dir(&[("out", Link), ("a", File)])));
        assert!(!tree.toggle(Path::new("out")));
        assert!(!tree.toggle(Path::new("a")));
        assert!(!tree.toggle(Path::new("unknown")));
        assert!(!tree.is_open(Path::new("out")));
        assert!(tree.wanted() == [PathBuf::new()], "only the root is read");
    }

    #[test]
    fn an_empty_or_unreadable_open_folder_says_so_and_a_cut_one_counts_the_rest() {
        use EntryKind::Folder;
        let mut tree = Tree::new();
        tree.set(Path::new(""), Ok(dir(&[("a", Folder), ("b", Folder), ("c", Folder)])));
        for name in ["a", "b", "c"] {
            tree.toggle(Path::new(name));
        }
        tree.set(Path::new("a"), Ok(dir(&[])));
        tree.set(Path::new("b"), Err(Failure::Denied));
        let mut cut = dir(&[]);
        cut.more = 7;
        tree.set(Path::new("c"), Ok(cut));
        let kinds: Vec<RowKind> = tree.rows().into_iter().map(|row| row.kind).collect();
        assert_eq!(
            kinds,
            [
                RowKind::Folder { open: true },
                RowKind::Empty,
                RowKind::Folder { open: true },
                RowKind::Unreadable(Failure::Denied),
                RowKind::Folder { open: true },
                RowKind::More { count: 7, at_least: false },
            ]
        );
    }

    #[test]
    fn a_read_that_finds_the_same_folder_changes_nothing() {
        use EntryKind::File;
        let mut tree = sample();
        assert!(!tree.set(
            Path::new(""),
            Ok(dir(&[("src", EntryKind::Folder), ("docs", EntryKind::Folder), ("Cargo.toml", File)]))
        ));
        assert!(tree.set(Path::new(""), Ok(dir(&[("new", File)]))));
    }

    #[test]
    fn a_folder_that_was_not_on_screen_does_not_count_as_a_change() {
        use EntryKind::{File, Folder};
        let mut tree = sample();
        tree.toggle(Path::new("src"));
        tree.set(Path::new("src"), Ok(dir(&[("a", Folder)])));
        tree.toggle(Path::new("src"));
        assert!(!tree.set(Path::new("src"), Ok(dir(&[("a", Folder), ("b", File)]))), "closed: nothing shows");
    }

    #[test]
    fn an_open_folder_that_disappeared_takes_what_was_under_it() {
        use EntryKind::Folder;
        let mut tree = sample();
        tree.toggle(Path::new("src"));
        tree.set(Path::new("src"), Ok(dir(&[("bin", Folder)])));
        tree.toggle(Path::new("src/bin"));
        tree.set(Path::new("src/bin"), Ok(dir(&[])));
        assert!(tree.set(Path::new(""), Ok(dir(&[("docs", Folder)]))), "src is gone");
        assert!(!tree.is_open(Path::new("src")) && !tree.is_open(Path::new("src/bin")));
        assert_eq!(labels(&tree), ["docs"]);
        assert_eq!(tree.wanted(), [PathBuf::new()]);
    }

    #[test]
    fn what_is_wanted_is_the_root_and_the_open_folders_that_show() {
        use EntryKind::Folder;
        let mut tree = sample();
        tree.open_path(Path::new("src/bin"));
        assert_eq!(
            tree.wanted(),
            [PathBuf::new(), PathBuf::from("src")],
            "bin is not known until src is read"
        );
        tree.set(Path::new("src"), Ok(dir(&[("bin", Folder)])));
        assert_eq!(tree.wanted(), [PathBuf::new(), PathBuf::from("src"), PathBuf::from("src/bin")]);
        tree.toggle(Path::new("src"));
        assert_eq!(tree.wanted(), [PathBuf::new()], "a closed folder hides what is open under it");
    }

    #[test]
    fn clearing_forgets_everything() {
        let mut tree = sample();
        tree.toggle(Path::new("src"));
        tree.clear();
        assert!(!tree.is_open(Path::new("src")));
        let kinds: Vec<RowKind> = tree.rows().into_iter().map(|row| row.kind).collect();
        assert_eq!(kinds, [RowKind::Reading], "the root is not read yet");
    }

    #[test]
    fn the_root_can_be_empty_or_fail_and_says_so_in_a_row() {
        let mut tree = Tree::new();
        tree.set(Path::new(""), Err(Failure::Gone));
        let kinds =
            |tree: &Tree| tree.rows().into_iter().map(|row| (row.depth, row.kind)).collect::<Vec<_>>();
        assert_eq!(kinds(&tree), [(0, RowKind::Unreadable(Failure::Gone))]);
        tree.set(Path::new(""), Ok(dir(&[])));
        assert_eq!(kinds(&tree), [(0, RowKind::Empty)]);
    }

    #[test]
    fn a_renamed_folder_keeps_what_was_open_in_it() {
        use EntryKind::{File, Folder};
        let mut tree = sample();
        tree.toggle(Path::new("src"));
        tree.set(Path::new("src"), Ok(dir(&[("lib", Folder), ("main.rs", File)])));
        tree.toggle(Path::new("src/lib"));
        tree.set(Path::new("src/lib"), Ok(dir(&[("a.rs", File)])));
        assert_eq!(labels(&tree), ["docs", "src", "  lib", "    a.rs", "  main.rs", "Cargo.toml"]);

        tree.renamed(Path::new("src"), Path::new("code"));
        // At once, without a read: the folder is where its name sorts, open, with the same inside.
        assert_eq!(labels(&tree), ["code", "  lib", "    a.rs", "  main.rs", "docs", "Cargo.toml"]);
        assert!(tree.is_open(Path::new("code")) && tree.is_open(Path::new("code/lib")));
        assert!(!tree.is_open(Path::new("src")));
        // The read that follows finds the same and changes nothing.
        let same = dir(&[("code", Folder), ("docs", Folder), ("Cargo.toml", File)]);
        assert!(!tree.set(Path::new(""), Ok(same)));
        assert_eq!(labels(&tree).len(), 6);

        // A file, or a path that moves out of its folder, is not this tree's business.
        tree.renamed(Path::new("Cargo.toml"), Path::new("Cargo.lock"));
        assert_eq!(labels(&tree).last().map(String::as_str), Some("Cargo.lock"));
        tree.renamed(Path::new("docs"), Path::new("code/docs"));
        assert!(labels(&tree).contains(&"docs".to_string()));
    }
}
