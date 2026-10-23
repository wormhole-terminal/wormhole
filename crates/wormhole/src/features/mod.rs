//! The window's features: the side panels' contents and what they work on, each in its own folder and
//! each owning its state in an entity of its own that the window reaches only through the table.
//! `notes` holds the notes and tasks of a terminal, `usage` the coding agents' plan limits, `project`
//! the `.wormhole/` folder's panel, trust and sync, and `git` the Changes and diff views, the History
//! tab and the worktrees.
//!
//! [`TABLE`] lists the features as the window's contract (`window::feature`) sees them; the app installs it
//! at launch and the window holds it without naming a feature. Above the window and below the surfaces
//! that list them (the palette, Settings, the status item): a feature reaches the window only through the
//! contract, the facts and the repository read (the `feature-api` rule), and another feature not at all
//! but for the one bridge the checker's config lists (`project`'s notes sync). Core's `FEATURES` lists
//! all of them.

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
