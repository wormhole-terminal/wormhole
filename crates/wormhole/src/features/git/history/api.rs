//! What the surfaces above the window (the command palette) may ask of History about a window: show a
//! commit it found. Each takes the window's workspace and finds the feature's entity in it, so none holds
//! the window's lease while it works on the entity. With History off there is nothing to show it in.

use gpui_kit::*;
use wormhole_core::config::FeatureId;
use wormhole_core::git::Commit;

use super::History;
use crate::window::Workspace;

/// Whether History is on in this window's profile.
pub(crate) fn on(ws: &Workspace, cx: &App) -> bool {
    crate::settings::features_in(ws.profile(), cx).has(FeatureId::History)
}

/// Shows `commit` of the repository at `root` open in the History tab of the terminal in front.
pub(crate) fn reveal_commit(ws: &Workspace, root: std::path::PathBuf, commit: Commit, cx: &mut App) {
    if !on(ws, cx) {
        return;
    }
    let Some(history) = ws.feature::<History>() else { return };
    let facts = ws.facts(cx);
    let Some(session) = facts.active().map(|active| active.id) else { return };
    history.update(cx, |history, cx| {
        history.follow(&facts);
        history.reveal_commit(session, root, commit, cx);
    });
}
