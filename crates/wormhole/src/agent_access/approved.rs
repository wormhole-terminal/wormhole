//! What the user has allowed, for Settings to list and take back.

use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::ipc::access::{Identity, Kind, Scope};

use super::gate::HOLDER_STORED;
use super::state;
use crate::app_state::{app, app_mut};

/// Which approval a row is: one held in memory, or one in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Which {
    Memory(u64),
    Stored(i64),
}

/// How long an approval lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum How {
    /// While the program's process runs.
    Session,
    /// Until Wormhole quits.
    UntilQuit,
    /// For good.
    Always,
}

/// One approved program, as Settings lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Approved {
    pub which: Which,
    pub label: String,
    pub how: How,
    pub kind: Kind,
    /// The registry agent it is, if it is one (its tile and name).
    pub agent: Option<AgentId>,
    /// Where it ran from, last time; empty when it was not noted.
    pub location: String,
    /// The signer's team id, for a Developer ID program.
    pub team: String,
    /// Seconds since the epoch.
    pub last_used: Option<i64>,
}

/// Every approval in force: the stored ones (most recently used first), then the ones held in memory.
pub(crate) fn approved(cx: &mut App) -> Vec<Approved> {
    let mut rows: Vec<Approved> = {
        let shown = app(cx);
        shown
            .store_of(shown.active_profile())
            .and_then(|store| store.agent_grants().ok())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| {
                let kind = Kind::from_word(&row.key.kind)?;
                Some(Approved {
                    which: Which::Stored(row.id),
                    label: row.label,
                    how: How::Always,
                    kind,
                    agent: Identity::from_key(&row.key).and_then(|identity| identity.agent),
                    location: row.location,
                    team: row.key.team_id,
                    last_used: row.last_used,
                })
            })
            .collect()
    };
    rows.extend(state(cx).grants.held().iter().map(|held| Approved {
        which: Which::Memory(held.id),
        label: held.label.clone(),
        how: match held.scope {
            Scope::Session(_) => How::Session,
            Scope::UntilQuit => How::UntilQuit,
        },
        kind: held.identity.kind,
        agent: held.identity.agent,
        location: String::new(),
        team: held.identity.team_id.clone(),
        last_used: None,
    }));
    rows
}

/// Take an approval back: the program is asked again, and the streams it started are closed.
pub(crate) fn revoke(which: Which, cx: &mut App) {
    let holder = match which {
        Which::Memory(id) => {
            state(cx).grants.revoke(id);
            id
        }
        Which::Stored(id) => {
            let profile = app(cx).active_profile().clone();
            if let Some(store) = app_mut(cx).store_of_mut(&profile)
                && let Err(err) = store.delete_agent_grant(id)
            {
                tracing::error!("couldn't take an approval back: {err:#}");
            }
            state(cx).touched.remove(&id);
            HOLDER_STORED | u64::try_from(id).unwrap_or(0)
        }
    };
    if let Some(bus) = crate::automation::bus(cx) {
        bus.revoke_holder(holder);
    }
    cx.refresh_windows();
}

/// Seconds since the epoch, for a row's "used … ago".
pub(crate) fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0))
}
