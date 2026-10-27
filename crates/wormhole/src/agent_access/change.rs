//! The mode in force, and a mode that was widened outside Wormhole (`agent-permissions.md`, 9.1): a
//! `config.json` edited by an editor, a script or a program must not let programs in by itself. A change that
//! widens access (Off to Ask, Off to Open, Ask to Open) and did not come from Settings keeps the earlier mode
//! in force and asks the user whether to keep it; narrowing, and anything done in Settings, applies at once.

use gpui_kit::*;
use wormhole_core::config::AgentAccess;
use wormhole_core::storage::paths::ProfileId;

use super::profile::scope_of;
use super::{State, state};
use crate::settings::{store_in, update_in};

/// A mode that was changed outside Wormhole to something wider, waiting for the user's answer.
#[derive(Debug, Clone)]
pub(super) struct Change {
    pub scope: u64,
    pub profile: ProfileId,
    /// The mode that stays in force until the user answers, and comes back if they say no.
    pub previous: AgentAccess,
    pub requested: AgentAccess,
    /// The question is on the screen (or answered by a script).
    pub asked: bool,
    /// Tells this question from an earlier one when a minute's timer fires.
    pub token: u64,
}

static TOKENS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// How much a mode lets in.
fn width(mode: AgentAccess) -> u8 {
    match mode {
        AgentAccess::Off => 0,
        AgentAccess::Ask => 1,
        AgentAccess::Open => 2,
    }
}

/// The mode in force for `profile`: what its settings say, unless that is a loosening nobody in Wormhole made.
/// Looked at when a request comes and on the tick, so a change in Settings is in force before the next request.
pub(super) fn reconcile(profile: &ProfileId, cx: &mut App) -> AgentAccess {
    let scope = scope_of(profile);
    let want = crate::settings::settings_in(profile, cx).agent_access;
    let edits = store_in(profile, cx).ui_edits;
    let state = state(cx);
    state.profiles.insert(scope, profile.clone());
    let seen = state.seen_edits.insert(scope, edits);
    let Some(&enforced) = state.enforced.get(&scope) else {
        // The first look is the mode the app was started with.
        state.enforced.insert(scope, want);
        return want;
    };
    if want == enforced || state.change.as_ref().is_some_and(|change| change.scope == scope) {
        return enforced;
    }
    let by_the_user = seen != Some(edits);
    if by_the_user || width(want) < width(enforced) {
        state.enforced.insert(scope, want);
        return want;
    }
    tracing::warn!("agent access was widened outside Wormhole to {}: asking whether to keep it", want.word());
    state.change = Some(Change {
        scope,
        profile: profile.clone(),
        previous: enforced,
        requested: want,
        asked: false,
        token: TOKENS.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    });
    cx.defer(super::ask::pump);
    enforced
}

/// The user's answer: keep the change, or put the earlier mode back in the file.
pub(super) fn answered(keep: bool, cx: &mut App) {
    let Some(change) = state(cx).change.take() else { return };
    if keep {
        state(cx).enforced.insert(change.scope, change.requested);
    } else {
        update_in(&change.profile, cx, |settings| settings.agent_access = change.previous);
        // What was written is the user's own edit: it is the mode in force, with nothing to ask.
        let edits = store_in(&change.profile, cx).ui_edits;
        state(cx).seen_edits.insert(change.scope, edits);
    }
    cx.refresh_windows();
}

impl State {
    /// Whether the question about a changed mode is waiting for its turn on the screen.
    pub(super) fn change_waits(&self) -> bool {
        self.change.as_ref().is_some_and(|change| !change.asked)
    }
}

#[cfg(test)]
mod tests {
    use super::width;
    use wormhole_core::config::AgentAccess::{Ask, Off, Open};

    #[test]
    fn a_mode_is_wider_the_more_it_lets_in() {
        assert!(width(Off) < width(Ask) && width(Ask) < width(Open));
    }
}
