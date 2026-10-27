//! Which profile a request reaches, and how the profile is told apart in this module's numbers
//! (`agent-permissions.md`, section 9): a profile is a trust domain with its own mode, database and approvals,
//! so a request is judged by the profile of the window it reaches, never by the one that happens to be shown.

use std::hash::{DefaultHasher, Hash as _, Hasher as _};

use gpui_kit::*;
use wormhole_core::ipc::control::{Action, Request, Target};
use wormhole_core::storage::paths::ProfileId;

use super::state;
use crate::app_state::{AppState, OpenWindow, app};

/// A profile as a number that fits in an approval's holder id: 30 bits of its name's hash.
pub(super) fn scope_of(profile: &ProfileId) -> u64 {
    let mut hasher = DefaultHasher::new();
    profile.hash(&mut hasher);
    hasher.finish() & 0x3FFF_FFFF
}

/// The number a stream is started under, to be ended with its approval: whether the approval is stored, the
/// profile it belongs to and its id there, so two databases' rows (and a row and a session approval) never
/// share one.
pub(super) fn holder(stored: bool, scope: u64, id: u64) -> u64 {
    (u64::from(stored) << 63) | ((scope & 0x3FFF_FFFF) << 32) | (id & 0xFFFF_FFFF)
}

/// Where a request goes.
pub(super) struct Reach {
    pub profile: ProfileId,
    pub scope: u64,
    /// The window is of a profile that is shown: a question can be put on it.
    pub shown: bool,
}

/// The terminal an action names, or defaults to (the caller's own, `from`).
fn target_of(action: &Action, from: Option<i64>) -> Option<Target> {
    let own = |target: Option<Target>| target.or(from.map(Target::Tab));
    match action {
        Action::SendText { target, .. } => Some(*target),
        Action::SendKeys(spec) => Some(spec.target),
        Action::Mouse(spec) => Some(spec.target),
        Action::Run(spec) => Some(spec.target),
        Action::Wait(spec) => Some(spec.target),
        Action::Close(spec) => Some(spec.target),
        Action::Focus(spec) => Some(spec.target),
        Action::Rename(spec) => Some(spec.target),
        Action::Split(spec) => Some(spec.target),
        Action::Read(spec) => own(spec.target),
        Action::Blocks(spec) => own(spec.target),
        Action::Status { target } | Action::Notify { target, .. } => own(*target),
        Action::Events(spec) => {
            spec.filter().ok().and_then(|f| f.terminals.first().map(|id| Target::Terminal(*id)))
        }
        _ => None,
    }
}

/// The window `request` reaches: the one with the terminal it names, the one named for a new terminal, else the
/// one a request with no target acts in (the front window: where a new terminal or window would open).
fn window_of(request: &Request, cx: &App) -> Option<OpenWindow> {
    let windows = &app(cx).windows;
    if let Action::NewTerminal { window: Some(id), .. } = &request.action
        && let Some(window) = windows.iter().find(|window| window.id == *id)
    {
        return Some(window.clone());
    }
    if let Some(target) = target_of(&request.action, request.from_tab())
        && let Some(window) = windows.iter().find(|window| {
            window.workspace.read_with(cx, |ws, _| ws.control_find(target)).ok().flatten().is_some()
        })
    {
        return Some(window.clone());
    }
    front(cx)
}

/// The window a request with no target would act in.
fn front(cx: &App) -> Option<OpenWindow> {
    AppState::front_window(cx).or_else(|| app(cx).windows.first().cloned())
}

/// The profile of the window `request` reaches.
pub(super) fn reach(request: &Request, cx: &mut App) -> Reach {
    let window = window_of(request, cx);
    reach_of(window, cx)
}

/// The profile of the window a request with no target acts in (what `doctor` is about).
pub(super) fn reach_front(cx: &mut App) -> Reach {
    let window = front(cx);
    reach_of(window, cx)
}

fn reach_of(window: Option<OpenWindow>, cx: &mut App) -> Reach {
    let profile =
        window.as_ref().map_or_else(|| app(cx).active_profile().clone(), |window| window.profile.clone());
    let scope = scope_of(&profile);
    state(cx).profiles.insert(scope, profile.clone());
    Reach { profile, scope, shown: window.is_none_or(|window| !window.hidden) }
}

/// The profile `scope` stands for, if it was ever seen.
pub(super) fn profile_of(scope: u64, cx: &mut App) -> Option<ProfileId> {
    state(cx).profiles.get(&scope).cloned()
}

#[cfg(test)]
mod tests {
    use wormhole_core::storage::paths::ProfileId;

    use super::{holder, scope_of};

    #[test]
    fn a_holder_names_where_an_approval_is_kept_and_which_profile_it_is_of() {
        let (a, b) =
            (scope_of(&ProfileId::default_profile()), scope_of(&ProfileId::parse("work").expect("an id")));
        assert_ne!(a, b);
        // The same row number in two databases, and a row and a session approval of one number, are four holders.
        let all = [holder(true, a, 3), holder(true, b, 3), holder(false, a, 3), holder(false, b, 3)];
        for (i, one) in all.iter().enumerate() {
            for other in &all[i + 1..] {
                assert_ne!(one, other);
            }
        }
        assert_eq!(scope_of(&ProfileId::default_profile()), a, "stable");
    }
}
