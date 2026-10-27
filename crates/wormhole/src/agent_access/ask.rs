//! The questions: which is on the screen, what an answer does, and what happens when nobody answers or the
//! mode changes under them.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use gpui_kit::*;
use wormhole_core::config::AgentAccess;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::ipc::access::{CapSet, Pending, Scope};
use wormhole_core::ipc::control::{ASK_WAIT_S, Error, ErrorCode, Reason, Reply};
use wormhole_core::ipc::{Decision, access};
use wormhole_core::platform::process;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::storage::store::AgentGrantRow;

use super::change::{self, reconcile};
use super::profile::{holder, profile_of, scope_of};
use super::scripted::{Asker, Script};
use super::sheet::{self, Button};
use super::{CHANGE_ID, Shown, state};
use crate::app_state::{AppState, OpenWindow, app, app_mut};
use crate::settings::settings_in;

/// How a question ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// The user pressed a button.
    Chose(Button),
    /// Nobody answered in time: no, and not remembered.
    TimedOut,
    /// The sheet went without an answer (its window closed, or none could be shown): no for now, and not
    /// remembered.
    Withdrawn,
}

/// Put the next question on the screen, if none is: a mode changed outside Wormhole first, then the queue.
/// Called when one is queued, when one is answered and on the tick (a window may have opened since).
pub(super) fn pump(cx: &mut App) {
    if state(cx).shown.is_some() {
        return;
    }
    if state(cx).change_waits() {
        show_change(cx);
        return;
    }
    let Some(id) = state(cx).queue.next_to_show().map(|pending| pending.id) else { return };
    let (display, scope) = {
        let held = cx.global::<super::State>();
        match held.queue.iter().find(|pending| pending.id == id) {
            Some(pending) => (sheet::display(pending, cx), pending.scope),
            None => return,
        }
    };
    state(cx).shown = Some(Shown { id, window: None });
    start_timer(id, cx);
    match state(cx).asker {
        Asker::Scripted(Script::Hold) => {}
        Asker::Scripted(Script::Answer(button)) => {
            let button = offered(&display.buttons, button);
            cx.defer(move |cx| settle(id, Outcome::Chose(button), cx));
        }
        Asker::Sheet => show(id, scope, display, cx),
    }
}

/// The button a script's wish comes to when the question does not offer it: the longest allowance it does.
fn offered(buttons: &[Button], wish: Button) -> Button {
    if buttons.contains(&wish) {
        return wish;
    }
    match wish {
        Button::Deny | Button::DontKeep => Button::Deny,
        Button::Session | Button::Always | Button::Keep => {
            if buttons.contains(&Button::Quit) {
                Button::Quit
            } else {
                Button::Deny
            }
        }
        Button::Quit => Button::Deny,
    }
}

/// The window a question of `scope` goes on: the front window if it is of that profile, else any shown one.
fn window_for(scope: u64, cx: &mut App) -> Option<OpenWindow> {
    let profile = profile_of(scope, cx)?;
    AppState::front_window(cx).filter(|window| window.profile == profile).or_else(|| {
        app(cx).windows.iter().find(|window| !window.hidden && window.profile == profile).cloned()
    })
}

fn show(id: u64, scope: u64, display: sheet::Display, cx: &mut App) {
    let Some(window) = window_for(scope, cx) else {
        settle(id, Outcome::Withdrawn, cx);
        return;
    };
    let Some(answers) = sheet::show(&window, &display, cx) else {
        settle(id, Outcome::Withdrawn, cx);
        return;
    };
    if let Some(shown) = state(cx).shown.as_mut() {
        shown.window = Some(window);
    }
    let buttons = display.buttons;
    cx.spawn(async move |cx| {
        let place = answers.recv().await.ok().flatten();
        let outcome = match place.and_then(|place| buttons.get(place)) {
            Some(button) => Outcome::Chose(*button),
            None => Outcome::Withdrawn,
        };
        cx.update(|cx| settle(id, outcome, cx));
    })
    .detach();
}

/// The question about a mode changed outside Wormhole, on the screen (or answered by a script).
fn show_change(cx: &mut App) {
    let Some(change) = state(cx).change.as_mut() else { return };
    change.asked = true;
    let (token, scope) = (change.token, change.scope);
    let display = sheet::change_display(change.previous, change.requested);
    state(cx).shown = Some(Shown { id: CHANGE_ID, window: None });
    start_change_timer(token, cx);
    match state(cx).asker {
        Asker::Scripted(Script::Hold) => {}
        Asker::Scripted(Script::Answer(button)) => {
            cx.defer(move |cx| settle_change(token, button != Button::Deny, cx));
        }
        Asker::Sheet => {
            let Some(window) = window_for(scope, cx) else {
                settle_change(token, false, cx);
                return;
            };
            let Some(answers) = sheet::show(&window, &display, cx) else {
                settle_change(token, false, cx);
                return;
            };
            if let Some(shown) = state(cx).shown.as_mut() {
                shown.window = Some(window);
            }
            let buttons = display.buttons;
            cx.spawn(async move |cx| {
                let place = answers.recv().await.ok().flatten();
                let keep = place.and_then(|place| buttons.get(place)) == Some(&Button::Keep);
                cx.update(|cx| settle_change(token, keep, cx));
            })
            .detach();
        }
    }
}

/// A question that nobody answers counts as no after a minute, and is not remembered.
fn start_timer(id: u64, cx: &mut App) {
    cx.spawn(async move |cx| {
        cx.background_executor().timer(Duration::from_secs(ASK_WAIT_S)).await;
        cx.update(|cx| settle(id, Outcome::TimedOut, cx));
    })
    .detach();
}

/// The same for the question about a changed mode: not answered is not kept.
fn start_change_timer(token: u64, cx: &mut App) {
    cx.spawn(async move |cx| {
        cx.background_executor().timer(Duration::from_secs(ASK_WAIT_S)).await;
        cx.update(|cx| settle_change(token, false, cx));
    })
    .detach();
}

/// End the question about a changed mode (`token` tells it from an earlier one): keep it or put the mode back.
fn settle_change(token: u64, keep: bool, cx: &mut App) {
    if state(cx).change.as_ref().is_none_or(|change| change.token != token || !change.asked) {
        return;
    }
    if let Some(Shown { window: Some(window), .. }) = state(cx).shown.take_if(|shown| shown.id == CHANGE_ID) {
        sheet::withdraw(&window, cx);
    }
    state(cx).shown.take_if(|shown| shown.id == CHANGE_ID);
    change::answered(keep, cx);
    pump(cx);
}

/// End question `id` with `outcome`: record what it means, tell every request that waited, take the sheet
/// off the screen and put the next question up.
fn settle(id: u64, outcome: Outcome, cx: &mut App) {
    if id == CHANGE_ID {
        return;
    }
    let Some(pending) = state(cx).queue.take(id) else { return };
    if let Some(Shown { window: Some(window), .. }) = state(cx).shown.take_if(|shown| shown.id == id) {
        sheet::withdraw(&window, cx);
    }
    let resolved = pending.resolved.clone();
    let label = resolved.label.clone();
    let scope = pending.scope;
    match (outcome, pending.about) {
        (Outcome::Chose(Button::Deny), Some(terminal)) => {
            state(cx).scopes.deny(resolved.identity, terminal, Instant::now());
            pending.answer(|| Decision::Refuse(declined_terminal(&label, terminal).into()));
        }
        (Outcome::Chose(Button::Deny), None) => {
            state(cx).grants_of(scope).deny(resolved.identity, Instant::now());
            pending.answer(|| Decision::Refuse(declined(&label).into()));
        }
        (Outcome::Chose(button), Some(terminal)) => {
            widen(&resolved, button, terminal, pending.caps, cx);
            pending.answer(|| Decision::Retry);
        }
        (Outcome::Chose(button), None) => {
            allow(&resolved, button, scope, cx);
            pending.answer(|| Decision::Retry);
        }
        (Outcome::TimedOut, _) => pending.answer(|| Decision::Refuse(timed_out(&label).into())),
        (Outcome::Withdrawn, _) => pending.answer(|| Decision::Refuse(not_answered(&label).into())),
    }
    // Settings lists what is allowed.
    state(cx).listing = None;
    cx.refresh_windows();
    pump(cx);
}

/// Remember that `resolved` was let in, as long as `button` says, in the profile `scope` stands for.
fn allow(resolved: &access::Resolved, button: Button, scope: u64, cx: &mut App) {
    let caps = CapSet::STANDARD;
    let label = resolved.label.clone();
    let identity = resolved.identity.clone();
    match (button, resolved.process) {
        (Button::Session, Some(process)) => {
            state(cx).grants_of(scope).grant(identity, label, Scope::Session(process), caps);
        }
        (Button::Always, _) if identity.can_be_remembered() => {
            if !store_always(resolved, caps, scope, cx) {
                // The database would not take it: it holds until Wormhole quits, which the user can see.
                state(cx).grants_of(scope).grant(identity, label, Scope::UntilQuit, caps);
            }
        }
        _ => {
            state(cx).grants_of(scope).grant(identity, label, Scope::UntilQuit, caps);
        }
    }
}

/// Open `terminal` up to `resolved` for as long as `button` says, for what the request that asked needed:
/// "for this session" while the program runs, otherwise until Wormhole quits (a terminal is never
/// approved for good: it is not a stable thing to approve).
fn widen(resolved: &access::Resolved, button: Button, terminal: i64, caps: CapSet, cx: &mut App) {
    let scope = match (button, resolved.process) {
        (Button::Session, Some(process)) => Scope::Session(process),
        _ => Scope::UntilQuit,
    };
    state(cx).scopes.widen(resolved.identity.clone(), scope, terminal, caps);
}

fn store_always(resolved: &access::Resolved, caps: CapSet, scope: u64, cx: &mut App) -> bool {
    let Some(profile) = profile_of(scope, cx) else { return false };
    let program = resolved.program.as_ref();
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0));
    let row = AgentGrantRow {
        key: resolved.identity.key(),
        label: resolved.label.clone(),
        location: program.map(|program| program.path.to_string_lossy().into_owned()).unwrap_or_default(),
        cdhash: program.and_then(|program| program.signature.cdhash.clone()).unwrap_or_default(),
        caps: caps.bits(),
        created,
        ..AgentGrantRow::default()
    };
    let Some(store) = app_mut(cx).store_of_mut(&profile) else { return false };
    match store.save_agent_grant(&row) {
        Ok(_) => true,
        Err(err) => {
            tracing::error!("couldn't store an approval: {err:#}");
            false
        }
    }
}

fn declined(label: &str) -> Error {
    Error::new(
        ErrorCode::NotAllowed,
        format!("the user did not allow {label} to control Wormhole: do not retry; tell the user what you wanted to do and ask them"),
    )
    .because(Reason::Declined)
}

fn declined_terminal(label: &str, terminal: i64) -> Error {
    Error::new(
        ErrorCode::NotAllowed,
        format!("the user did not allow {label} to use terminal {terminal}, which it did not open: do not retry; tell the user what you wanted to do there and ask them"),
    )
    .because(Reason::Declined)
}

fn timed_out(label: &str) -> Error {
    Error::new(
        ErrorCode::NotAllowed,
        format!("nobody answered the question about {label} in Wormhole within a minute; it counts as no, and is not remembered"),
    )
    .because(Reason::AskTimedOut)
}

fn not_answered(label: &str) -> Error {
    Error::new(
        ErrorCode::NotAllowed,
        format!("the question about {label} went away without an answer (its window closed, or none could be shown); it counts as no for now, and is not remembered"),
    )
    .because(Reason::NotAnswered)
}

fn access_off() -> Reply {
    Error::new(ErrorCode::NotAllowed, "agent access was switched off while the question was open")
        .because(Reason::AgentAccessOff)
        .into()
}

/// A question as the tests and devctl see it.
#[cfg(any(test, feature = "devctl"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Waiting {
    pub id: u64,
    pub label: String,
    /// Whether it is the one on the screen.
    pub shown: bool,
    pub buttons: Vec<Button>,
    /// The terminal it is about, when it is not the question about the program.
    pub about: Option<i64>,
    /// What the sheet says: its title, its text and the block of facts under it.
    pub title: String,
    pub detail: String,
    pub facts: String,
}

/// The questions waiting for an answer, the one on the screen first; the question about a changed mode (id 0)
/// before them when there is one. `words` also says what the sheet says (title, text, facts), which reads the
/// windows: not from inside the update of one.
#[cfg(any(test, feature = "devctl"))]
pub(crate) fn pending_questions(words: bool, cx: &mut App) -> Vec<Waiting> {
    let shown = state(cx).shown.as_ref().map(|shown| shown.id);
    let held = cx.global::<super::State>();
    let mut waiting: Vec<Waiting> = held
        .queue
        .iter()
        .map(|pending| {
            let said = words.then(|| sheet::display(pending, cx));
            Waiting {
                id: pending.id,
                label: pending.resolved.label.clone(),
                about: pending.about,
                shown: Some(pending.id) == shown,
                buttons: sheet::buttons(pending),
                title: said.as_ref().map(|said| said.title.clone()).unwrap_or_default(),
                detail: said.as_ref().map(|said| said.detail.clone()).unwrap_or_default(),
                facts: said.map(|said| said.accessory).unwrap_or_default(),
            }
        })
        .collect();
    if let Some(change) = &held.change {
        let said = sheet::change_display(change.previous, change.requested);
        waiting.push(Waiting {
            id: CHANGE_ID,
            label: String::new(),
            about: None,
            shown: shown == Some(CHANGE_ID),
            buttons: said.buttons,
            title: said.title,
            detail: said.detail,
            facts: String::new(),
        });
    }
    waiting.sort_by_key(|question| !question.shown);
    waiting
}

/// Answer the question on the screen with `button` (devctl's `agent-ask`, a test): as if it was pressed, a
/// moment later, outside the update that asked. Whether there was one. For the question about a changed mode
/// `deny` is Don't Keep and anything else is Keep.
#[cfg(any(test, feature = "devctl"))]
pub(crate) fn answer_pending(button: Button, cx: &mut App) -> bool {
    let Some(id) = state(cx).shown.as_ref().map(|shown| shown.id) else { return false };
    if id == CHANGE_ID {
        let Some(token) = state(cx).change.as_ref().map(|change| change.token) else { return false };
        cx.defer(move |cx| settle_change(token, button != Button::Deny, cx));
        return true;
    }
    let Some(pending) = cx.global::<super::State>().queue.iter().find(|pending| pending.id == id) else {
        return false;
    };
    let button = offered(&sheet::buttons(pending), button);
    cx.defer(move |cx| settle(id, Outcome::Chose(button), cx));
    true
}

/// A question that was withdrawn (its window closed, no sheet could be shown): the test's way to do it.
#[cfg(test)]
pub(crate) fn withdraw_pending(cx: &mut App) -> bool {
    let Some(id) = state(cx).shown.as_ref().map(|shown| shown.id).filter(|id| *id != CHANGE_ID) else {
        return false;
    };
    settle(id, Outcome::Withdrawn, cx);
    true
}

/// Whether some profile lets every program read every terminal (Allowed while open), as the last tick settled:
/// the streams that were let in by that end when it stops.
pub(crate) fn reads_open(cx: &mut App) -> bool {
    state(cx).applied.values().any(|mode| matches!(mode, Some(AgentAccess::Open)))
}

/// What the app does about this on its tick: look at each profile's mode (a change in Settings applies at
/// once, one made outside Wormhole that widens asks first), settle what a mode that changed means for the
/// questions and streams of its profile, forget the approvals of programs that ended, and put a question up that
/// was waiting for a window.
pub(crate) fn tick(ticks: u64, cx: &mut App) {
    let mut profiles: Vec<ProfileId> = vec![app(cx).active_profile().clone()];
    for window in &app(cx).windows {
        if !profiles.contains(&window.profile) {
            profiles.push(window.profile.clone());
        }
    }
    for profile in profiles {
        settle_mode(&profile, cx);
    }
    let sweep_sessions = ticks.is_multiple_of(5);
    if sweep_sessions {
        end_sessions_of_ended_programs(cx);
    }
    // A closed terminal drops out of every scope; a session's opening of a terminal ends with the program.
    super::scopes::sweep(sweep_sessions, cx);
    pump(cx);
}

/// What `profile`'s mode, as it stands now, means for what is waiting and what is open there.
fn settle_mode(profile: &ProfileId, cx: &mut App) {
    let scope = scope_of(profile);
    let on = settings_in(profile, cx).feature_enabled(FeatureFlag::Automation);
    let mode = on.then(|| reconcile(profile, cx));
    if state(cx).applied.insert(scope, mode) == Some(mode) {
        return;
    }
    match mode {
        Some(AgentAccess::Open) => allow_waiting_questions(scope, cx),
        Some(AgentAccess::Ask) => {}
        Some(AgentAccess::Off) | None => {
            end_streams_under_approvals(scope, profile, cx);
            refuse_waiting_questions(scope, cx);
        }
    }
}

/// The questions of `scope` out of the queue, with the sheet of the one on the screen taken down.
fn take_questions(scope: u64, cx: &mut App) -> Vec<Pending> {
    let questions = state(cx).queue.take_scope(scope);
    let shown = state(cx).shown.as_ref().map(|shown| shown.id);
    if shown.is_some_and(|id| questions.iter().any(|pending| pending.id == id))
        && let Some(Shown { window, .. }) = state(cx).shown.take()
        && let Some(window) = window
    {
        sheet::withdraw(&window, cx);
    }
    questions
}

/// Everything waiting is refused: access was switched off while the user had not answered.
fn refuse_waiting_questions(scope: u64, cx: &mut App) {
    for pending in take_questions(scope, cx) {
        pending.answer(|| Decision::Refuse(access_off()));
    }
}

/// Everything waiting is let in: every program is allowed now, so there is nothing to ask.
fn allow_waiting_questions(scope: u64, cx: &mut App) {
    for pending in take_questions(scope, cx) {
        pending.answer(|| Decision::Retry);
    }
}

/// What was allowed no longer reaches the streams it started.
fn end_streams_under_approvals(scope: u64, profile: &ProfileId, cx: &mut App) {
    let held: Vec<u64> = state(cx).grants_of(scope).held().iter().map(|held| held.id).collect();
    let stored: Vec<i64> = {
        let shown = app(cx);
        shown
            .store_of(profile)
            .and_then(|store| store.agent_grants().ok())
            .map(|rows| rows.into_iter().map(|row| row.id).collect())
            .unwrap_or_default()
    };
    if let Some(bus) = crate::automation::bus(cx) {
        for id in held {
            bus.revoke_holder(holder(false, scope, id));
        }
        for id in stored {
            bus.revoke_holder(holder(true, scope, u64::try_from(id).unwrap_or(0)));
        }
    }
}

/// "For this session" lasts while the program runs: when it has ended, the approval goes and so do the
/// streams it started.
fn end_sessions_of_ended_programs(cx: &mut App) {
    let scopes: Vec<u64> = state(cx).grants.keys().copied().collect();
    let mut changed = false;
    for scope in scopes {
        let dropped = state(cx)
            .grants_of(scope)
            .prune(Instant::now(), |process| process::start_time(process.pid) == Some(process.started));
        if dropped.is_empty() {
            continue;
        }
        changed = true;
        if let Some(bus) = crate::automation::bus(cx) {
            for id in dropped {
                bus.revoke_holder(holder(false, scope, id));
            }
        }
    }
    if changed {
        state(cx).listing = None;
        cx.refresh_windows();
    }
}
