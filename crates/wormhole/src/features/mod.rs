//! The window's features: the side panels' contents and what they work on, each in its own folder and
//! each owning its state in an entity of its own that the window reaches only through the table.
//! `notes` holds the notes and tasks of a terminal, `usage` the coding agents' plan limits, `project`
//! the `.wormhole/` folder's panel, trust and sync, and `git` the Changes and diff views, the History
//! tab and the worktrees.
//!
//! [`TABLE`] lists the features as the window's contract (`feature`) sees them. Changes, Notes, Usage and
//! Project have their entries; a feature gets its entry when it moves out of `Workspace`'s fields, and the
//! table moves up beside the window when the last one has. Core's `FEATURES` already lists all of them.

pub(crate) mod git;
pub(crate) mod notes;
pub(crate) mod project;
pub(crate) mod usage;

use gpui_kit::App;

use crate::window::feature::{FeatureDef, FeatureTable};

/// The features in the table, in tab order.
pub(crate) const TABLE: &[FeatureDef] =
    &[git::changes::DEF, git::history::DEF, notes::DEF, usage::DEF, project::DEF, git::worktree::DEF];

/// Hands the table to the windows opened from now on, and makes the app-wide state that features keep
/// outside their windows. The app root calls it at launch.
pub(crate) fn install(cx: &mut App) {
    usage::install(cx);
    FeatureTable::install(TABLE, cx);
}
