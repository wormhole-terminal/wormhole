//! What a test instance's control folder does to the desk without a question (`devctl/agents.rs`
//! `agent-grant`): let Scripts in, so a scenario can start approved. Nothing here is reachable from a
//! request: only the app's control folder calls it, as `close-answer` answers the sheet before closing.

use std::time::{SystemTime, UNIX_EPOCH};

use wormhole_core::ipc::access::{CapSet, Identity, Scope};
use wormhole_core::storage::store::{AgentGrantRow, Store};

use super::AccessDesk;
use crate::aux_backends::common::locked;

/// What Scripts is called in the approvals (`agent-grant`'s label in the GPUI app).
const SCRIPTS_LABEL: &str = "Scripts";

#[uniffi::export]
impl AccessDesk {
    /// Lets Scripts in as an answer to its question would: until Wormhole quits, or for good (`always`, a
    /// stored approval of this desk's profile). A named agent is told apart by its signature, which a
    /// scenario cannot know beforehand: its question is answered. Whether it was granted.
    pub fn grant_scripts(&self, always: bool) -> bool {
        let identity = Identity::scripts();
        if always {
            let created = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0));
            let row = AgentGrantRow {
                key: identity.key(),
                label: SCRIPTS_LABEL.into(),
                caps: CapSet::STANDARD.bits(),
                created,
                ..AgentGrantRow::default()
            };
            let saved = Store::open_at(&self.profile_dir).and_then(|mut store| store.save_agent_grant(&row));
            if let Err(err) = &saved {
                tracing::error!("couldn't store an approval: {err:#}");
            }
            saved.is_ok()
        } else {
            locked(&self.inner).grants.grant(
                identity,
                SCRIPTS_LABEL.into(),
                Scope::UntilQuit,
                CapSet::STANDARD,
            );
            self.waker.notify();
            true
        }
    }
}
