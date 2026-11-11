//! What the diff card says where it shows no lines (`features/git/diff/notes.rs`): a note over each column with nothing to
//! show (a new or deleted file's missing side, a binary file with its sizes, a Git LFS pointer, an empty file, a change of
//! mode alone). Core owns the sides and the figures; the words are the GPUI card's.

use wormhole_core::config::SizeUnits;
use wormhole_core::git::{self, ChangeKind, FileDiff, Side};

/// The note over a column: why it is empty, a size or what a pointer stands for, and the LFS object to copy.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitSideNote {
    pub text: String,
    pub detail: Option<String>,
    pub oid: Option<String>,
}

/// The notes of a diff with no hunks: one per column when split (old, new), one when unified.
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct GitDiffNotes {
    pub split: Vec<Option<GitSideNote>>,
    pub unified: Option<GitSideNote>,
}

fn text(side: Side, old_side: bool, sides: (Side, Side)) -> Option<&'static str> {
    Some(match side {
        Side::Missing if old_side => "New file",
        Side::Missing => "Deleted",
        Side::Binary => "Binary file",
        Side::Lfs => "Stored in Git LFS",
        Side::Text if sides.0 == Side::Missing || sides.1 == Side::Missing => "Empty file",
        Side::Text => "No content changes",
    })
}

pub(crate) fn notes(diff: &FileDiff, kind: ChangeKind, units: SizeUnits) -> GitDiffNotes {
    let (old, new) = git::sides(diff, kind);
    let sides = (old, new);
    let pointer = |old_side: bool| {
        diff.lfs.as_ref().and_then(|change| if old_side { change.old.as_ref() } else { change.new.as_ref() })
    };
    let split_note = |side: Side, old_side: bool| {
        let (detail, oid) = match side {
            Side::Binary => ((if old_side { diff.sizes.0 } else { diff.sizes.1 }).map(|b| git::byte_size(b, units)), None),
            Side::Lfs => (pointer(old_side).map(|p| git::lfs_side(p, units)), pointer(old_side).map(|p| p.oid.clone())),
            _ => (None, None),
        };
        text(side, old_side, sides).map(|text| GitSideNote { text: text.into(), detail, oid })
    };
    // One column: the side the file has now (the old one when it was deleted), with both sides' sizes.
    let side = if new == Side::Missing { old } else { new };
    let (detail, oid) = match side {
        Side::Binary => (git::sizes_label(diff.sizes.0, diff.sizes.1, units), None),
        Side::Lfs => {
            let named = diff.lfs.as_ref().and_then(|change| change.new.as_ref().or(change.old.as_ref()));
            (diff.lfs.as_ref().and_then(|change| git::lfs_summary(change, units)), named.map(|p| p.oid.clone()))
        }
        _ => (None, None),
    };
    GitDiffNotes {
        split: vec![split_note(old, true), split_note(new, false)],
        unified: text(side, false, sides).map(|text| GitSideNote { text: text.into(), detail, oid }),
    }
}
