//! What the regions draw of the app's shared state (`AppState`).
//!
//! `AppState` is marked changed by every `app_mut`, whether or not a field changed, and callers write it
//! for reasons the chrome does not show (a project remembered, a lookup begun, a save). The windows watch it
//! for these facts only: they draw their regions again when one differs from what they saw last.
//!
//! A branch's pull request is not among them although it lives here too (`AppState::pr`): its answer ends in
//! `cx.refresh_windows()` (`Workspace::ask_for_pull_request`), which draws every region again, and the title
//! row's facts (`ExtraFacts`) are what the check on the chrome reads it with.

use wormhole_core::config::profile::ProfileList;
use wormhole_core::onboarding::DismissedTips;

use crate::app_state::app;

/// The facts of the shared state that a window's regions draw.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::window) struct AppFacts {
    /// The profile chip, the odd window's tag and edge, and the colours a profile's windows wear.
    profiles: ProfileList,
    /// Whether the sidebar's note after a first run without agents was closed for good.
    tips: DismissedTips,
    /// The open windows' ids and whether each is hidden: the status bar counts the visible ones.
    windows: Vec<(i64, bool)>,
}

impl AppFacts {
    /// What the shared state says now.
    pub(in crate::window) fn of(cx: &gpui_kit::App) -> Self {
        let state = app(cx);
        Self {
            profiles: state.profiles.clone(),
            tips: state.tips.clone(),
            windows: state.windows.iter().map(|window| (window.id, window.hidden)).collect(),
        }
    }
}
