//! What one refresh of the Session Hosts window needs from the app (on the main thread, cheap) and
//! the survey it then runs off it: the app's windows know which terminals hold which host (read on
//! the main thread: memory only), and everything that touches the disk, the database included, is
//! read in the background: the databases are opened read-only, so a look never creates, upgrades
//! or backs up anything.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui_kit::App;
use wormhole_core::ipc::hosts::report::{AppFacts, Attachment, Inputs, Report};
use wormhole_core::ipc::hosts::{self, Survey};
use wormhole_core::storage::paths::ProfilePaths;
use wormhole_core::storage::store::Store;

use crate::app_state::app;
use crate::window::hosting::host_binary;
use crate::window::windows::persistent_terminals_on;

/// How long one host gets to answer `GetInfo`: a host that is there answers in a millisecond, so a
/// stuck one is shown as not answering rather than holding the refresh up.
const ANSWER_WITHIN: Duration = Duration::from_millis(400);

/// The app's side of one refresh.
pub(super) struct Gather {
    flag_on: bool,
    host_binary: Option<PathBuf>,
    attachments: Vec<Attachment>,
    /// Every profile's files, whose databases name the hosts their terminals run in.
    profiles: Vec<ProfilePaths>,
}

impl Gather {
    /// Read the app's windows (memory only).
    pub(super) fn collect(cx: &App) -> Self {
        let state = app(cx);
        let mut attachments = Vec::new();
        for window in &state.windows {
            if let Some(workspace) = window.workspace.upgrade() {
                attachments.extend(workspace.read(cx).host_attachments(cx));
            }
        }
        let profiles =
            state.profiles.profiles().iter().map(|profile| state.root.profile(&profile.id)).collect();
        Self {
            flag_on: persistent_terminals_on(cx),
            host_binary: host_binary().cloned(),
            attachments,
            profiles,
        }
    }

    /// Survey the hosts folder and build the report. Blocks (files, sockets): run it off the main thread.
    pub(super) fn run(self) -> Report {
        let hosts_dir = hosts::hosts_dir(&wormhole_core::storage::paths::data_dir());
        let survey = hosts::survey(&hosts_dir, ANSWER_WITHIN).unwrap_or_else(|err| {
            tracing::warn!("couldn't look at the session hosts: {err}");
            Survey::default()
        });
        // Which hosts a saved terminal names, in every profile: an unnamed host is an orphan.
        let named = self.profiles.iter().flat_map(Store::host_ids_read_only).collect();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        Report::build(Inputs {
            app: AppFacts::this_build(),
            hosts_dir,
            flag_on: self.flag_on,
            host_binary: self.host_binary,
            scan: survey.scan,
            observed: survey.observed,
            attachments: self.attachments,
            named,
            now,
        })
    }
}
