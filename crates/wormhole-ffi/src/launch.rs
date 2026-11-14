//! What a terminal of the app runs and how its engine starts, decided as the GPUI app's window decides it
//! (`window/terminals.rs` `make_pane`, `terminal_view/mod.rs` `TerminalView::new`): the shell the terminal was
//! opened with, else a trusted project's, else `default_shell`, else the person's login shell as a login shell
//! (core `shells::for_terminal`); a trusted project's environment, then the profile's (environment, git identity,
//! agent logins, `WORMHOLE_PROFILE`, start folder); the engine options from the profile's settings; the theme's
//! colours (`engine_look`); and the saved text a restored terminal starts above. Every start (local, hosted,
//! attached, the welcome) takes its options from here, so an attach gives the engine what its start did.
//!
//! A program named in the request (`--shell`, the welcome's tour, tests) runs instead of the decided shell unless the
//! terminal was opened with one of its own, and the engine's own `WORMHOLE_SHELL` (test instances) replaces everything
//! in core `terminal::process`.

use std::path::{Path, PathBuf};

use wormhole_core::config::profile::ProfileList;
use wormhole_core::config::{Config, FeatureId, FeatureMask, Settings};
use wormhole_core::project::{self as proj, Feature, ProjectPrefs, TrustStore};
use wormhole_core::storage::paths::{DataRoot, ProfileId, ProfilePaths};
use wormhole_core::terminal::SpawnOptions;
use wormhole_core::terminal::shells::{self, ShellCommand};

use crate::hosting::HostedStart;

/// The options terminal `what` starts with, in profile `profile` of the data folder `root`.
pub(crate) fn options(root: &DataRoot, profile: &ProfilePaths, what: &HostedStart) -> SpawnOptions {
    let settings = Config::load_for(profile).settings;
    let cwd = what.cwd.as_deref().map(PathBuf::from).filter(|dir| dir.is_dir());
    let (project_env, project_shell) = project_adds(profile, &settings, cwd.as_deref()).unwrap_or_default();
    // A worktree Wormhole made gives its terminals their own ports (`git::worktree_env`); a trusted project's env goes on top.
    let mut env: std::collections::BTreeMap<String, String> = cwd
        .as_deref()
        .map(|cwd| crate::git::worktree_env(cwd, profile))
        .unwrap_or_default()
        .into_iter()
        .collect();
    env.extend(project_env);
    let shell = match (&what.program, what.shell.as_deref()) {
        // A program asked for by name (`--shell`, the welcome's tour, a test) runs as it is, unless the terminal was
        // opened with a shell of its own.
        (Some(program), None) => ShellCommand {
            name: shells::program_name(program).to_string(),
            program: program.clone(),
            args: what.args.clone(),
        },
        (_, chosen) => shells::for_terminal(chosen, project_shell.as_deref(), &settings),
    };
    // This profile's colours, as its windows draw them: not the shown profile's (`engine_look`).
    let look = crate::engine_look::current(&profile.id().to_string());
    let mut options = SpawnOptions {
        cwd,
        tab_id: what.tab.to_string(),
        restore: what.saved_text.as_deref().map(seed).filter(|seed| !seed.is_empty()),
        scrollback_lines: settings.scrollback_lines,
        image_budget: Some(settings.image_memory_bytes()),
        allow_screen_checksum: settings.allow_screen_checksum,
        allow_window_ops: settings.allow_window_ops,
        allow_title_report: settings.allow_title_report,
        env,
        shell: Some(shell),
        colors: look.as_ref().map(|look| look.colors.clone()),
        color_scheme: look.map(|look| look.scheme),
        ..SpawnOptions::default()
    };
    // After the project's environment: what a trusted project set is kept. The profile's environment is the
    // person's own (not repository content), so it applies in an untrusted project too.
    let (profiles, _) = ProfileList::load_or_recover(root);
    if let Some(found) = profiles.get(profile.id()) {
        options.apply_profile(found);
    }
    options
}

/// The profile `id` names in `root` (`None`, or one that is not there: the profile Wormhole is in).
pub(crate) fn profile_in(root: &DataRoot, id: Option<&str>) -> ProfilePaths {
    let (list, _) = ProfileList::load_or_recover(root);
    match id.and_then(|id| ProfileId::parse(id).ok()).filter(|id| list.get(id).is_some()) {
        Some(id) => root.profile(&id),
        None => root.profile(list.active_id()),
    }
}

/// A trusted project's environment and shell for a terminal starting in `cwd` (the app's `detect::spawn_options`):
/// only with project folders on, its Settings switched on, and its files approved as they are now. Read from disk at
/// the start, as what runs is decided; a store that does not read approves nothing.
fn project_adds(
    profile: &ProfilePaths,
    settings: &Settings,
    cwd: Option<&Path>,
) -> Option<(std::collections::BTreeMap<String, String>, Option<String>)> {
    if !FeatureMask::resolve(settings).has(FeatureId::Project) {
        return None;
    }
    let folder = proj::find_project(cwd?)?;
    let snapshot = folder.snapshot();
    let prefs = ProjectPrefs::load(profile.dir()).ok()?;
    let trust = TrustStore::load(profile.dir()).ok()?;
    let enabled = snapshot.has(Feature::Settings)
        && prefs.enabled(snapshot.root(), Feature::Settings)
        && trust.allows_in(&snapshot, Feature::Settings);
    let config = snapshot.config().filter(|_| enabled)?.clone();
    Some((config.env, config.shell))
}

/// The saved text as the seed a new shell starts above: dim, with the divider that says when.
fn seed(text: &str) -> String {
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs());
    wormhole_core::terminal::restored::seed_from_text(text, now)
}
