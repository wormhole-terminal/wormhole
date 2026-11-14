//! The I/O of the title row's extras: what `WorkspaceModel` says is wanted (`ports_wanted`,
//! `pull_request_branches`) is looked up here and handed back, as the GPUI app's `window/title/extras.rs`
//! does. The rules are core's: the ports from the shell's process tree (`wormhole_term::listening_ports`),
//! the pull request from `gh` (`git::pr`, its cache's deadlines), and `gh` only in a project the person
//! trusts, since it reads the repository's own git configuration. What the row shows follows the trust as it is
//! now: the model holds a branch's pull request only while its project is approved, and a trust changed in this
//! process is looked at at once (GPUI's `shown_pull_request` over `session_pr_branch`, every tick).
//!
//! Nothing here sleeps: the pump asks `step` what is due and when the next look is.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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

/// How many times a project's approvals (or its switches) changed, in this process or in the approvals' file (another
/// program's write is told as an event: `files_panel::project::stores`): the branches are judged again at the next look
/// whenever it moved, not at the minute's end. The minute is the fallback for a change no event told.
static TRUST_CHANGES: AtomicU64 = AtomicU64::new(0);

/// A project was trusted or revoked: every workspace looks at its branches again now.
pub(crate) fn trust_changed() {
    TRUST_CHANGES.fetch_add(1, Ordering::AcqRel);
    crate::workspace_pump::kick_every();
}

#[derive(Default)]
pub(crate) struct Extras {
    ports_looked: Mutex<Option<(i64, Instant)>>,
    ports_busy: AtomicBool,
    cache: Mutex<PrCache>,
    /// The branches last judged, at which count of trust changes, and when.
    scanned: Mutex<Option<(Vec<Branch>, u64, Instant)>>,
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
    let told = TRUST_CHANGES.load(Ordering::Acquire);
    {
        let mut scanned = locked(&shared.extras.scanned);
        match scanned.as_ref() {
            Some((seen, seen_told, at))
                if *seen == branches && *seen_told == told && at.elapsed() < PULL_REQUESTS_EVERY =>
            {
                return Some(PULL_REQUESTS_EVERY.saturating_sub(at.elapsed()));
            }
            _ => *scanned = Some((branches.clone(), told, Instant::now())),
        }
    }
    for branch in branches {
        // Shown only while the project is approved: withheld once it is not, and what is known shown again at once
        // when it is approved anew (the cache keeps it either way).
        let trusted = approved(shared, &branch.root);
        let known = if trusted { locked(&shared.extras.cache).get(&branch).cloned() } else { None };
        let events = shared.with_model(|model| model.set_pull_request(branch.clone(), known));
        shared.post(events);
        if !trusted || !locked(&shared.extras.cache).begin(&branch, Instant::now()) {
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
                // Revoked while `gh` ran: the answer is kept, not shown.
                let known = known.filter(|_| approved(&shared, &branch.root));
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

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods, reason = "the test makes its repository in a temp folder")]

    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use wormhole_core::git::pr::{Branch, PrState, PullRequest};
    use wormhole_core::project::ProjectFolder;
    use wormhole_core::workspace::Action;

    use super::{locked, pull_requests};
    use crate::{Windows, Workspace, WorkspaceConfig, WorkspaceListener};

    struct Quiet;

    impl WorkspaceListener for Quiet {
        fn wake(&self) {}
    }

    fn eventually(what: &str, mut done: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !done() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    // GPUI: `window/title/extras.rs:183` `session_pr_branch` ends in `programs_approved`, so a revoked project's pull
    // request is not shown, though the app's cache still holds it.
    #[test]
    fn a_cached_pull_request_is_not_shown_once_its_project_is_revoked() {
        let data = tempfile::tempdir().expect("data dir");
        let repo = tempfile::tempdir().expect("repo");
        std::fs::create_dir(repo.path().join(".git")).expect("git");
        std::fs::create_dir(repo.path().join(".wormhole")).expect("project folder");
        let root = repo.path().canonicalize().expect("canonical");
        let windows = Windows::in_dir(data.path()).expect("windows");
        let profile = windows.profile().dir().to_path_buf();
        let config = WorkspaceConfig {
            home: None,
            numbers_follow_list: true,
            start_folder: None,
            pane_header_height: 0.0,
        };
        let workspace = Workspace::in_window(windows, 1, config, Arc::new(Quiet));
        let shared = workspace.shared();

        let folder = ProjectFolder::open(&root).expect("a project");
        crate::files_panel::change_trust_in(&profile, |trust| {
            trust.trust(&folder);
            true
        })
        .expect("trusted");
        // `gh` answered a moment ago: nothing is due, so no lookup starts.
        let branch = Branch { root: root.clone(), name: "main".into() };
        let found = PullRequest {
            number: 7,
            url: "https://github.invalid/o/r/pull/7".into(),
            state: PrState::Open,
            draft: false,
            checks: None,
            review: None,
        };
        {
            let mut cache = locked(&shared.extras.cache);
            assert!(cache.begin(&branch, Instant::now()));
            let _ = cache.finish(&branch, Ok(Some(found)), Instant::now());
        }
        workspace.apply(Action::NewTerminal);
        let id = workspace.selected().expect("a terminal");
        let _ = shared.with_model(|model| {
            model.update_facts(id, |facts| {
                facts.repo = Some(root.clone());
                facts.branch = Some("main".into());
            })
        });
        let shown = || {
            pull_requests(shared);
            workspace.title("Macintosh HD".into()).pull_request_words
        };
        eventually("the chip while trusted", || shown().is_some());

        crate::files_panel::change_trust_in(&profile, |trust| trust.revoke(&root)).expect("revoked");
        // Looked at straight after, well inside the minute between two looks: the revoke itself made it due.
        eventually("no chip once revoked", || shown().is_none());
        assert!(
            locked(&shared.extras.cache).get(&branch).is_some(),
            "the cache keeps the answer for a trust to come"
        );
    }
}
