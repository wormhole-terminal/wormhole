//! The I/O of the title row's extras: what `WorkspaceModel` says is wanted (`ports_wanted`,
//! `pull_request_branches`) is looked up here and handed back, as the GPUI app's `window/title/extras.rs`
//! does. The rules are core's: the ports from the shell's process tree (`wormhole_term::listening_ports`),
//! the pull request from `gh` (`git::pr`, its cache's deadlines), and `gh` only in a project the person
//! trusts, since it reads the repository's own git configuration. What the row shows follows the trust as it is
//! now: the model holds a branch's pull request only while its project is approved, and a trust changed in this
//! process is looked at at once (GPUI's `shown_pull_request` over `session_pr_branch`, every tick).
//!
//! Nothing here sleeps: the pump asks `step` what is due and when the next look is.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use wormhole_core::config::{Config, FeatureId, FeatureMask};
use wormhole_core::git::pr::{self, Branch, PrCache};
use wormhole_core::project::{TrustStatus, find_project};
use wormhole_core::storage::paths::{DataRoot, ProfilePaths};
use wormhole_core::workspace::{StatusEvent, TerminalFacts};
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
        let trusted = approved(shared, &branch);
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
                let known = known.filter(|_| approved(&shared, &branch));
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

/// Whether `gh` may be started for `branch` and its pull request shown: what the GPUI app asks of each terminal
/// (`window/title/extras.rs:183-191`, `features/project/def.rs:123-128`). The Project feature is on in the window's
/// profile, by its gate and by the person's own switch (core's `FeatureMask`; `features/project/mod.rs:70-72`), and
/// the terminal's own project is approved there (`mod.rs:129`). The model holds a pull request per branch, not per
/// terminal, so every terminal on the branch must pass: a branch with a terminal that may not show it shows none.
fn approved(shared: &Shared, branch: &Branch) -> bool {
    let Some(profile) = profile_of(shared) else { return false };
    if !FeatureMask::resolve(&Config::load_for(&profile).settings).has(FeatureId::Project) {
        return false;
    }
    let folders = shared.peek_model(|model| {
        let on_branch = |facts: &&TerminalFacts| {
            facts.repo.as_deref() == Some(branch.root.as_path())
                && facts.branch.as_deref() == Some(branch.name.as_str())
        };
        let facts = model.ids().into_iter().filter_map(|id| model.facts(id));
        facts.filter(on_branch).map(|facts| facts.cwd.clone()).collect::<Vec<_>>()
    });
    let passes = |folder: &Option<PathBuf>| {
        folder.as_deref().is_some_and(|folder| in_approved_project(&profile, folder))
    };
    !folders.is_empty() && folders.iter().all(passes)
}

/// Whether the project `folder` is in is approved in `profile`: the nearest `.wormhole/` above it, never past its
/// repository's root (core's `find_project`, the GPUI app's `features/project/detect.rs:50`), and not the one at the
/// git root when a nearer one governs the folder. A folder in no project is in none that is approved.
fn in_approved_project(profile: &ProfilePaths, folder: &Path) -> bool {
    let Some(project) = find_project(folder) else { return false };
    // The profile's approvals as the process holds them (`files_panel::trust_status_in`), every window's revoke in them.
    crate::files_panel::trust_status_in(profile.dir(), &project.snapshot()) == TrustStatus::Trusted
}

/// The Project feature was switched in this window's profile: its branches are judged again at the next look, as
/// after a change of trust.
pub(crate) fn judge_again(shared: &Shared) {
    *locked(&shared.extras.scanned) = None;
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods, reason = "the test makes its repository in a temp folder")]

    use std::path::Path;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use wormhole_core::config::FeatureId;
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
                // The terminal's own folder is what its project is found from.
                facts.cwd = Some(root.clone());
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

    /// A window on a scratch data folder with one terminal on `main` of the repository at `root`, in `folder`, and
    /// a pull request `gh` answered a moment ago for that branch (nothing is due, so no lookup starts).
    fn window_on_main(data: &Path, root: &Path, folder: &Path) -> Arc<Workspace> {
        let windows = Windows::in_dir(data).expect("windows");
        let config = WorkspaceConfig {
            home: None,
            numbers_follow_list: true,
            start_folder: None,
            pane_header_height: 0.0,
        };
        let workspace = Workspace::in_window(windows, 1, config, Arc::new(Quiet));
        let branch = Branch { root: root.to_path_buf(), name: "main".into() };
        let found = PullRequest {
            number: 7,
            url: "https://github.invalid/o/r/pull/7".into(),
            state: PrState::Open,
            draft: false,
            checks: None,
            review: None,
        };
        {
            let mut cache = locked(&workspace.shared().extras.cache);
            assert!(cache.begin(&branch, Instant::now()));
            let _ = cache.finish(&branch, Ok(Some(found)), Instant::now());
        }
        workspace.apply(Action::NewTerminal);
        let id = workspace.selected().expect("a terminal");
        let _ = workspace.shared().with_model(|model| {
            model.update_facts(id, |facts| {
                facts.cwd = Some(folder.to_path_buf());
                facts.repo = Some(root.to_path_buf());
                facts.branch = Some("main".into());
            })
        });
        workspace
    }

    fn trust(profile: &Path, project: &Path) {
        let folder = ProjectFolder::open(project).expect("a project");
        crate::files_panel::change_trust_in(profile, |trust| {
            trust.trust(&folder);
            true
        })
        .expect("trusted");
    }

    fn shown(workspace: &Workspace) -> bool {
        pull_requests(workspace.shared());
        workspace.title("Macintosh HD".into()).pull_request_words.is_some()
    }

    // GPUI: `features/project/mod.rs:70-72,129`: with the Project feature switched off in Settings no terminal is in a
    // project, so `programs_approved` is false and the row neither asks `gh` nor shows what it said.
    // Order: the project is trusted and its pull request shown; the person switches the Project feature off.
    #[test]
    fn a_pull_request_is_not_shown_with_the_project_feature_switched_off() {
        let data = tempfile::tempdir().expect("data dir");
        let repo = tempfile::tempdir().expect("repo");
        std::fs::create_dir(repo.path().join(".git")).expect("git");
        std::fs::create_dir(repo.path().join(".wormhole")).expect("project folder");
        let root = repo.path().canonicalize().expect("canonical");
        let workspace = window_on_main(data.path(), &root, &root);
        let profile = workspace.shared().windows().expect("in a window").0.profile().clone();
        let config = profile.config_file();
        // The gate is on whatever a test build's default is: only the person's switch differs between the two.
        std::fs::write(&config, r#"{ "feature_flags": { "project_folders": true } }"#).expect("config.json");
        trust(profile.dir(), &root);
        eventually("the chip while the feature is on", || shown(&workspace));

        std::fs::write(
            &config,
            r#"{ "feature_flags": { "project_folders": true }, "features": { "project": false } }"#,
        )
        .expect("config.json");
        // As Swift tells the window when the profile's switches moved.
        workspace.apply(Action::SetFeature { feature: FeatureId::Project, on: false });
        assert!(!shown(&workspace), "switched off by the person: no chip, and `gh` is not asked");

        std::fs::write(&config, r#"{ "feature_flags": { "project_folders": true } }"#).expect("config.json");
        workspace.apply(Action::SetFeature { feature: FeatureId::Project, on: true });
        eventually("the chip again once it is switched back on", || shown(&workspace));
    }

    // GPUI: a terminal's project is the nearest `.wormhole/` above its folder (`wormhole-core/src/project/mod.rs:191-205`,
    // `features/project/detect.rs:50`), and that one's trust decides (`def.rs:123-128`).
    // Order: the repository's root project is trusted; the terminal sits in `sub/`, which has a project of its own
    // that nobody approved. Then `sub`'s is approved and the root's revoked.
    #[test]
    fn the_terminals_nearest_project_decides_not_the_one_at_the_git_root() {
        let data = tempfile::tempdir().expect("data dir");
        let repo = tempfile::tempdir().expect("repo");
        std::fs::create_dir(repo.path().join(".git")).expect("git");
        std::fs::create_dir(repo.path().join(".wormhole")).expect("the root's project");
        std::fs::create_dir_all(repo.path().join("sub/.wormhole")).expect("sub's project");
        let root = repo.path().canonicalize().expect("canonical");
        let sub = root.join("sub");
        let workspace = window_on_main(data.path(), &root, &sub);
        let profile = workspace.shared().windows().expect("in a window").0.profile().clone();
        std::fs::write(profile.config_file(), r#"{ "feature_flags": { "project_folders": true } }"#)
            .expect("config.json");

        trust(profile.dir(), &root);
        assert!(!shown(&workspace), "the root's trust says nothing for a terminal in `sub`'s project");

        trust(profile.dir(), &sub);
        crate::files_panel::change_trust_in(profile.dir(), |trust| trust.revoke(&root)).expect("revoked");
        eventually("the chip once the terminal's own project is trusted", || shown(&workspace));
    }
}
