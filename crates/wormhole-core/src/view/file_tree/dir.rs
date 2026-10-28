//! One folder's entries as the tab lists them: folders first, then files, each group in natural order,
//! cut at a cap with a count of what was left out.

use std::ffi::OsString;

use super::names::{clean, natural_cmp};
use crate::storage::files::listing::{EntryKind, Listing};

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
}

impl Dir {
    /// Sorts what the disk gave and keeps the first `cap` (a folder of a hundred thousand files is
    /// cut after sorting, so the cap keeps the front of the alphabet and not whatever the disk listed
    /// first). Run it where the folder was read: sorting a big folder takes a moment.
    pub fn from_listing(listing: Listing, cap: usize) -> Self {
        let Listing { entries, unread } = listing;
        // A link to a folder is listed with the folders, as Finder does, though it never opens.
        let is_folder =
            |kind: EntryKind| matches!(kind, EntryKind::Folder | EntryKind::Link { to_folder: true });
        let mut keyed: Vec<(String, EntryKind, OsString)> = entries
            .into_iter()
            .map(|entry| (entry.name.to_string_lossy().into_owned(), entry.kind, entry.name))
            .collect();
        keyed.sort_by(|a, b| is_folder(b.1).cmp(&is_folder(a.1)).then_with(|| natural_cmp(&a.0, &b.0)));
        let more = keyed.len().saturating_sub(cap) + unread;
        keyed.truncate(cap);
        let entries =
            keyed.into_iter().map(|(lossy, kind, name)| Entry { label: clean(&lossy), name, kind }).collect();
        Self { entries, more }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{Dir, ENTRY_CAP};
    use crate::storage::files::listing::{DirEntry, EntryKind, Listing};

    fn listing(entries: &[(&str, EntryKind)], unread: usize) -> Listing {
        Listing {
            entries: entries
                .iter()
                .map(|&(name, kind)| DirEntry { name: OsString::from(name), kind })
                .collect(),
            unread,
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
                0,
            ),
            ENTRY_CAP,
        );
        assert_eq!(labels(&dir), [".git", "Docs", "src", "file2.txt", "file10.txt", "README.md"]);
        assert_eq!(dir.more, 0);
    }

    #[test]
    fn a_link_to_a_folder_sorts_with_the_folders_but_is_not_one() {
        use EntryKind::{File, Folder, Link};
        let dir = Dir::from_listing(
            listing(
                &[
                    ("a", File),
                    ("z", Link { to_folder: true }),
                    ("m", Folder),
                    ("b", Link { to_folder: false }),
                ],
                0,
            ),
            ENTRY_CAP,
        );
        assert_eq!(labels(&dir), ["m", "z", "a", "b"]);
        let flags: Vec<bool> = dir.entries.iter().map(|entry| entry.is_folder()).collect();
        assert_eq!(flags, [true, false, false, false], "only a real folder opens");
    }

    #[test]
    fn the_cap_keeps_the_front_of_the_alphabet_and_counts_the_rest() {
        let names: Vec<String> = (0..12).rev().map(|n| format!("f{n}")).collect();
        let entries: Vec<(&str, EntryKind)> = names.iter().map(|n| (n.as_str(), EntryKind::File)).collect();
        let dir = Dir::from_listing(listing(&entries, 3), 5);
        assert_eq!(labels(&dir), ["f0", "f1", "f2", "f3", "f4"]);
        assert_eq!(dir.more, 7 + 3, "seven past the cap and three the reader left unread");
    }

    #[test]
    fn a_hostile_name_is_cleaned_for_the_label_and_kept_for_the_path() {
        let dir = Dir::from_listing(listing(&[("a\nb", EntryKind::File)], 0), ENTRY_CAP);
        assert_eq!(dir.entries[0].label, "a\u{FFFD}b");
        assert_eq!(dir.entries[0].name, OsString::from("a\nb"));
    }

    #[test]
    fn an_empty_folder_is_an_empty_dir() {
        assert_eq!(Dir::from_listing(Listing::default(), ENTRY_CAP), Dir::default());
    }
}
