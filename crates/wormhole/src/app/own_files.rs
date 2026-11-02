//! The app's own files followed through file events: `config.json`, the theme files and the custom
//! fonts folder of the shown profile. A change made by hand, by a script or by the Theme Studio in
//! another window applies at once instead of at the next tick's `stat`.
//!
//! The watcher only says "look now". What changed, and what to do about it, stays where it was
//! decided: `SettingsStore::poll` (config and themes) and `fonts::poll_custom` (fonts) compare the disk
//! with what they hold and do nothing when it is the same, so a write Wormhole made itself, a stray
//! event or a lost one costs a `stat` and never a wrong reload.
//!
//! The pieces are those of `window/repo/watches.rs`: the OS delivers events on a thread of its own,
//! whose callback only sorts the paths ([`Roots::classify`]) and leaves marks; a task on the UI executor
//! waits for the writer to finish ([`SETTLE`]) and applies them. The stream is opened off the UI thread.
//!
//! One watch on the profile's folder as a whole, not on the file: an atomic write replaces
//! `config.json`, and a watch on the old file would follow the old file. That folder is also where the
//! database and logs are written, so the callback drops everything but the three places that matter
//! before it wakes anyone.
//!
//! The tick keeps polling, slowly ([`SLOW_POLL`]): a volume the watcher refuses (a network home), a watch
//! that failed or is still starting, and what no event names (a theme given as a path outside the
//! profile) are only ever seen that way.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::platform::watch::{self as os_watch, Batch, Watch};
use wormhole_core::storage::paths::ProfilePaths;

/// How long after the first event of a burst the files are read: an editor's save, or a theme saved
/// from the Studio, is a few events, and the read should see the finished file. Short, because what
/// the person did is waiting on it.
const SETTLE: Duration = Duration::from_millis(40);

/// How often the tick looks anyway (in ticks of 400 ms: about ten seconds) while the watch is running.
pub(super) const SLOW_POLL: u64 = 25;

/// How long after a watch failed another try is made.
const RETRY: Duration = Duration::from_secs(60);

const CONFIG: u8 = 1;
const FONTS: u8 = 2;

/// The three places of a profile that Wormhole reads back. Real paths, because that is how the system
/// names the paths it reports.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Roots {
    config: PathBuf,
    themes: PathBuf,
    fonts: PathBuf,
}

impl Roots {
    fn of(profile: &ProfilePaths, base: &Path) -> Self {
        let under = |path: PathBuf| match path.file_name() {
            Some(name) => base.join(name),
            None => path,
        };
        Self {
            config: under(profile.config_file()),
            themes: under(profile.themes_dir()),
            fonts: under(profile.fonts_dir()),
        }
    }

    /// Which reads a changed path calls for (`CONFIG`, `FONTS`, both or none). Themes belong to the
    /// settings: they are resolved with them.
    fn classify(&self, path: &Path) -> u8 {
        if path == self.config || path.starts_with(&self.themes) {
            CONFIG
        } else if path.starts_with(&self.fonts) {
            FONTS
        } else {
            0
        }
    }

    fn classify_batch(&self, batch: &Batch) -> u8 {
        if batch.rescan && batch.paths.is_empty() {
            return CONFIG | FONTS; // events were lost and none named: anything may have changed
        }
        let found = batch.paths.iter().fold(0, |found, path| found | self.classify(path));
        // Lost events under named paths: what is under the profile folder may be any of them.
        if batch.rescan { found | CONFIG | FONTS } else { found }
    }
}

/// A watch that started, with what its task needs.
struct Started {
    watch: Watch,
    marks: Arc<AtomicU8>,
    woken: smol::channel::Receiver<()>,
}

enum State {
    Starting,
    Live,
    Failed(Instant),
}

/// The followed profile folder (see the module). A global of its own: `AppState`'s fields may only
/// shrink.
pub(crate) struct OwnFiles {
    /// The folder watched, as `active_profile` named it, so a profile switch is seen by comparing.
    dir: PathBuf,
    state: State,
    /// Held for their drop: the start is cancelled, the stream stops, the task ends.
    _starter: Option<Task<()>>,
    _watch: Option<Watch>,
    _task: Option<Task<()>>,
}

impl Global for OwnFiles {}

impl OwnFiles {
    /// Called on every tick: starts following the shown profile's folder, follows a profile switch and
    /// tries again after a failure. A comparison of two paths when nothing changed.
    pub(crate) fn follow(cx: &mut App) {
        let profile = crate::active_profile::paths();
        let now = Instant::now();
        let stale = match cx.try_global::<Self>() {
            None => true,
            Some(this) if this.dir != profile.dir() => true,
            Some(this) => matches!(this.state, State::Failed(at) if now.duration_since(at) >= RETRY),
        };
        if stale {
            let entry = start(profile, cx);
            cx.set_global(entry); // the old one is dropped here: its stream stops
        }
    }

    /// Whether a running watch follows the files, so the tick's poll is only the backstop.
    pub(crate) fn live(cx: &App) -> bool {
        cx.try_global::<Self>().is_some_and(|this| matches!(this.state, State::Live))
    }
}

fn start(profile: ProfilePaths, cx: &mut App) -> OwnFiles {
    let dir = profile.dir().to_path_buf();
    let bg = cx.background_executor().clone();
    let starter = cx.spawn(async move |cx| {
        let started = {
            let profile = profile.clone();
            bg.spawn(async move { begin(&profile) }).await
        };
        cx.update(|cx| install(profile, started, cx));
    });
    OwnFiles { dir, state: State::Starting, _starter: Some(starter), _watch: None, _task: None }
}

/// Open the stream. Runs off the UI thread (it resolves links and starts the system's stream).
fn begin(profile: &ProfilePaths) -> io::Result<Started> {
    // The folder may not exist yet on a first launch: made by the store before long, retried until.
    let base = profile.dir().canonicalize()?;
    let roots = Roots::of(profile, &base);
    let marks = Arc::new(AtomicU8::new(0));
    let (wake, woken) = smol::channel::bounded::<()>(1);
    let watch = os_watch::watch(&[base.as_path()], {
        let marks = marks.clone();
        move |batch| {
            let found = roots.classify_batch(&batch);
            if found != 0 {
                marks.fetch_or(found, Ordering::AcqRel);
                // Full already means a wake-up is on its way.
                let _ = wake.try_send(());
            }
        }
    })?;
    Ok(Started { watch, marks, woken })
}

fn install(profile: ProfilePaths, started: io::Result<Started>, cx: &mut App) {
    // A switch since the start began: the next tick starts the right one.
    if cx.try_global::<OwnFiles>().is_none_or(|this| this.dir != profile.dir()) {
        return;
    }
    let (state, watch, task) = match started {
        Ok(started) => {
            let task = cx.spawn(async move |cx| follow(started.woken, started.marks, cx).await);
            // What changed while the stream was starting is read once now.
            apply(CONFIG | FONTS, cx);
            (State::Live, Some(started.watch), Some(task))
        }
        Err(err) => {
            if !matches!(err.kind(), io::ErrorKind::Unsupported | io::ErrorKind::NotFound) {
                tracing::warn!("not following the profile's files: {err}");
            }
            (State::Failed(Instant::now()), None, None)
        }
    };
    let this = cx.global_mut::<OwnFiles>();
    (this.state, this._watch, this._task) = (state, watch, task);
}

/// Wait for marks and apply them. Ends when the watch is dropped (the channel closes with the callback
/// that held its sender) or the task is cancelled.
async fn follow(woken: smol::channel::Receiver<()>, marks: Arc<AtomicU8>, cx: &mut AsyncApp) {
    while woken.recv().await.is_ok() {
        cx.background_executor().timer(SETTLE).await;
        // Emptied before the marks are taken: an event after the take sends a wake-up of its own.
        while woken.try_recv().is_ok() {}
        let found = marks.swap(0, Ordering::AcqRel);
        if found != 0 {
            cx.update(|cx| apply(found, cx));
        }
    }
}

/// The same reloads the tick runs, at once.
fn apply(found: u8, cx: &mut App) {
    tracing::debug!(config = found & CONFIG != 0, fonts = found & FONTS != 0, "own files changed on disk");
    if found & CONFIG != 0 {
        super::tick::reload_settings(cx);
    }
    if found & FONTS != 0 && crate::fonts::poll_custom(cx) {
        cx.refresh_windows();
    }
}

#[cfg(test)]
#[path = "own_files_tests.rs"]
mod tests;
