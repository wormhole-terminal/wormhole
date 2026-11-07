//! A scratch data folder, the host binary built into this tree's target, and waiting.

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
        let data = self.dir.path().display().to_string();
        HostedTerminals::new(Some(data), Some(self.host.display().to_string()), 1).expect("desk")
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
            program: "/bin/cat".into(),
            args: Vec::new(),
            cwd: Some(self.cwd().into()),
            cols: 80,
            rows: 24,
            cell_width: 8.0,
            cell_height: 16.0,
            old_host: None,
            saved_text: None,
        }
    }

    pub(super) fn live_hosts(&self) -> Vec<String> {
        let mut ids: Vec<String> = hosts::scan(&self.hosts_dir())
            .map(|scan| scan.live.into_iter().map(|meta| meta.id).collect())
            .unwrap_or_default();
        ids.sort();
        ids
    }

    /// The database forgets every saved terminal (a lost database, another profile's).
    pub(super) fn forget_rows(&self) {
        let mut store = wormhole_core::storage::store::Store::open_at(self.dir.path()).expect("store");
        store.delete_sessions_except(1, &[]).expect("forgets");
    }
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
        let status = Command::new(env!("CARGO"))
            .args(["build", "-p", "wormhole-host", "--manifest-path"])
            .arg(root.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(target)
            .status()
            .expect("cargo");
        assert!(status.success(), "building wormhole-host");
    });
    host
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
