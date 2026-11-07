//! The fixture the tests share: a server on a temp data folder, a socket client and the waits.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::workspace::{Action, SidebarRow};

use crate::aux_backends::{AccessDesk, AuxListener};
use crate::files_panel::PanelListener;
use crate::hook_server::{HookEvent, HookListener, HookServer};
use crate::{Terminal, TerminalListener, UsageService, Workspace, WorkspaceConfig, WorkspaceListener};

#[derive(Default)]
pub(super) struct Wakes(AtomicUsize);

impl Wakes {
    pub(super) fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

macro_rules! counts_wakes {
    ($($listener:ty),*) => {$(
        impl $listener for Wakes {
            fn wake(&self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
    )*};
}
counts_wakes!(WorkspaceListener, TerminalListener, HookListener, AuxListener, PanelListener);

pub(super) struct Fixture {
    pub(super) _root: tempfile::TempDir,
    pub(super) dir: PathBuf,
    pub(super) workspace: Arc<Workspace>,
    pub(super) usage: Arc<UsageService>,
    pub(super) desk: Arc<AccessDesk>,
    pub(super) server: Arc<HookServer>,
    pub(super) wakes: Arc<Wakes>,
}

/// A server on a fresh `Wormhole-swift-hook` folder whose settings let agents in as `mode` says
/// (`open`, `ask` or `off`), with the Automation feature on.
pub(super) fn fixture(mode: &str) -> Fixture {
    crate::isolate_for_tests();
    let root = tempfile::tempdir().expect("a temp dir");
    let dir = root.path().join("Wormhole-swift-hook");
    std::fs::create_dir_all(&dir).expect("data dir");
    let config =
        json!({ "agent_access": mode, "control_write": true, "feature_flags": { "automation": true } });
    let file = DataRoot::new(&dir).default_profile().config_file();
    std::fs::File::create(file)
        .and_then(|mut file| file.write_all(config.to_string().as_bytes()))
        .expect("config.json");
    let wakes = Arc::new(Wakes::default());
    let workspace = Workspace::new(
        WorkspaceConfig {
            home: None,
            numbers_follow_list: true,
            start_folder: None,
            pane_header_height: 0.0,
        },
        wakes.clone(),
    );
    let usage = UsageService::in_dir(&dir, wakes.clone());
    let desk = AccessDesk::new(None, Some(dir.display().to_string()), wakes.clone()).expect("a desk");
    let server = HookServer::start(
        workspace.clone(),
        usage.clone(),
        desk.clone(),
        wakes.clone(),
        Some(dir.display().to_string()),
    )
    .expect("a server");
    Fixture { _root: root, dir, workspace, usage, desk, server, wakes }
}

impl Fixture {
    /// A terminal as Swift makes one: the model's `Opened`, a `/bin/cat` started for its tab, registered.
    pub(super) fn terminal(&self) -> i64 {
        let events = self.workspace.apply(Action::NewTerminal);
        let id = opened(&events);
        self.start(id);
        id
    }

    /// A terminal whose program is a script named `claude`, which the registry takes for the agent its name
    /// says (a copy of a system program would be killed by macOS), found in front once this returns: an agent
    /// that hooks drive is not one a look at the terminal's process has to disagree with.
    pub(super) fn agent_terminal(&self) -> i64 {
        let script = self.dir.join("claude");
        let body = "#!/bin/sh\nwhile IFS= read -r line; do :; done\n";
        std::fs::File::create(&script)
            .and_then(|mut file| file.write_all(body.as_bytes()))
            .expect("a script");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("executable");
        let events = self.workspace.apply(Action::NewTerminal);
        let id = opened(&events);
        self.start_program(id, &script.to_string_lossy());
        eventually("the agent in front", || self.row(id).and_then(|row| row.agent).is_some());
        id
    }

    /// Start and register the program of terminal `id` (made by the model already).
    pub(super) fn start(&self, id: i64) {
        self.start_program(id, "/bin/cat");
    }

    fn start_program(&self, id: i64, program: &str) {
        let terminal = Terminal::start_for_tab(
            id,
            program.into(),
            Vec::new(),
            None,
            80,
            24,
            8.0,
            16.0,
            self.wakes.clone(),
        )
        .expect("a terminal");
        self.workspace.register_terminal(id, terminal);
    }

    pub(super) fn client(&self) -> Client {
        let stream = UnixStream::connect(self.server.socket_path()).expect("the socket");
        stream.set_read_timeout(Some(Duration::from_secs(10))).expect("timeout");
        Client { reader: BufReader::new(stream.try_clone().expect("clone")), stream }
    }

    /// What the server asked of Swift so far.
    pub(super) fn events(&self) -> Vec<HookEvent> {
        self.server.drain_events()
    }

    pub(super) fn row(&self, id: i64) -> Option<SidebarRow> {
        self.workspace
            .sidebar()
            .sections
            .into_iter()
            .flat_map(|section| section.rows)
            .find(|row| row.id == id)
    }
}

fn opened(events: &[crate::WorkspaceEvent]) -> i64 {
    events
        .iter()
        .find_map(|event| match event {
            crate::WorkspaceEvent::Opened { id, .. } => Some(*id),
            _ => None,
        })
        .expect("an opened terminal")
}

/// A connection to the socket, as `wh` makes one.
pub(super) struct Client {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
}

impl Client {
    pub(super) fn send(&mut self, line: &Value) {
        writeln!(self.stream, "{line}").expect("write");
    }

    pub(super) fn writer(&mut self) -> &mut UnixStream {
        &mut self.stream
    }

    pub(super) fn send_raw(&mut self, bytes: &[u8]) {
        self.stream.write_all(bytes).expect("write");
    }

    /// The next line, parsed; `None` when the server closed the connection or nothing came in time.
    pub(super) fn line(&mut self) -> Option<Value> {
        let mut line = String::new();
        let read = self.reader.read_line(&mut line).ok()?;
        (read > 0).then(|| serde_json::from_str(&line).expect("a JSON line"))
    }

    /// A control request and its reply.
    pub(super) fn control(&mut self, action: Value) -> Value {
        let mut request = json!({ "kind": "control", "version": 1 });
        request.as_object_mut().expect("object").extend(action.as_object().expect("object").clone());
        self.send(&request);
        self.line().expect("a reply")
    }
}

/// Wait (bounded) until `done` holds: the feeds and the server run on threads of their own.
pub(super) fn eventually(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}
