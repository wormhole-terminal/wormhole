//! Who may control Wormhole through the automation API, and the question that asks
//! (`../repo-docs/docs/agent-permissions.md`, the connection approval).
//!
//! A control request that needs more than titles and states is judged here before any handler sees it
//! (`gate`): the mode in Settings, then, in Ask mode, the program behind the `wh` (its process chain and
//! signature: core's `ipc::access`), then what the user allowed it. A program nothing covers gets a
//! question (`ask`, `sheet`): a sheet on the front window, queued one at a time, whose answer is kept for
//! this run of the program, until Wormhole quits, or for good (`approved`, the store's table). The socket
//! thread never waits for the user: the request waits on a thread of its own (`AskJob`).
//!
//! No control request can change anything held here: the handlers never see this module's state, and a
//! test walks every action. Answers come from the user, on a sheet or in Settings, and from devctl in a
//! test instance, which never shows a sheet.
//!
//! A part that owns its own once-only state keeps it in a global of its own: [`State`]. Above `features`
//! and below `settings_window`, which lists and revokes what is allowed, and `app`, which calls the gate.

mod approved;
mod ask;
mod change;
mod drive;
mod gate;
mod processes;
mod profile;
mod scripted;
mod sheet;

use std::collections::HashMap;
use std::time::{Instant, SystemTime};

use gpui_kit::*;
use wormhole_core::config::AgentAccess;
use wormhole_core::ipc::access::{Grants, Queue, Resolved};
use wormhole_core::storage::paths::ProfileId;

use self::approved::Listing;
use self::processes::Signatures;
use self::scripted::Asker;
use crate::app_state::OpenWindow;

#[cfg(any(test, feature = "devctl"))]
pub(crate) use self::approved::Which;
pub(crate) use self::approved::{Approved, How, approved, forget_listing, now_seconds, revoke};
#[cfg(test)]
pub(crate) use self::ask::Waiting;
#[cfg(any(test, feature = "devctl"))]
pub(crate) use self::ask::{answer_pending, pending_questions};
pub(crate) use self::ask::{reads_open, tick};
pub(crate) use self::drive::{announce, drive_of};
pub(crate) use self::gate::{Gate, gate, standing};
#[cfg(any(test, feature = "devctl"))]
pub(crate) use self::scripted::Script;
#[cfg(any(test, feature = "devctl"))]
pub(crate) use self::sheet::Button;

/// The question on the screen. A question about a change of the mode (`change`) has the id [`CHANGE_ID`], which
/// no queued question has.
struct Shown {
    id: u64,
    /// The window it is a sheet on, when it is a sheet (a script's question is on none).
    window: Option<OpenWindow>,
}

/// The id of the question about a mode that was changed outside Wormhole.
const CHANGE_ID: u64 = 0;

/// What this part keeps between requests.
struct State {
    /// What was allowed for a session or until Wormhole quits, by profile (a profile decides for itself).
    grants: HashMap<u64, Grants>,
    queue: Queue,
    shown: Option<Shown>,
    asker: Asker,
    signatures: Signatures,
    /// The profile each scope stands for.
    profiles: HashMap<u64, ProfileId>,
    /// The mode in force in each profile. It follows the settings, except for a loosening that did not come
    /// from Settings (`change`).
    enforced: HashMap<u64, AgentAccess>,
    /// How many edits from inside Wormhole each profile's settings had when the mode was last looked at.
    seen_edits: HashMap<u64, u64>,
    /// The mode each profile's questions and streams were last settled for (`ask::tick`).
    applied: HashMap<u64, Option<AgentAccess>>,
    /// A mode changed outside Wormhole that widens access, waiting for the user to keep it or not.
    change: Option<change::Change>,
    /// When each stored approval was last noted as used (`(scope, row)`), and how many uses are not written yet:
    /// the database is written at most once a minute for an approval, with the total.
    touched: HashMap<(u64, i64), Instant>,
    uses: HashMap<(u64, i64), i64>,
    /// Who was found behind a process lately: `wh mcp` asks again and again from one process.
    identities: Vec<(u32, SystemTime, Instant, Resolved)>,
    /// What Settings lists, kept until something changes it (`approved`).
    listing: Option<Listing>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            grants: HashMap::new(),
            queue: Queue::default(),
            shown: None,
            asker: Asker::detect(),
            signatures: Signatures::default(),
            profiles: HashMap::new(),
            enforced: HashMap::new(),
            seen_edits: HashMap::new(),
            applied: HashMap::new(),
            change: None,
            touched: HashMap::new(),
            uses: HashMap::new(),
            identities: Vec::new(),
            listing: None,
        }
    }
}

impl Global for State {}

fn state(cx: &mut App) -> &mut State {
    cx.default_global::<State>()
}

impl State {
    /// What is allowed in `scope` for a while.
    fn grants_of(&mut self, scope: u64) -> &mut Grants {
        self.grants.entry(scope).or_default()
    }
}

/// Make `script` answer the questions from now on (a test; devctl's `agent-script`).
#[cfg(any(test, feature = "devctl"))]
pub(crate) fn set_script(script: Script, cx: &mut App) {
    state(cx).asker = Asker::Scripted(script);
}

/// Let `identity` in until Wormhole quits, as the second button of its question does (devctl).
#[cfg(feature = "devctl")]
pub(crate) fn grant_until_quit(identity: wormhole_core::ipc::access::Identity, label: &str, cx: &mut App) {
    use wormhole_core::ipc::access::{CapSet, Scope};
    let scope = profile::scope_of(&crate::app_state::app(cx).active_profile().clone());
    state(cx).grants_of(scope).grant(identity, label.to_string(), Scope::UntilQuit, CapSet::STANDARD);
    state(cx).listing = None;
}

/// What the tests that drive the control handlers share: letting Scripts in as much as a test wants, and
/// answering questions the way a test says.
#[cfg(test)]
pub(crate) mod testing {
    use gpui_kit::App;
    use gpui_kit::TestAppContext;
    use wormhole_core::config::AgentAccess;
    use wormhole_core::ipc::access::{CapSet, Capability, Identity, Scope};

    use super::profile::{holder, scope_of};
    use super::{Script, state};
    use crate::app_state::app;
    use crate::settings::update_settings;

    /// Ask mode in the shown profile, with Scripts (which every caller of a test is, having no agent above it)
    /// allowed to read and/or to type, run and open, until Wormhole quits; with neither, nothing is allowed and
    /// a question is answered "Don't Allow" at once.
    pub(crate) fn allow(cx: &mut TestAppContext, read: bool, write: bool) {
        cx.update(|cx| allow_in(cx, read, write));
    }

    pub(crate) fn allow_in(cx: &mut App, read: bool, write: bool) {
        update_settings(cx, |settings| settings.agent_access = AgentAccess::Ask);
        let scope = scope_of(&app(cx).active_profile().clone());
        // What was allowed before is taken back, and the streams it started end with it.
        let before: Vec<u64> = state(cx).grants_of(scope).held().iter().map(|held| held.id).collect();
        if let Some(bus) = crate::automation::bus(cx) {
            for id in before {
                bus.revoke_holder(holder(false, scope, id));
            }
        }
        state(cx).listing = None;
        let grants = state(cx).grants_of(scope);
        *grants = wormhole_core::ipc::access::Grants::default();
        let mut caps = CapSet::EMPTY;
        if read {
            caps = caps.with(Capability::Read);
        }
        if write {
            caps = caps.with(Capability::Type).with(Capability::Run).with(Capability::Open);
        }
        if caps != CapSet::EMPTY {
            grants.grant(Identity::scripts(), "Scripts".into(), Scope::UntilQuit, caps);
        }
    }

    /// "For this session" for a coding agent, tied to a process that does not exist, so that the next sweep
    /// of ended programs drops it.
    pub(crate) fn grant_session_of_a_gone_process(cx: &mut TestAppContext) -> u64 {
        use wormhole_core::agents::AgentId;
        use wormhole_core::ipc::access::{Kind, ProcessRef};
        cx.update(|cx| {
            let process = ProcessRef { pid: u32::MAX, started: std::time::SystemTime::UNIX_EPOCH };
            let agent = Identity { kind: Kind::AdHoc, agent: Some(AgentId::Claude), ..Identity::scripts() };
            let scope = scope_of(&app(cx).active_profile().clone());
            state(cx).listing = None;
            state(cx).grants_of(scope).grant(
                agent,
                "Claude Code".into(),
                Scope::Session(process),
                CapSet::STANDARD,
            )
        })
    }

    /// Nobody answers a question until the test does.
    pub(crate) fn hold(cx: &mut TestAppContext) {
        cx.update(|cx| super::set_script(Script::Hold, cx));
    }

    /// The question on the screen goes away without an answer, as when its window closes.
    pub(crate) fn withdraw(cx: &mut TestAppContext) -> bool {
        let withdrawn = cx.update(super::ask::withdraw_pending);
        cx.run_until_parked();
        withdrawn
    }

    /// `gate` with who asks given instead of found.
    pub(crate) fn gate_as(
        request: &wormhole_core::ipc::control::Request,
        who: &dyn Fn() -> Result<wormhole_core::ipc::access::Resolved, wormhole_core::ipc::access::Changed>,
        cx: &mut TestAppContext,
    ) -> super::Gate {
        cx.update(|cx| {
            super::gate::gate_with(request, &wormhole_core::ipc::Peer::default(), &|_, _| who(), cx)
        })
    }
}
