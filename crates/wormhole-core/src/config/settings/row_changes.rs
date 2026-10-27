//! What a terminal's row counts as its changed files (Settings → Terminals & sidebar → Changed files),
//! and how the count is worked out (`git::attribution`). Each choice is a snake_case word in
//! `config.json`.
//!
//! A word this build doesn't know reads as the default (`#[serde(other)]` on each default), unlike the
//! other choices of `choices`, which refuse it and leave the file unread until it is fixed. That is
//! chosen: these are habits, and a newer build's value should not stop an older one loading the rest of
//! the file. The cost is that the older build, saving, writes the default where the newer value was.

use serde::{Deserialize, Serialize};

/// What the row's ±N is of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangesScope {
    /// Everything changed in the repository, on the topmost row of the repository in the list: one
    /// number for all of its terminals, not the same number on each.
    Repository,
    /// The uncommitted files this terminal changed.
    #[default]
    #[serde(other)]
    Terminal,
}

/// Which programs in front make a plain terminal (a shell without an agent or a status of its own)
/// count as working, so that the files changed meanwhile are its.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlainChanges {
    /// Any program in front, an editor or a dev server too: they stay open for as long as you like
    /// and take every edit made outside the app meanwhile.
    Any,
    /// Never: only terminals that tell their status count.
    None,
    /// A command that ends by itself (a build, a formatter, a test run).
    #[default]
    #[serde(other)]
    Finishing,
}

/// What becomes of a file that more than one terminal changed (they worked at the same time, so
/// it can't be told whose it is).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedChanges {
    /// Nobody's: it is in no terminal's count.
    None,
    /// Every one of them counts it.
    #[default]
    #[serde(other)]
    Each,
}
