//! What the user has allowed, for Settings to list and take back.

use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::ipc::access::{Identity, Kind, Scope};

use super::profile::{holder, scope_of};
use super::state;
use crate::app_state::{app, app_mut};
use wormhole_text::keys::AGENT_SCRIPTS_LABEL;
use wormhole_text::text;

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

/// What Settings lists, for the profile it is of, until something changes it.
pub(super) struct Listing {
    scope: u64,
    rows: Vec<Approved>,
}

/// Every approval in force in the shown profile (what its Settings are about): the stored ones (most recently
/// used first), then the ones held in memory. Read from the database when the list is first wanted and after
/// something changed it, not at every draw.
pub(crate) fn approved(cx: &mut App) -> Vec<Approved> {
    let profile = app(cx).active_profile().clone();
    let scope = scope_of(&profile);
    if let Some(listing) = state(cx).listing.as_ref().filter(|listing| listing.scope == scope) {
        return listing.rows.clone();
    }
    let mut rows: Vec<Approved> = app(cx)
        .store_of(&profile)
        .and_then(|store| store.agent_grants().ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let kind = Kind::from_word(&row.key.kind)?;
            Some(Approved {
                which: Which::Stored(row.id),
                label: label_of(kind, row.label),
                how: How::Always,
                kind,
                agent: Identity::from_key(&row.key).and_then(|identity| identity.agent),
                location: row.location,
                team: row.key.team_id,
                last_used: row.last_used,
            })
        })
        .collect();
    rows.extend(state(cx).grants_of(scope).held().iter().map(|held| Approved {
        which: Which::Memory(held.id),
        label: label_of(held.identity.kind, held.label.clone()),
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
    state(cx).listing = Some(Listing { scope, rows: rows.clone() });
    rows
}

/// What a row is called: the program's own name, or, for the one identity that is everything else, the
/// catalog's word for it (the name that was stored is English).
fn label_of(kind: Kind, stored: String) -> String {
    if kind == Kind::Scripts { text(AGENT_SCRIPTS_LABEL).to_string() } else { stored }
}

/// The list is read again at the next look: the database may have changed under it (a use was noted, another
/// window). Settings does this when it opens.
pub(crate) fn forget_listing(cx: &mut App) {
    state(cx).listing = None;
}

/// Take an approval back: the program is asked again, and the streams it started are closed.
pub(crate) fn revoke(which: Which, cx: &mut App) {
    let profile = app(cx).active_profile().clone();
    let scope = scope_of(&profile);
    let holder = match which {
        Which::Memory(id) => {
            state(cx).grants_of(scope).revoke(id);
            holder(false, scope, id)
        }
        Which::Stored(id) => {
            if let Some(store) = app_mut(cx).store_of_mut(&profile)
                && let Err(err) = store.delete_agent_grant(id)
            {
                tracing::error!("couldn't take an approval back: {err:#}");
            }
            state(cx).touched.remove(&(scope, id));
            state(cx).uses.remove(&(scope, id));
            holder(true, scope, u64::try_from(id).unwrap_or(0))
        }
    };
    state(cx).listing = None;
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
