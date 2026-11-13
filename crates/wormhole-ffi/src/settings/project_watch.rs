//! Following a project's `.wormhole` folder while the Project folders page shows it, so a hand edit, a script or an
//! agent reaches the page without anyone polling it (the page's own read when the window comes forward stays as
//! the fallback: a watch can fail or drop events). The same shape as `watch`: the OS delivers events on a thread
//! of its own whose callback only sorts the paths and sends a mark; one thread of ours waits for the writer to
//! finish and tells the listener once. Dropping the [`ProjectWatch`] stops both.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use wormhole_core::platform::watch::{self as os_watch, Watch};
use wormhole_core::project::{DIR_NAME, LEGACY_DIR_NAME};

/// How long after the first event of a burst the listener is told: a save is a few events.
const SETTLE: Duration = Duration::from_millis(100);

/// Hears that the project's folder changed. Called on a thread of ours; it must return at once.
#[uniffi::export(with_foreign)]
pub trait ProjectListener: Send + Sync {
    fn changed(&self);
}

/// Keeps the watch and its thread alive.
#[derive(uniffi::Object)]
pub struct ProjectWatch {
    _watch: Option<Watch>,
    _stop: Sender<()>,
}

/// Follows the `.wormhole` folder of the project at `root` (and the older `.andromeda` one). A project folder that
/// cannot be watched is followed by nothing: the page then reads when its window comes forward.
#[uniffi::export]
pub fn settings_watch_project(root: String, listener: Arc<dyn ProjectListener>) -> Arc<ProjectWatch> {
    let (mark, marked) = mpsc::channel::<()>();
    let watch = open(&root, mark.clone());
    std::thread::Builder::new()
        .name("project-watch".into())
        .spawn(move || {
            // Ends when the sender in the `ProjectWatch` (and the watch's own) is gone.
            while marked.recv().is_ok() {
                std::thread::sleep(SETTLE);
                while marked.try_recv().is_ok() {}
                listener.changed();
            }
        })
        .ok();
    Arc::new(ProjectWatch { _watch: watch, _stop: mark })
}

fn open(root: &str, mark: Sender<()>) -> Option<Watch> {
    let base = PathBuf::from(root).canonicalize().ok()?;
    let folders = [base.join(DIR_NAME), base.join(LEGACY_DIR_NAME)];
    os_watch::watch(&[base.as_path()], move |batch| {
        if batch.rescan
            || batch.paths.iter().any(|path| folders.iter().any(|folder| path.starts_with(folder)))
        {
            let _ = mark.send(());
        }
    })
    .map_err(|err| tracing::warn!("not following the project's folder: {err}"))
    .ok()
}
