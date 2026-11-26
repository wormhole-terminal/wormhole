//! A scratch data folder, the host binary built into this tree's target, and waiting.

use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use tempfile::TempDir;
use wormhole_core::ipc::hosts;

use crate::events::TerminalListener;
use crate::hosting::{HostedStart, HostedTerminals};

pub(super) struct Fixture {
    dir: TempDir,
    host: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        // /tmp, not the system's temp folder: a socket's path must fit in 104 bytes.
        let dir = tempfile::Builder::new().prefix("Wormhole-swift-p-").tempdir_in("/tmp").expect("scratch");
        Self { dir, host: host_binary() }
    }

    pub(super) fn desk(&self) -> Arc<HostedTerminals> {
        self.desk_over(&self.host)
    }

    /// A desk that starts `host` (a path of the test's choosing) as its session host program.
    pub(super) fn desk_over(&self, host: &Path) -> Arc<HostedTerminals> {
        let data = self.dir.path().display().to_string();
        HostedTerminals::new(Some(data), Some(host.display().to_string()), 1).expect("desk")
    }

    /// A host program that waits until [`HeldHost::release`] and then is the real host: a start that uses it stays
    /// under way for as long as the test wants, where a real host's takes a few milliseconds.
    pub(super) fn held_host(&self) -> HeldHost {
        let (program, go) = (self.dir.path().join("held-host"), self.dir.path().join("go"));
        // The host is started with next to no environment, so the script names every program by its path. It gives up
        // after ten seconds (the start itself gives up after five), so a test that failed leaves no shell waiting.
        let script = format!(
            "#!/bin/sh\nn=0\nwhile [ ! -e {go} ]; do\n  [ \"$n\" -lt 500 ] || exit 1\n  n=$((n + 1))\n  /bin/sleep 0.02\ndone\nexec {host} \"$@\"\n",
            go = sh_quoted(&go),
            host = sh_quoted(&self.host),
        );
        std::fs::write(&program, script).expect("the held host");
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("executable");
        HeldHost { program, go }
    }

    pub(super) fn desk_without_program(&self) -> Arc<HostedTerminals> {
        HostedTerminals::new(Some(self.dir.path().display().to_string()), None, 1).expect("desk")
    }

    pub(super) fn cwd(&self) -> &str {
        self.dir.path().to_str().expect("utf8")
    }

    pub(super) fn config_file(&self) -> PathBuf {
        self.dir.path().join("config.json")
    }

    pub(super) fn sessions_dir(&self) -> PathBuf {
        wormhole_core::terminal::checkpoint::sessions_dir(self.dir.path())
    }

    pub(super) fn hosts_dir(&self) -> PathBuf {
        hosts::hosts_dir(self.dir.path())
    }

    /// `cat`: what is typed comes back, so the screen shows it.
    pub(super) fn request(&self, tab: i64) -> HostedStart {
        HostedStart {
            tab,
            shell: None,
            program: Some("/bin/cat".into()),
            args: Vec::new(),
            cwd: Some(self.cwd().into()),
            cols: 80,
            rows: 24,
            cell_width: 8.0,
            cell_height: 16.0,
            old_host: None,
            saved_text: None,
            profile: None,
        }
    }

    pub(super) fn live_hosts(&self) -> Vec<String> {
        let mut ids: Vec<String> = hosts::scan(&self.hosts_dir())
            .map(|scan| scan.live.into_iter().map(|meta| meta.id).collect())
            .unwrap_or_default();
        ids.sort();
        ids
    }

    /// The terminal ids window 1 names a host for in the profile `profile` of this folder (its own database).
    pub(super) fn hosted_rows_in(&self, profile: &str) -> Vec<i64> {
        use wormhole_core::storage::paths::{DataRoot, ProfileId};
        let paths = DataRoot::new(self.dir.path()).profile(&ProfileId::parse(profile).expect("id"));
        let store = wormhole_core::storage::store::Store::open_at(paths.dir()).expect("store");
        let windows = store.load().expect("loads");
        windows
            .iter()
            .filter(|w| w.id == 1)
            .flat_map(|w| &w.sessions)
            .filter(|s| s.host_id.is_some())
            .map(|s| s.id)
            .collect()
    }

    /// The default profile's database, as the app's other parts open it.
    fn store(&self) -> wormhole_core::storage::store::Store {
        wormhole_core::storage::store::Store::open_at(self.dir.path()).expect("store")
    }

    /// Terminal `tab`'s row in window 1, as the next launch loads it (with its notes, tasks and saved text).
    pub(super) fn row(&self, tab: i64) -> Option<wormhole_core::storage::store::SessionRecord> {
        let windows = self.store().load().expect("loads");
        windows.into_iter().filter(|w| w.id == 1).flat_map(|w| w.sessions).find(|s| s.id == tab)
    }

    /// The window saved a name for terminal `tab` (its row as it is, with the name).
    pub(super) fn name_row(&self, tab: i64, name: &str) {
        let row = self.row(tab).expect("a row to name");
        let named = wormhole_core::storage::store::SessionRecord { name: Some(name.into()), ..row };
        self.store().save_session(1, &named).expect("saves");
    }

    /// The Notes tab saved a note and a task for terminal `tab` (`SessionNotesStore.save`).
    pub(super) fn save_notes(&self, tab: i64, note: &str, task: &str) {
        use wormhole_core::storage::store::{NoteRecord, SessionNotes, TaskRecord};
        let notes = SessionNotes {
            notes: vec![NoteRecord { body: note.into(), collapsed: false }],
            tasks: vec![TaskRecord { text: task.into(), done: false }],
            ..SessionNotes::default()
        };
        self.store().save_session_notes(tab, &notes).expect("saves the notes");
    }

    /// The database forgets every saved terminal (a lost database, another profile's).
    pub(super) fn forget_rows(&self) {
        let mut store = wormhole_core::storage::store::Store::open_at(self.dir.path()).expect("store");
        store.delete_sessions_except(1, &[]).expect("forgets");
    }
}

/// A host program made by [`Fixture::held_host`].
pub(super) struct HeldHost {
    program: PathBuf,
    go: PathBuf,
}

impl HeldHost {
    pub(super) fn program(&self) -> &Path {
        &self.program
    }

    /// Let the hosts started so far (and later) go on to be the real host.
    pub(super) fn release(&self) {
        std::fs::write(&self.go, "").expect("releases the host");
    }
}

fn sh_quoted(path: &Path) -> String {
    format!("'{}'", path.to_str().expect("utf8").replace('\'', "'\\''"))
}

impl Drop for Fixture {
    /// A failed test must not leave a host running.
    fn drop(&mut self) {
        let _ = hosts::end_all_live(&self.hosts_dir(), Duration::from_secs(5));
    }
}

/// The host program of this tree, built beside the tests the first time (`cargo build -p wormhole-host` in
/// the repository's workspace, into the target the tests run from).
fn host_binary() -> PathBuf {
    static BUILD: Once = Once::new();
    let exe = std::env::current_exe().expect("test exe");
    let profile_dir = exe.parent().and_then(Path::parent).expect("target/<profile>");
    let host = profile_dir.join("wormhole-host");
    BUILD.call_once(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let target = profile_dir.parent().expect("target");
        let mut cargo = Command::new(env!("CARGO"));
        cargo
            .args(["build", "-p", "wormhole-host", "--manifest-path"])
            .arg(root.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(target);
        for name in std::env::vars_os().map(|(name, _)| name).filter(|name| is_cargo_tests_own(name)) {
            cargo.env_remove(name);
        }
        let status = cargo.status().expect("cargo");
        assert!(status.success(), "building wormhole-host");
    });
    host
}

/// What `cargo test` tells a test about the package under test. A build that inherits it is not the build a shell
/// makes: a dependency's build script (ring's) records `CARGO_MANIFEST_DIR` and the like in its fingerprint, so the
/// test's build and every build after it from outside rebuilt ring and all that depends on it, again and again
/// (30 s on a runner), and a host built in an earlier step was never the one the tests found fresh.
fn is_cargo_tests_own(name: &OsStr) -> bool {
    const NAMED: [&str; 5] = [
        "CARGO_MANIFEST_DIR",
        "CARGO_MANIFEST_PATH",
        "CARGO_CRATE_NAME",
        "CARGO_PRIMARY_PACKAGE",
        "CARGO_TARGET_TMPDIR",
    ];
    let name = name.to_string_lossy();
    NAMED.contains(&name.as_ref())
        || ["CARGO_PKG_", "CARGO_BIN_EXE_"].iter().any(|prefix| name.starts_with(prefix))
}

pub(super) fn listener() -> Arc<dyn TerminalListener> {
    struct Quiet;
    impl TerminalListener for Quiet {
        fn wake(&self) {}
    }
    Arc::new(Quiet)
}

pub(super) fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}
