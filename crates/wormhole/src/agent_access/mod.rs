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
mod drive;
mod gate;
mod processes;
mod scripted;
mod sheet;

use std::collections::HashMap;
use std::time::Instant;

use gpui_kit::*;
use wormhole_core::config::AgentAccess;
use wormhole_core::ipc::access::{Grants, Queue};

use self::processes::Signatures;
use self::scripted::Asker;
use crate::app_state::OpenWindow;

pub(crate) use self::approved::{Approved, How, Which, approved, now_seconds, revoke};
#[cfg(test)]
pub(crate) use self::ask::Waiting;
pub(crate) use self::ask::{answer_pending, pending_questions, tick};
pub(crate) use self::drive::{announce, drive_of};
pub(crate) use self::gate::{Gate, gate, standing};
#[cfg(any(test, feature = "devctl"))]
pub(crate) use self::scripted::Script;
pub(crate) use self::sheet::Button;

/// The question on the screen.
struct Shown {
    id: u64,
    /// The window it is a sheet on, when it is a sheet (a script's question is on none).
    window: Option<OpenWindow>,
}

/// What this part keeps between requests.
struct State {
    grants: Grants,
    queue: Queue,
    shown: Option<Shown>,
    asker: Asker,
    signatures: Signatures,
    /// When each stored approval was last noted as used: once a minute at most, so the database is not
    /// written for every request.
    touched: HashMap<i64, Instant>,
    /// The mode the last tick saw, to notice when it changes.
    mode: Option<AgentAccess>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            grants: Grants::default(),
            queue: Queue::default(),
            shown: None,
            asker: Asker::detect(),
            signatures: Signatures::default(),
            touched: HashMap::new(),
            mode: None,
        }
    }
}

impl Global for State {}

fn state(cx: &mut App) -> &mut State {
    cx.default_global::<State>()
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
    state(cx).grants.grant(identity, label.to_string(), Scope::UntilQuit, CapSet::STANDARD);
}

/// What the tests that drive the control handlers share: letting Scripts in as much as a test wants, and
/// answering questions the way a test says.
#[cfg(test)]
pub(crate) mod testing {
    use gpui_kit::App;
    use gpui_kit::TestAppContext;
    use wormhole_core::config::AgentAccess;
    use wormhole_core::ipc::access::{CapSet, Capability, Identity, Scope};

    use super::{Script, state};
    use crate::settings::update_settings;

    /// Ask mode, with Scripts (which every caller of a test is, having no agent above it) allowed to read
    /// and/or to type, run and open, until Wormhole quits; with neither, nothing is allowed and a question
    /// is answered "Don't Allow" at once.
    pub(crate) fn allow(cx: &mut TestAppContext, read: bool, write: bool) {
        cx.update(|cx| allow_in(cx, read, write));
    }

    pub(crate) fn allow_in(cx: &mut App, read: bool, write: bool) {
        update_settings(cx, |settings| settings.agent_access = AgentAccess::Ask);
        // What was allowed before is taken back, and the streams it started end with it.
        let before: Vec<u64> = state(cx).grants.held().iter().map(|held| held.id).collect();
        if let Some(bus) = crate::automation::bus(cx) {
            for id in before {
                bus.revoke_holder(id);
            }
        }
        let state = state(cx);
        state.grants = wormhole_core::ipc::access::Grants::default();
        let mut caps = CapSet::EMPTY;
        if read {
            caps = caps.with(Capability::Read);
        }
        if write {
            caps = caps.with(Capability::Type).with(Capability::Run).with(Capability::Open);
        }
        if caps != CapSet::EMPTY {
            state.grants.grant(Identity::scripts(), "Scripts".into(), Scope::UntilQuit, caps);
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
            state(cx).grants.grant(agent, "Claude Code".into(), Scope::Session(process), CapSet::STANDARD)
        })
    }

    /// Nobody answers a question until the test does.
    pub(crate) fn hold(cx: &mut TestAppContext) {
        cx.update(|cx| super::set_script(Script::Hold, cx));
    }
}
