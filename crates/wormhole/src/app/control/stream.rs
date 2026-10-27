//! `events`: a subscription to what happens in the app, answered with the state as of the moment it
//! starts and then a stream the socket thread writes (`wormhole_core::ipc::server`).
//!
//! The subscription is made in this call, on the UI thread, where everything that is published is
//! published too, so the state read here is exactly the one the first event follows.

use gpui_kit::*;
use wormhole_core::ipc::access::Access;
use wormhole_core::ipc::bus::Grant;
use wormhole_core::ipc::control::event::{Subscribed, Topic};
use wormhole_core::ipc::control::{Error, ErrorCode, EventsSpec, Limited, Reply, reads_off};
use wormhole_core::ipc::{Answer, Peer};

use super::{automation_off, budget, listing};
use crate::app_state::{AppState, app};

/// Answer an `events` request. Order of the checks: the flag, the request itself, what it may
/// hear, the budget, then the stream caps.
pub(super) fn events(
    spec: &EventsSpec,
    from: Option<i64>,
    access: Access,
    readable: Option<Vec<i64>>,
    peer: &Peer,
    cx: &mut App,
) -> Answer {
    match subscribe(spec, from, access, readable, peer, cx) {
        Ok(answer) => answer,
        Err(error) => Answer::Reply(error.into()),
    }
}

fn subscribe(
    spec: &EventsSpec,
    from: Option<i64>,
    access: Access,
    readable: Option<Vec<i64>>,
    peer: &Peer,
    cx: &mut App,
) -> Result<Answer, Error> {
    if let Some(off) = automation_off(cx) {
        return Err(off);
    }
    let filter = spec.filter()?;
    let Some(bus) = crate::automation::bus(cx) else {
        return Err(Error::new(ErrorCode::Failed, "there is no event bus in this app"));
    };
    if let Err(refusal) = budget(Limited::Subscribe, from, cx) {
        return Err(refusal
            .error
            .unwrap_or_else(|| Error::new(ErrorCode::RateLimited, "too many streams lately")));
    }
    // The windows tell what changed in a terminal when they next look; look now, so that what the
    // snapshot shows and what the stream goes on to say don't both carry the same change.
    for window in app(cx).windows.clone() {
        AppState::with_workspace(&window, cx, |ws, _, cx| ws.sync_summaries(cx));
    }
    // What the caller may read of terminals' text: the ones it runs inside of, and every one while the
    // setting allows it.
    let own: Vec<i64> = app(cx)
        .windows
        .iter()
        .filter_map(|window| {
            window.workspace.read_with(cx, |ws, cx| ws.control_terminals_under(peer, cx)).ok()
        })
        .flatten()
        .collect();
    // A program that is held to the terminals in its scope hears the text of those and no others (`readable`,
    // which the gate worked out): they are what it may read, as the terminals it runs in always are.
    let confined = readable.is_some();
    let readable = readable.unwrap_or_default();
    let reads_all = access.read && !confined;
    // What a terminal prints is for those who may read it: the setting, or the terminals the caller
    // runs in and named (the same rule that decides which streams the setting going off ends).
    if filter.topics.contains(Topic::Output) && !confined {
        let only_own = !filter.terminals.is_empty() && filter.terminals.iter().all(|id| own.contains(id));
        if !reads_all && !only_own {
            return Err(reads_off());
        }
    }
    let grant = Grant { reads_all, own, readable, holder: access.holder };
    let (subscription, started) = bus
        .subscribe(filter.clone(), grant, from)
        .map_err(|refused| Error::new(ErrorCode::RateLimited, refused.to_string()))?;
    if filter.topics.contains(Topic::Output) {
        // Where each terminal's output starts from: its cursor now, so what is printed right after
        // the reply is told.
        for window in app(cx).windows.clone() {
            let readers = window.workspace.read_with(cx, |ws, cx| ws.control_readers(cx)).unwrap_or_default();
            for facts in readers {
                bus.watch_output(window.id, facts.terminal, &facts.reader);
            }
        }
    }
    let subscribed = Subscribed {
        subscribed: true,
        seq: started.seq,
        boot: started.boot,
        topics: filter.topics.names(),
        listing: listing(cx),
    };
    tracing::info!(?from, topics = ?subscribed.topics, "control: an event stream began");
    Ok(Answer::Stream { reply: Reply::ok(subscribed), subscription })
}
