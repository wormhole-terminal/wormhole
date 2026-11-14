//! The I/O of the title row's extras: what `WorkspaceModel` says is wanted (`ports_wanted`,
//! `pull_request_branches`) is looked up here and handed back, as the GPUI app's `window/title/extras.rs`
//! does. The rules are core's: the ports from the shell's process tree (`wormhole_term::listening_ports`),
//! the pull request from `gh` (`git::pr`, its cache's deadlines), and `gh` only in a project the person
//! trusts, since it reads the repository's own git configuration.
//!
//! Nothing here sleeps: the pump asks `step` what is due and when the next look is.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use wormhole_core::config::Config;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::git::pr::{self, Branch, PrCache};
use wormhole_core::project::{ProjectFolder, TrustStatus};
use wormhole_core::storage::paths::{DataRoot, ProfilePaths};
use wormhole_core::workspace::StatusEvent;
use wormhole_term::listening_ports;

use crate::workspace::Shared;

/// How often the ports under the selected terminal are looked at: the system sends no event for a socket
/// that starts listening, so this is the whole mechanism, and only while the row shows ports.
const PORTS_EVERY: Duration = Duration::from_secs(5);
/// How often the branches are asked what is due (the cache's own deadlines are minutes): a new branch is
/// asked at once, as the set of branches moving shows.
const PULL_REQUESTS_EVERY: Duration = Duration::from_secs(60);

#[derive(Default)]
pub(crate) struct Extras {
    ports_looked: Mutex<Option<(i64, Instant)>>,
    ports_busy: AtomicBool,
    cache: Mutex<PrCache>,
    scanned: Mutex<Option<(Vec<Branch>, Instant)>>,
}

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Do what is due, and say when the next look is, on the workspace's clock.
pub(crate) fn step(shared: &Arc<Shared>) -> Option<u64> {
    let now = shared.now();
    let ms = |wait: Duration| now + u64::try_from(wait.as_millis()).unwrap_or(0);
    [ports(shared).map(ms), pull_requests(shared).map(ms)].into_iter().flatten().min()
}

fn ports(shared: &Arc<Shared>) -> Option<Duration> {
    let id = shared.peek_model(|model| model.ports_wanted())?;
    let extras = &shared.extras;
    let wait = match *locked(&extras.ports_looked) {
        Some((who, at)) if who == id => PORTS_EVERY.saturating_sub(at.elapsed()),
        _ => Duration::ZERO,
    };
    if !wait.is_zero() || extras.ports_busy.swap(true, Ordering::AcqRel) {
        return Some(wait.max(Duration::from_millis(250)));
    }
    *locked(&extras.ports_looked) = Some((id, Instant::now()));
    let shell = shared.terminal_of(id).and_then(|terminal| terminal.shell_pid());
    let shared = shared.clone();
    let spawned = std::thread::Builder::new().name("wormhole-ffi-ports".into()).spawn(move || {
        let found = shell.map(listening_ports).unwrap_or_default();
        let events = shared.with_model(|model| model.set_ports(id, found));
        shared.post(events);
        shared.extras.ports_busy.store(false, Ordering::Release);
    });
    if spawned.is_err() {
        // No thread: the next round tries again.
        extras.ports_busy.store(false, Ordering::Release);
    }
    Some(PORTS_EVERY)
}

fn pull_requests(shared: &Arc<Shared>) -> Option<Duration> {
    let branches = shared.peek_model(|model| model.pull_request_branches());
    if branches.is_empty() {
        return None;
    }
    {
        let mut scanned = locked(&shared.extras.scanned);
        match scanned.as_ref() {
            Some((seen, at)) if *seen == branches && at.elapsed() < PULL_REQUESTS_EVERY => {
                return Some(PULL_REQUESTS_EVERY.saturating_sub(at.elapsed()));
            }
            _ => *scanned = Some((branches.clone(), Instant::now())),
        }
    }
    for branch in branches {
        let now = Instant::now();
        if !locked(&shared.extras.cache).is_due(&branch, now) || !approved(shared, &branch.root) {
            continue;
        }
        if !locked(&shared.extras.cache).begin(&branch, now) {
            continue;
        }
        let shared = shared.clone();
        let spawned = std::thread::Builder::new().name("wormhole-ffi-pr".into()).spawn({
            let shared = shared.clone();
            let branch = branch.clone();
            move || {
                let found = pr::pull_request(&branch.root);
                let (settled, known) = {
                    let mut cache = locked(&shared.extras.cache);
                    let settled = cache.finish(&branch, found, Instant::now());
                    (settled, cache.get(&branch).cloned())
                };
                let events = shared.with_model(|model| model.set_pull_request(branch.clone(), known));
                shared.post(events);
                if let Some(settled) = settled {
                    tell_checks_settled(&shared, &branch, &settled);
                }
            }
        });
        if spawned.is_err() {
            let _ = locked(&shared.extras.cache).finish(&branch, Err(pr::PrError::TimedOut), Instant::now());
        }
    }
    Some(PULL_REQUESTS_EVERY)
}

/// A pull request's checks finished: the first terminal on its branch tells it the way a program's own
/// notification is told (`Wormhole notify`'s path), so the notification settings and the sidebar's
/// needs-you rules apply as they do to those (GPUI's `tell_checks_settled`).
fn tell_checks_settled(shared: &Shared, branch: &Branch, settled: &pr::Settled) {
    let Some(id) = shared.peek_model(|model| model.first_on_branch(branch)) else { return };
    let (title, body) = settled.notification(&branch.name);
    shared.feed(id, StatusEvent::ControlNotice { title, body });
}

/// The profile the window is in: its trust store and config decide, the data folder's default where the
/// workspace is in no window (tests).
fn profile_of(shared: &Shared) -> Option<ProfilePaths> {
    if let Some((windows, _)) = shared.windows() {
        return Some(windows.profile().clone());
    }
    let dir = crate::aux_backends::resolve_data_dir(None).ok()?;
    Some(DataRoot::new(dir).default_profile())
}

/// Whether `gh` may be started for the repository at `root`: the project folders feature is on and its
/// `.wormhole/` content is approved (`ProjectHub::trusted`), both in the window's profile.
fn approved(shared: &Shared, root: &Path) -> bool {
    let Some(profile) = profile_of(shared) else { return false };
    if !Config::load_for(&profile).settings.feature_enabled(FeatureFlag::ProjectFolders) {
        return false;
    }
    let Some(folder) = ProjectFolder::open(root) else { return false };
    // The profile's approvals as the process holds them (`files_panel::trust_status_in`), every window's revoke in them.
    crate::files_panel::trust_status_in(profile.dir(), &folder.snapshot()) == TrustStatus::Trusted
}
