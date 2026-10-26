//! `events`: a subscription to what happens in the app, answered with the state as of the moment it
//! starts and then a stream the socket thread writes (`wormhole_core::ipc::server`).
//!
//! The subscription is made in this call, on the UI thread, where everything that is published is
//! published too, so the state read here is exactly the one the first event follows.

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::ipc::bus::Grant;
use wormhole_core::ipc::control::event::{Subscribed, Topic};
use wormhole_core::ipc::control::{Error, ErrorCode, EventsSpec, Limited, Reason, Reply};
use wormhole_core::ipc::{Answer, Peer};

use super::{budget, listing};
use crate::app_state::app;
use crate::settings::settings;

/// Answer an `events` request. Order of the checks: the flag, the request itself, what it may
/// hear, the budget, then the stream caps.
pub(super) fn events(spec: &EventsSpec, from: Option<i64>, peer: &Peer, cx: &mut App) -> Answer {
    match subscribe(spec, from, peer, cx) {
        Ok(answer) => answer,
        Err(error) => Answer::Reply(error.into()),
    }
}

fn subscribe(spec: &EventsSpec, from: Option<i64>, peer: &Peer, cx: &mut App) -> Result<Answer, Error> {
    if !settings(cx).feature_enabled(FeatureFlag::Automation) {
        return Err(Error::new(ErrorCode::Unsupported, "automation is not in this build yet")
            .because(Reason::FlagOff));
    }
    let filter = spec.filter()?;
    if filter.topics.contains(Topic::Output) {
        return Err(Error::new(
            ErrorCode::Unsupported,
            "what terminals print can't be watched yet: the output topic comes in a later build",
        ));
    }
    let Some(bus) = crate::automation::bus(cx) else {
        return Err(Error::new(ErrorCode::Failed, "there is no event bus in this app"));
    };
    if let Err(refusal) = budget(Limited::Subscribe, from, cx) {
        return Err(refusal
            .error
            .unwrap_or_else(|| Error::new(ErrorCode::RateLimited, "too many streams lately")));
    }
    // What the caller may read of terminals' text: the ones it runs inside of. (The setting that
    // lets it read every terminal comes with reading itself.)
    let own: Vec<i64> = app(cx)
        .windows
        .iter()
        .filter_map(|window| {
            window.workspace.read_with(cx, |ws, cx| ws.control_terminals_under(peer, cx)).ok()
        })
        .flatten()
        .collect();
    let grant = Grant { reads_all: false, own };
    let (subscription, started) = bus
        .subscribe(filter.clone(), grant, from)
        .map_err(|refused| Error::new(ErrorCode::RateLimited, refused.to_string()))?;
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
