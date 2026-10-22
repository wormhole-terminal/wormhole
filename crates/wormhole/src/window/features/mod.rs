//! The window's features: the side panels' contents and what they work on, each in its own folder until
//! phase 5 gives them their own state (all but notes and usage read and write `Workspace`'s fields today). `notes` holds the
//! notes and tasks of a terminal, `usage` the coding agents' plan limits, `project` the `.wormhole/`
//! folder's panel, trust and sync, and `git` the Changes, History and diff views and the worktrees.
//!
//! [`TABLE`] lists the features as the window's contract (`feature`) sees them. Changes, Notes and Usage have
//! their entries; a feature gets its entry when it moves out of `Workspace`'s fields, and the table
//! moves up beside the window when the last one has. Core's `FEATURES` already lists all of them.

pub(crate) mod git;
pub(crate) mod notes;
pub(crate) mod project;
pub(crate) mod usage;

use gpui_kit::App;

use crate::window::feature::{FeatureDef, FeatureTable};

/// The features in the table, in tab order.
pub(in crate::window) const TABLE: &[FeatureDef] = &[git::changes::DEF, notes::DEF, usage::DEF];

/// Hands the table to the windows opened from now on, and makes the app-wide state that features keep
/// outside their windows. The app root calls it at launch.
pub(crate) fn install(cx: &mut App) {
    usage::install(cx);
    FeatureTable::install(TABLE, cx);
}

/// The shown profile's data is in `AppState` now (at launch, after a profile switch): each feature that
/// keeps app-wide data reads this profile's.
pub(crate) fn profile_shown(cx: &mut App) {
    for profile in TABLE.iter().filter_map(|def| def.profile) {
        profile(cx);
    }
}
