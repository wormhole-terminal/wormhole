//! `events`: a subscription to what happens in the app, answered with the state as of the moment it starts and
//! then a stream the socket thread writes (`wormhole_core::ipc::server`).
//!
//! The subscription and the listing it starts with are made against one look at the workspace
//! (`Publisher::listing_and_rebase`), so the state read here is exactly the one the first event follows.

use wormhole_core::ipc::access::Access;
use wormhole_core::ipc::bus::Grant;
use wormhole_core::ipc::control::event::{Subscribed, Topic};
use wormhole_core::ipc::control::{Error, ErrorCode, EventsSpec, Limited, Reply, reads_off};
use wormhole_core::ipc::{Answer, Peer};

use super::automation_off;
use crate::hook_server::core::Core;

/// Answer an `events` request. Order of the checks: the flag, the request itself, what it may hear, the
/// budget, then the stream caps.
pub(super) fn events(
    core: &Core,
    spec: &EventsSpec,
    from: Option<i64>,
    access: Access,
    readable: Option<Vec<i64>>,
    peer: &Peer,
) -> Answer {
    match subscribe(core, spec, from, access, readable, peer) {
        Ok(answer) => answer,
        Err(error) => Answer::Reply(error.into()),
    }
}

fn subscribe(
    core: &Core,
    spec: &EventsSpec,
    from: Option<i64>,
    access: Access,
    readable: Option<Vec<i64>>,
    peer: &Peer,
) -> Result<Answer, Error> {
    if let Some(off) = automation_off(core) {
        return Err(off);
    }
    let filter = spec.filter()?;
    if let Err(refusal) = core.budget(Limited::Subscribe, from) {
        return Err(refusal
            .error
            .unwrap_or_else(|| Error::new(ErrorCode::RateLimited, "too many streams lately")));
    }
    // What the caller may read of terminals' text: the ones it runs inside of, and every one while the
    // setting allows it. A program held to the terminals in its scope hears the text of those and no others
    // (`readable`, which the gate worked out): they are what it may read, as the terminals it runs in always are.
    let own = core.terminals_under(peer);
    let confined = readable.is_some();
    let reads_all = access.read && !confined;
    if filter.topics.contains(Topic::Output) && !confined {
        let only_own = !filter.terminals.is_empty() && filter.terminals.iter().all(|id| own.contains(id));
        if !reads_all && !only_own {
            return Err(reads_off());
        }
    }
    let grant = Grant { reads_all, own, readable: readable.unwrap_or_default(), holder: access.holder };
    let (subscription, started) = core
        .bus
        .subscribe(filter.clone(), grant, from)
        .map_err(|refused| Error::new(ErrorCode::RateLimited, refused.to_string()))?;
    let listing = core.publisher.listing_and_rebase();
    if filter.topics.contains(Topic::Output) {
        // Where each terminal's output starts from: its cursor now, so what is printed right after the reply is told.
        for terminal in listing.windows.iter().flat_map(|window| &window.terminals) {
            if let Some(reader) = core.terminal(terminal.id).and_then(|terminal| terminal.reader()) {
                core.bus.watch_output(core.window, terminal.id, &reader);
            }
        }
    }
    let subscribed = Subscribed {
        subscribed: true,
        seq: started.seq,
        boot: started.boot,
        topics: filter.topics.names(),
        listing,
    };
    tracing::info!(?from, topics = ?subscribed.topics, "control: an event stream began");
    Ok(Answer::Stream { reply: Reply::ok(subscribed), subscription })
}
