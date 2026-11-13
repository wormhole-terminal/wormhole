//! The approvals in force, for the list in Settings → Agent access (`settings_window/sections/agent_access.rs`
//! and `agent_access/approved.rs` of the GPUI app): the ones stored in the profile's database, most recently
//! used first, then the ones held in memory. Taking one back is `revoke` for a held one and `revoke_stored`
//! for a stored one; the server ends the streams either let in (`HookServer::revoke_approval`).

use wormhole_core::ipc::access::{CapSet, Capability, Kind, Scope};
use wormhole_core::storage::store::{AgentGrantRow, Store};
use wormhole_core::view::sidebar::wording;
use wormhole_text::keys::{
    AGENT_SCRIPTS_LABEL, SETTINGS_AGENTS_ACCESS_AD_HOC, SETTINGS_AGENTS_ACCESS_ALWAYS,
    SETTINGS_AGENTS_ACCESS_NEVER_USED, SETTINGS_AGENTS_ACCESS_NOT_SIGNED, SETTINGS_AGENTS_ACCESS_SESSION,
    SETTINGS_AGENTS_ACCESS_UNTIL_QUIT, agent_sheet_signed_team, settings_agents_access_last_used,
};
use wormhole_text::text;

use super::{AccessDesk, locked};
use crate::aux_backends::access_records::AccessCapability;

/// How long an approval lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum AccessHow {
    /// While the program's process runs.
    Session,
    /// Until Wormhole quits.
    UntilQuit,
    /// For good, in the profile's database.
    Always,
}

/// One approved program, as Settings lists it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AccessApprovalRecord {
    /// A stored approval's row id, or a held one's number: `revoke_stored` or `revoke` takes it.
    pub id: u64,
    pub stored: bool,
    pub label: String,
    pub how: AccessHow,
    /// The registry agent it is, if it is one (`claude`): its tile and name.
    pub agent: Option<String>,
    /// How long, who signed it, when it was last used: one line.
    pub detail: String,
    /// Where it ran from, last time; empty when it was not noted.
    pub location: String,
    /// What it may do, in the catalog's words, one per capability it has.
    pub allowed: Vec<String>,
    /// The same as data, for the ticks of its row: Read, Type, Run and Open, the ones it has. Closing always asks.
    pub capabilities: Vec<AccessCapability>,
}

fn capabilities(caps: CapSet) -> Vec<AccessCapability> {
    [
        (Capability::Read, AccessCapability::Read),
        (Capability::Type, AccessCapability::Type),
        (Capability::Run, AccessCapability::Run),
        (Capability::Open, AccessCapability::Open),
    ]
    .into_iter()
    .filter(|(core, _)| caps.contains(*core))
    .map(|(_, record)| record)
    .collect()
}

/// The core's capability a tick stands for; closing is not a tick, so it is none.
fn core_capability(capability: AccessCapability) -> Option<Capability> {
    match capability {
        AccessCapability::Read => Some(Capability::Read),
        AccessCapability::Type => Some(Capability::Type),
        AccessCapability::Run => Some(Capability::Run),
        AccessCapability::Open => Some(Capability::Open),
        AccessCapability::Close => None,
    }
}

const SHOWN: [Capability; 4] = [Capability::Read, Capability::Type, Capability::Run, Capability::Open];

fn capability_word(capability: Capability) -> &'static str {
    use wormhole_text::keys::{
        SETTINGS_AGENTS_ACCESS_CAP_OPEN, SETTINGS_AGENTS_ACCESS_CAP_READ, SETTINGS_AGENTS_ACCESS_CAP_RUN,
        SETTINGS_AGENTS_ACCESS_CAP_TYPE,
    };
    text(match capability {
        Capability::Read => SETTINGS_AGENTS_ACCESS_CAP_READ,
        Capability::Type => SETTINGS_AGENTS_ACCESS_CAP_TYPE,
        Capability::Run => SETTINGS_AGENTS_ACCESS_CAP_RUN,
        Capability::Open | Capability::Close => SETTINGS_AGENTS_ACCESS_CAP_OPEN,
    })
}

fn label_of(kind: Kind, stored: String) -> String {
    if kind == Kind::Scripts { text(AGENT_SCRIPTS_LABEL).to_string() } else { stored }
}

fn detail(how: AccessHow, kind: Kind, team: &str, last_used: Option<i64>, now: i64) -> String {
    let mut parts = vec![
        text(match how {
            AccessHow::Session => SETTINGS_AGENTS_ACCESS_SESSION,
            AccessHow::UntilQuit => SETTINGS_AGENTS_ACCESS_UNTIL_QUIT,
            AccessHow::Always => SETTINGS_AGENTS_ACCESS_ALWAYS,
        })
        .to_string(),
    ];
    match kind {
        Kind::DeveloperId if !team.is_empty() => parts.push(agent_sheet_signed_team(team)),
        Kind::AdHoc => parts.push(text(SETTINGS_AGENTS_ACCESS_AD_HOC).to_string()),
        Kind::Unsigned => parts.push(text(SETTINGS_AGENTS_ACCESS_NOT_SIGNED).to_string()),
        Kind::DeveloperId | Kind::Apple | Kind::Script | Kind::Scripts => {}
    }
    if how == AccessHow::Always {
        parts.push(match last_used {
            Some(at) => {
                let ago = wording::ago(std::time::Duration::from_secs(u64::try_from(now - at).unwrap_or(0)));
                settings_agents_access_last_used(&ago)
            }
            None => text(SETTINGS_AGENTS_ACCESS_NEVER_USED).to_string(),
        });
    }
    parts.join(" \u{b7} ")
}

fn stored_record(row: AgentGrantRow, now: i64) -> Option<AccessApprovalRecord> {
    let kind = Kind::from_word(&row.key.kind)?;
    let caps = CapSet::from_bits(row.caps);
    Some(AccessApprovalRecord {
        id: u64::try_from(row.id).unwrap_or(0),
        stored: true,
        label: label_of(kind, row.label),
        how: AccessHow::Always,
        agent: wormhole_core::ipc::access::Identity::from_key(&row.key)
            .and_then(|identity| identity.agent)
            .map(wormhole_core::ipc::access::agent_word),
        detail: detail(AccessHow::Always, kind, &row.key.team_id, row.last_used, now),
        location: row.location,
        allowed: SHOWN
            .iter()
            .filter(|cap| caps.contains(**cap))
            .map(|cap| capability_word(*cap).to_string())
            .collect(),
        capabilities: capabilities(caps),
    })
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0))
}

#[uniffi::export]
impl AccessDesk {
    /// Every approval in force: the stored ones (most recently used first), then the ones held in memory.
    pub fn approvals(&self) -> Vec<AccessApprovalRecord> {
        let now = now();
        let mut rows: Vec<AccessApprovalRecord> = super::stored_rows(&self.profile_dir)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| stored_record(row, now))
            .collect();
        rows.extend(locked(&self.inner).grants.held().iter().map(|held| {
            let how = match held.scope {
                Scope::Session(_) => AccessHow::Session,
                Scope::UntilQuit => AccessHow::UntilQuit,
            };
            AccessApprovalRecord {
                id: held.id,
                stored: false,
                label: label_of(held.identity.kind, held.label.clone()),
                how,
                agent: held.identity.agent.map(wormhole_core::ipc::access::agent_word),
                detail: detail(how, held.identity.kind, &held.identity.team_id, None, now),
                location: String::new(),
                allowed: SHOWN
                    .iter()
                    .filter(|cap| held.caps.contains(**cap))
                    .map(|cap| capability_word(*cap).to_string())
                    .collect(),
                capabilities: capabilities(held.caps),
            }
        }));
        rows
    }

    /// Takes stored approval `id` (a database row) back: the program is asked again. Whether there was one.
    pub fn revoke_stored(&self, id: u64) -> bool {
        let Ok(id) = i64::try_from(id) else { return false };
        if !self.profile_dir.join("wormhole.db").is_file() {
            return false;
        }
        let Ok(mut store) = Store::open_at(&self.profile_dir) else { return false };
        let removed = store.delete_agent_grant(id).unwrap_or(false);
        if removed {
            self.waker.notify();
        }
        removed
    }

    /// Switches one capability of approval `id` (`stored`: a database row; else one held in memory) on or off, as the
    /// ticks of its row in Settings do. Whether the approval changed (closing is never a tick; neither is an unknown id).
    pub fn set_capability(&self, id: u64, stored: bool, capability: AccessCapability, on: bool) -> bool {
        let Some(capability) = core_capability(capability) else { return false };
        let change = |caps: CapSet| if on { caps.with(capability) } else { caps.without(capability) };
        if !stored {
            let mut inner = locked(&self.inner);
            let Some(caps) = inner.grants.held().iter().find(|held| held.id == id).map(|held| held.caps)
            else {
                return false;
            };
            return inner.grants.set_caps(id, change(caps));
        }
        let Ok(row_id) = i64::try_from(id) else { return false };
        if !self.profile_dir.join("wormhole.db").is_file() {
            return false;
        }
        let Ok(mut store) = Store::open_at(&self.profile_dir) else { return false };
        let Some(caps) = store
            .agent_grants()
            .ok()
            .and_then(|rows| rows.into_iter().find(|row| row.id == row_id))
            .map(|row| CapSet::from_bits(row.caps))
        else {
            return false;
        };
        let saved = store.set_agent_grant_caps(row_id, change(caps).bits()).is_ok();
        if saved {
            self.waker.notify();
        }
        saved
    }
}
