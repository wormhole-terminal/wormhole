//! Tests of the panel's seams over real temp folders: no GPUI, no window, no Touch ID prompt (the owner
//! is a stub), and a data folder of each test's own so none touches the experiment's real one.

#![allow(clippy::disallowed_methods, reason = "tests write their fixtures into temp folders")]

mod files;
mod files_pins;
mod notes_store;
mod project;
mod project_notes;
mod project_paint;
mod project_trust;
mod usage;

use std::fs;
use std::path::Path;

pub(super) fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(path, text).expect("write");
}

pub(super) fn link(target: &Path, at: &Path) {
    std::os::unix::fs::symlink(target, at).expect("symlink");
}

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use tempfile::TempDir;

use crate::files_panel::{OwnerAnswer, OwnerAuthenticator, ProjectService};

/// The owner, answering from a script and counting how often it was asked. A test never raises Touch ID.
pub(super) struct Owner {
    answers: Mutex<VecDeque<OwnerAnswer>>,
    pub(super) asked: AtomicU32,
}

impl Owner {
    pub(super) fn answering(answers: impl IntoIterator<Item = OwnerAnswer>) -> Arc<Self> {
        Arc::new(Self { answers: Mutex::new(answers.into_iter().collect()), asked: AtomicU32::new(0) })
    }

    pub(super) fn asked(&self) -> u32 {
        self.asked.load(Ordering::SeqCst)
    }
}

impl OwnerAuthenticator for Owner {
    fn authenticate(&self, _reason: String) -> OwnerAnswer {
        self.asked.fetch_add(1, Ordering::SeqCst);
        self.answers.lock().expect("script").pop_front().unwrap_or(OwnerAnswer::Cancelled)
    }
}

/// A repository with a `.wormhole/` folder holding `files` (and a `.git`, where the walk up stops).
pub(super) struct Repo {
    pub(super) dir: TempDir,
}

impl Repo {
    pub(super) fn new(files: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::create_dir(dir.path().join(".git")).expect("git");
        for (name, text) in files {
            write(&dir.path().join(".wormhole").join(name), text);
        }
        if files.is_empty() {
            fs::create_dir(dir.path().join(".wormhole")).expect("mkdir");
        }
        Self { dir }
    }

    /// The project's root as the service names it (canonical).
    pub(super) fn root(&self) -> String {
        self.dir.path().canonicalize().expect("canonical").to_string_lossy().into_owned()
    }

    pub(super) fn path(&self) -> PathBuf {
        self.dir.path().canonicalize().expect("canonical")
    }

    pub(super) fn put(&self, name: &str, text: &str) {
        write(&self.dir.path().join(".wormhole").join(name), text);
    }
}

/// A service with a data folder of its own, over the scripted owner.
pub(super) fn service(owner: &Arc<Owner>) -> (TempDir, Arc<ProjectService>) {
    let data = tempfile::tempdir().expect("data dir");
    let service = ProjectService::in_dir(data.path(), owner.clone());
    (data, service)
}

pub(super) const RUN_TESTS: &str =
    r#"[{"id": "test", "title": "Run tests", "type": "run", "command": "cargo test", "key": "cmd-ctrl-t"}]"#;
