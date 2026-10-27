//! The questions: which is on the screen, what an answer does, and what happens when nobody answers.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::ipc::access::{CapSet, Held, Scope};
use wormhole_core::ipc::control::{ASK_WAIT_S, Error, ErrorCode, Reason, Reply};
use wormhole_core::ipc::{Decision, access};
use wormhole_core::platform::process;
use wormhole_core::storage::store::AgentGrantRow;

use super::gate::{HOLDER_STORED, lets_in};
use super::scripted::{Asker, Script};
use super::sheet::{self, Button};
use super::{Shown, state};
use crate::app_state::{AppState, app, app_mut};
use crate::settings::settings;

/// How a question ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// The user pressed a button.
    Chose(Button),
    /// Nobody answered in time: no, and not remembered.
    TimedOut,
    /// The sheet went without an answer (its window closed): no, and not remembered.
    Withdrawn,
}

/// Put the next question on the screen, if none is. Called when one is queued, when one is answered and
/// on the tick (a window may have opened since).
pub(super) fn pump(cx: &mut App) {
    let Some(id) = state(cx).queue.next_to_show().map(|pending| pending.id) else { return };
    let display = {
        let held = cx.global::<super::State>();
        match held.queue.iter().find(|pending| pending.id == id) {
            Some(pending) => sheet::display(pending, cx),
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
        Asker::Sheet => show(id, display, cx),
    }
}

/// The button a script's wish comes to when the question does not offer it: the longest allowance it does.
fn offered(buttons: &[Button], wish: Button) -> Button {
    if buttons.contains(&wish) {
        return wish;
    }
    match wish {
        Button::Deny => Button::Deny,
        Button::Session | Button::Always => {
            if buttons.contains(&Button::Quit) {
                Button::Quit
            } else {
                Button::Deny
            }
        }
        Button::Quit => Button::Deny,
    }
}

fn show(id: u64, display: sheet::Display, cx: &mut App) {
    let Some(window) =
        AppState::front_window(cx).or_else(|| app(cx).windows.iter().find(|w| !w.hidden).cloned())
    else {
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

/// A question that nobody answers counts as no after a minute, and is not remembered.
fn start_timer(id: u64, cx: &mut App) {
    cx.spawn(async move |cx| {
        cx.background_executor().timer(Duration::from_secs(ASK_WAIT_S)).await;
        cx.update(|cx| settle(id, Outcome::TimedOut, cx));
    })
    .detach();
}

/// End question `id` with `outcome`: record what it means, tell every request that waited, take the sheet
/// off the screen and put the next question up.
fn settle(id: u64, outcome: Outcome, cx: &mut App) {
    let Some(pending) = state(cx).queue.take(id) else { return };
    if let Some(Shown { window: Some(window), .. }) = state(cx).shown.take_if(|shown| shown.id == id) {
        sheet::withdraw(&window, cx);
    }
    let resolved = pending.resolved.clone();
    let label = resolved.label.clone();
    match outcome {
        Outcome::Chose(Button::Deny) => {
            state(cx).grants.deny(resolved.identity, Instant::now());
            pending.answer(|| Decision::Refuse(declined(&label).into()));
        }
        Outcome::Chose(button) => {
            allow(&resolved, button, cx);
            pending.answer(|| Decision::Retry);
        }
        Outcome::TimedOut => pending.answer(|| Decision::Refuse(timed_out(&label).into())),
        Outcome::Withdrawn => pending.answer(|| Decision::Refuse(declined(&label).into())),
    }
    // Settings lists what is allowed.
    cx.refresh_windows();
    pump(cx);
}

/// Remember that `resolved` was let in, as long as `button` says.
fn allow(resolved: &access::Resolved, button: Button, cx: &mut App) {
    let caps = CapSet::STANDARD;
    let label = resolved.label.clone();
    let identity = resolved.identity.clone();
    match (button, resolved.process) {
        (Button::Session, Some(process)) => {
            state(cx).grants.grant(identity, label, Scope::Session(process), caps);
        }
        (Button::Always, _) if identity.can_be_remembered() => {
            if !store_always(resolved, caps, cx) {
                // The database would not take it: it holds until Wormhole quits, which the user can see.
                state(cx).grants.grant(identity, label, Scope::UntilQuit, caps);
            }
        }
        _ => {
            state(cx).grants.grant(identity, label, Scope::UntilQuit, caps);
        }
    }
}

fn store_always(resolved: &access::Resolved, caps: CapSet, cx: &mut App) -> bool {
    let profile = app(cx).active_profile().clone();
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

fn timed_out(label: &str) -> Error {
    Error::new(
        ErrorCode::NotAllowed,
        format!("nobody answered the question about {label} in Wormhole within a minute; it counts as no, and is not remembered"),
    )
    .because(Reason::AskTimedOut)
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
    /// What the sheet says: its title, its text and the block of facts under it.
    pub title: String,
    pub detail: String,
    pub facts: String,
}

#[cfg(any(test, feature = "devctl"))]
/// The questions waiting for an answer, the one on the screen first. `words` also says what the sheet says
/// (title, text, facts), which reads the windows: not from inside the update of one.
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
                shown: Some(pending.id) == shown,
                buttons: sheet::buttons(pending),
                title: said.as_ref().map(|said| said.title.clone()).unwrap_or_default(),
                detail: said.as_ref().map(|said| said.detail.clone()).unwrap_or_default(),
                facts: said.map(|said| said.accessory).unwrap_or_default(),
            }
        })
        .collect();
    waiting.sort_by_key(|question| !question.shown);
    waiting
}

#[cfg(any(test, feature = "devctl"))]
/// Answer the question on the screen with `button` (devctl's `agent-ask`, a test): as if it was pressed, a
/// moment later, outside the update that asked. Whether there was one.
pub(crate) fn answer_pending(button: Button, cx: &mut App) -> bool {
    let Some(id) = state(cx).shown.as_ref().map(|shown| shown.id) else { return false };
    let Some(pending) = cx.global::<super::State>().queue.iter().find(|pending| pending.id == id) else {
        return false;
    };
    let button = offered(&sheet::buttons(pending), button);
    cx.defer(move |cx| settle(id, Outcome::Chose(button), cx));
    true
}

/// What the app does about this on its tick: refuse the questions that are waiting when nothing is let in
/// any more, end the streams that were started under approvals when the mode goes off, forget the approvals
/// of programs that ended, and put a question up that was waiting for a window.
pub(crate) fn tick(ticks: u64, cx: &mut App) {
    let settings = settings(cx);
    let mode = settings.feature_enabled(FeatureFlag::Automation).then_some(settings.agent_access);
    let was = std::mem::replace(&mut state(cx).mode, mode);
    let nothing_gets_in = mode.is_none_or(|mode| !lets_in(mode));
    if nothing_gets_in && was != mode {
        end_streams_under_approvals(cx);
    }
    if nothing_gets_in && !state(cx).queue.is_empty() {
        refuse_waiting_questions(cx);
    }
    if ticks.is_multiple_of(5) {
        end_sessions_of_ended_programs(cx);
    }
    pump(cx);
}

/// Everything waiting is refused: access was switched off while the user had not answered.
fn refuse_waiting_questions(cx: &mut App) {
    let questions = std::mem::take(&mut state(cx).queue).drain();
    if let Some(Shown { window: Some(window), .. }) = state(cx).shown.take() {
        sheet::withdraw(&window, cx);
    }
    for pending in questions {
        pending.answer(|| Decision::Refuse(access_off()));
    }
}

/// What was allowed no longer reaches the streams it started.
fn end_streams_under_approvals(cx: &mut App) {
    let held: Vec<u64> = state(cx).grants.held().iter().map(|held: &Held| held.id).collect();
    let stored: Vec<i64> = approved_rows(cx);
    if let Some(bus) = crate::automation::bus(cx) {
        for id in held {
            bus.revoke_holder(id);
        }
        for id in stored {
            bus.revoke_holder(HOLDER_STORED | u64::try_from(id).unwrap_or(0));
        }
    }
}

fn approved_rows(cx: &App) -> Vec<i64> {
    let state = app(cx);
    state
        .store_of(state.active_profile())
        .and_then(|store| store.agent_grants().ok())
        .map(|rows| rows.into_iter().map(|row| row.id).collect())
        .unwrap_or_default()
}

/// "For this session" lasts while the program runs: when it has ended, the approval goes and so do the
/// streams it started.
fn end_sessions_of_ended_programs(cx: &mut App) {
    if state(cx).grants.held().is_empty() {
        return;
    }
    let dropped = state(cx)
        .grants
        .prune(Instant::now(), |process| process::start_time(process.pid) == Some(process.started));
    if dropped.is_empty() {
        return;
    }
    if let Some(bus) = crate::automation::bus(cx) {
        for id in dropped {
            bus.revoke_holder(id);
        }
    }
    cx.refresh_windows();
}
