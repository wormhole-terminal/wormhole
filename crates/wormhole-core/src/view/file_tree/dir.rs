//! One folder's entries as the tab lists them: folders first, then files, each group in natural order,
//! cut at a cap with a count of what was left out.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};

use super::names::{clean, natural_cmp};
use crate::storage::files::listing::{DirEntry, EntryKind, Listing};

/// The most entries of one folder the tab lists; the rest are a count ("120 more").
pub const ENTRY_CAP: usize = 5_000;

/// An entry of a folder: its real name (what a path is made of) and the name as drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: OsString,
    /// [`clean`]ed, with its whole extension.
    pub label: String,
    pub kind: EntryKind,
}

impl Entry {
    /// Whether the tree can open it: a real folder, never a link to one.
    pub fn is_folder(&self) -> bool {
        self.kind == EntryKind::Folder
    }
}

/// A folder as last read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dir {
    pub entries: Vec<Entry>,
    /// Entries left out: past the cap, or past what was read at all.
    pub more: usize,
    /// The reader stopped before the end of the folder, so `more` counts only what it saw: there are at
    /// least that many.
    pub at_least: bool,
    /// Each entry's place, by its real name: looking one up is not a walk of the list.
    index: HashMap<OsString, usize>,
}

impl Dir {
    /// Sorts what the disk gave and keeps the first `cap` (a folder of a hundred thousand files is
    /// cut after sorting, so the cap keeps the front of the alphabet and not whatever the disk listed
    /// first). Run it where the folder was read: sorting a big folder takes a moment.
    pub fn from_listing(listing: Listing, cap: usize) -> Self {
        let Listing { entries, truncated } = listing;
        let mut keyed: Vec<(String, EntryKind, OsString)> = entries
            .into_iter()
            .map(|entry| (entry.name.to_string_lossy().into_owned(), entry.kind, entry.name))
            .collect();
        // A link is listed with the files: what it points at is not looked at, so it is not known to be
        // a folder (and it never opens).
        let folder = |kind: EntryKind| kind == EntryKind::Folder;
        keyed.sort_by(|a, b| folder(b.1).cmp(&folder(a.1)).then_with(|| natural_cmp(&a.0, &b.0)));
        let more = keyed.len().saturating_sub(cap);
        keyed.truncate(cap);
        let entries: Vec<Entry> =
            keyed.into_iter().map(|(lossy, kind, name)| Entry { label: clean(&lossy), name, kind }).collect();
        let index = entries.iter().enumerate().map(|(at, entry)| (entry.name.clone(), at)).collect();
        Self { entries, more, at_least: truncated, index }
    }

    /// This folder with the entry `from` called `to` instead, in its place by the new name; `None` when
    /// there is no entry `from`. What the disk will say once a rename is made, so the tree need not wait
    /// for the next read to be true.
    pub fn renamed(&self, from: &OsStr, to: &OsStr) -> Option<Self> {
        self.get(from)?;
        let entries = self
            .entries
            .iter()
            .map(|entry| DirEntry {
                name: if entry.name == from { to.to_os_string() } else { entry.name.clone() },
                kind: entry.kind,
            })
            .collect();
        let mut renamed = Self::from_listing(Listing { entries, truncated: self.at_least }, usize::MAX);
        renamed.more = self.more;
        Some(renamed)
    }

    /// The entry called `name`.
    pub fn get(&self, name: &OsStr) -> Option<&Entry> {
        self.entries.get(*self.index.get(name)?)
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};

    use super::{Dir, ENTRY_CAP};
    use crate::storage::files::listing::{DirEntry, EntryKind, Listing};

    fn listing(entries: &[(&str, EntryKind)], truncated: bool) -> Listing {
        Listing {
            entries: entries
                .iter()
                .map(|&(name, kind)| DirEntry { name: OsString::from(name), kind })
                .collect(),
            truncated,
        }
    }

    fn labels(dir: &Dir) -> Vec<&str> {
        dir.entries.iter().map(|entry| entry.label.as_str()).collect()
    }

    #[test]
    fn folders_come_first_then_files_each_in_natural_order() {
        use EntryKind::{File, Folder};
        let dir = Dir::from_listing(
            listing(
                &[
                    ("file10.txt", File),
                    ("src", Folder),
                    (".git", Folder),
                    ("file2.txt", File),
                    ("Docs", Folder),
                    ("README.md", File),
                ],
                false,
            ),
            ENTRY_CAP,
        );
        assert_eq!(labels(&dir), [".git", "Docs", "src", "file2.txt", "file10.txt", "README.md"]);
        assert_eq!((dir.more, dir.at_least), (0, false));
    }

    #[test]
    fn a_link_sorts_with_the_files_and_never_opens() {
        use EntryKind::{File, Folder, Link};
        let dir = Dir::from_listing(
            listing(&[("a", File), ("z", Link), ("m", Folder), ("b", Link)], false),
            ENTRY_CAP,
        );
        assert_eq!(labels(&dir), ["m", "a", "b", "z"]);
        let flags: Vec<bool> = dir.entries.iter().map(|entry| entry.is_folder()).collect();
        assert_eq!(flags, [true, false, false, false], "only a real folder opens");
    }

    #[test]
    fn the_cap_keeps_the_front_of_the_alphabet_and_counts_the_rest() {
        let names: Vec<String> = (0..12).rev().map(|n| format!("f{n}")).collect();
        let entries: Vec<(&str, EntryKind)> = names.iter().map(|n| (n.as_str(), EntryKind::File)).collect();
        let dir = Dir::from_listing(listing(&entries, true), 5);
        assert_eq!(labels(&dir), ["f0", "f1", "f2", "f3", "f4"]);
        assert_eq!(dir.more, 7, "seven past the cap");
        assert!(dir.at_least, "and the reader stopped before the end, so there may be more");
    }

    #[test]
    fn an_entry_is_found_by_its_real_name() {
        let dir = Dir::from_listing(
            listing(&[("a\nb", EntryKind::Folder), ("c", EntryKind::File)], false),
            ENTRY_CAP,
        );
        assert!(dir.get(OsStr::new("a\nb")).is_some_and(|entry| entry.is_folder()));
        assert!(dir.get(OsStr::new("c")).is_some());
        assert!(dir.get(OsStr::new("d")).is_none());
    }

    #[test]
    fn a_hostile_name_is_cleaned_for_the_label_and_kept_for_the_path() {
        let dir = Dir::from_listing(listing(&[("a\nb", EntryKind::File)], false), ENTRY_CAP);
        assert_eq!(dir.entries[0].label, "a\u{FFFD}b");
        assert_eq!(dir.entries[0].name, OsString::from("a\nb"));
    }

    #[test]
    fn an_empty_folder_is_an_empty_dir() {
        assert_eq!(Dir::from_listing(Listing::default(), ENTRY_CAP), Dir::default());
    }
}
