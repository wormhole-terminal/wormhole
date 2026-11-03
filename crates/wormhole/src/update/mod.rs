//! Self-updates: scheduled and manual checks against the release feed,
//! download + install in the background, restart when the user is ready.
//!
//! Above the window and the shared state. `Updates` is a global of its own (set in `app::launch`),
//! which Settings and the status item read through these functions.
//!
//! The feed URL and Apple team come from the release build's environment
//! (`WORMHOLE_UPDATE_URL`, `WORMHOLE_TEAM_ID`); development builds have no
//! URL and never check. Which feeds a check reads is the `update_channel` setting. The public key
//! is `packaging/updater.pub`, the same file `scripts/package.sh` checks the signing key against.

mod prompt;
pub(crate) mod restart;
#[cfg(test)]
pub(crate) use prompt::announce as announce_installed;
pub(crate) use prompt::keep_offered;
#[cfg(test)]
mod restart_tests;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use gpui_kit::*;
use wormhole_core::storage::store::Store;
use wormhole_core::update::{
    self, Checked, Count, DownloadedUpdate, Held, Launched, OnSwitch, Release, UpdateChannel, UpdateError,
    UpdateSchedule, UpdateStatus, UpdaterConfig,
};

use crate::settings::{SettingsStore, settings};
use crate::update::restart::IdleWatch;

const BUNDLE_ID: &str = "app.getwormhole.terminal";
/// The releases the download server redirects to (`.github/workflows/release.yml`).
const GITHUB_RELEASES: &str = "https://github.com/wormhole-terminal/wormhole/releases";
const SCHEDULE_KEY: &str = "update_schedule";

#[derive(Clone)]
pub(crate) enum UpdatePhase {
    Idle,
    Checking,
    UpToDate,
    /// Boxed: a release is far bigger than the other phases.
    Available(Box<Release>),
    /// Newer version exists, but it needs a newer macOS.
    Unsupported(String),
    Downloading,
    /// Installed over the running app (by it, or by something else: see [`watch_bundle`]); takes
    /// effect on restart. The version on disk, as far as it is known.
    Installed(String),
    Failed(String),
}

/// What a change of channel did to the build it found, for Settings to say (`channel_note`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ChannelNote {
    /// An Unstable build was on its way; Stable called it off before it replaced anything.
    Removed(String),
    /// An Unstable build is already installed; Stable takes over at the next release newer than it.
    Stays(String),
}

/// The download-and-install in flight, so a change of channel can call it off.
struct Installing {
    version: semver::Version,
    /// Read by the install right before it swaps the bundle (`update::install_unless`).
    called_off: Arc<AtomicBool>,
}

pub(crate) struct Updates {
    config: Option<UpdaterConfig>,
    schedule: UpdateSchedule,
    schedule_file: Option<PathBuf>,
    pub phase: UpdatePhase,
    /// Bytes downloaded / total (0 = unknown), written from the download thread.
    progress: Arc<(AtomicU64, AtomicU64)>,
    task: Option<Task<()>>,
    /// "Restart When Idle" was chosen: the app restarts once its terminals have been idle for a
    /// while (`update::restart::tick`). Not saved: a request lives as long as the run that made it.
    restart_when_idle: Option<IdleWatch>,
    /// The channel the shown phase belongs to; switching it checks the new one (see [`init`]).
    channel: UpdateChannel,
    /// A version whose download or install failed for a reason a retry won't fix (not the network):
    /// automatic checks offer it with its Install button instead of downloading it again every six
    /// hours. Not saved: a new run tries once more.
    failed_install: Option<semver::Version>,
    /// The executable this run started from, while it is in a bundle that updates itself (a release
    /// build): what [`watch_bundle`] compares the one on disk with. `None` for a dev build.
    launched: Option<Launched>,
    /// A look at the bundle is under way on a background thread.
    watching: bool,
    /// The version the notice chip last told about and the number of that install (`prompt`).
    told: Option<(String, u32)>,
    /// The version whose offer the person closed: not offered again, in any window, until a newer one.
    dismissed: Option<String>,
    /// The window (`OpenWindow::id`) that has the offer now, so it moves when the front window changes.
    offered_in: Option<i64>,
    installing: Option<Installing>,
    /// What the last change of channel did to a build of the channel left, until the next change.
    pub channel_note: Option<ChannelNote>,
}

impl Global for Updates {}

impl Updates {
    /// `schedule_file` is the one every profile shares (`DataRoot::update_schedule_file`); a
    /// version from before profiles kept the schedule in the database, which seeds the file.
    pub fn new(schedule_file: Option<PathBuf>, store: Option<&Store>) -> Self {
        let legacy = || store.and_then(|s| s.meta(SCHEDULE_KEY).ok().flatten());
        let mut schedule = match &schedule_file {
            Some(file) => UpdateSchedule::load_shared(file, legacy),
            None => legacy().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default(),
        };
        // A day before this copy is counted, from the first launch of a version that counts, so
        // there is time to turn the count off (Settings → Updates).
        if schedule.start_count_grace(SystemTime::now())
            && let Some(file) = &schedule_file
            && let Err(err) = schedule.save_shared(file)
        {
            tracing::warn!("saving update schedule: {err:#}");
        }
        Self {
            config: feed_config(),
            schedule_file,
            schedule,
            phase: UpdatePhase::Idle,
            progress: Default::default(),
            task: None,
            restart_when_idle: None,
            channel: UpdateChannel::Stable,
            failed_install: None,
            launched: feed_config().and_then(|_| Launched::capture(env!("CARGO_PKG_VERSION"))),
            watching: false,
            told: None,
            dismissed: None,
            offered_in: None,
            installing: None,
            channel_note: None,
        }
    }

    /// False for development builds, which have no feed.
    pub fn available(&self) -> bool {
        self.config.is_some()
    }

    pub fn busy(&self) -> bool {
        matches!(self.phase, UpdatePhase::Checking | UpdatePhase::Downloading)
    }

    pub fn progress(&self) -> (u64, u64) {
        (self.progress.0.load(Ordering::Relaxed), self.progress.1.load(Ordering::Relaxed))
    }

    /// The update the app holds, and what the new `channel` makes of it.
    fn on_switch(&self, channel: UpdateChannel) -> (OnSwitch, Option<semver::Version>) {
        let installed = match &self.phase {
            UpdatePhase::Installed(version) => semver::Version::parse(version).ok(),
            _ => None,
        };
        let on_its_way = self.installing.as_ref().filter(|_| self.phase_is_downloading());
        let held = match (&installed, on_its_way) {
            (Some(version), _) => Held::Installed(version),
            (None, Some(installing)) => Held::OnItsWay(&installing.version),
            (None, None) => Held::Nothing,
        };
        let version = installed.clone().or_else(|| on_its_way.map(|i| i.version.clone()));
        (update::on_channel_switch(held, channel), version)
    }

    fn phase_is_downloading(&self) -> bool {
        matches!(self.phase, UpdatePhase::Downloading)
    }

    /// Calls the download-and-install off. Its thread keeps running until it next looks at the flag
    /// (a download is a blocking read), but it cannot get past the swap, and its result is not heard.
    fn call_off_install(&mut self, version: semver::Version) {
        if let Some(installing) = self.installing.take() {
            installing.called_off.store(true, Ordering::Release);
        }
        self.task = None;
        self.phase = UpdatePhase::Idle;
        self.channel_note = Some(ChannelNote::Removed(version.to_string()));
    }

    /// Whether the app is waiting for its terminals to be idle to restart.
    pub fn restart_pending(&self) -> bool {
        self.restart_when_idle.is_some()
    }

    fn restart_watch(&mut self) -> Option<&mut IdleWatch> {
        self.restart_when_idle.as_mut()
    }

    /// When the last check that reached the feed ended (Unix seconds).
    pub fn last_check(&self) -> Option<u64> {
        self.schedule.last_check
    }

    /// How long until this copy is counted, while it waits (`UpdateSchedule::count_starts_in`).
    pub fn count_starts_in(&self) -> Option<Duration> {
        self.schedule.count_starts_in(SystemTime::now())
    }
}

fn feed_config() -> Option<UpdaterConfig> {
    let url = option_env!("WORMHOLE_UPDATE_URL").filter(|s| !s.is_empty())?;
    let key = include_str!("../../../../packaging/updater.pub");
    let version = semver::Version::parse(env!("CARGO_PKG_VERSION")).ok()?;
    let mut config = UpdaterConfig::new(url, key, version, BUNDLE_ID);
    config.team_id = option_env!("WORMHOLE_TEAM_ID").filter(|s| !s.is_empty()).map(str::to_string);
    // Where the release workflow publishes, for when the download server doesn't answer.
    config.fallback_url = Some(GITHUB_RELEASES.to_string());
    Some(config)
}

/// Follows the `update_channel` setting: once `app::launch` has set the `Updates` global, and
/// whenever settings change.
pub(crate) fn init(cx: &mut App) {
    let channel = settings(cx).update_channel;
    cx.global_mut::<Updates>().channel = channel;
    cx.observe_global::<SettingsStore>(channel_changed).detach();
}

/// What the old channel offered (or that it had nothing) no longer holds, so the new one is asked
/// at once, as if the person had pressed Check: they just made a choice about updates.
///
/// A build the new channel would not have offered is dealt with first (`wormhole_core::update::held`):
/// one still on its way is called off, one already installed stays and is said so.
fn channel_changed(cx: &mut App) {
    let channel = settings(cx).update_channel;
    let updates = cx.global_mut::<Updates>();
    if updates.channel == channel {
        return;
    }
    updates.channel = channel;
    updates.channel_note = None;
    match updates.on_switch(channel) {
        (OnSwitch::Keep, _) => {}
        (OnSwitch::CallOff, Some(version)) => updates.call_off_install(version),
        (OnSwitch::StaysInstalled, Some(version)) => {
            updates.channel_note = Some(ChannelNote::Stays(version.to_string()));
            return;
        }
        (_, None) => {}
    }
    let updates = cx.global_mut::<Updates>();
    if !updates.available() || updates.busy() || matches!(updates.phase, UpdatePhase::Installed(_)) {
        return;
    }
    check_for_updates(true, cx);
}

/// Called from the app tick: automatic checks at launch and every few hours.
pub(crate) fn schedule_check(ticks: u64, cx: &mut App) {
    // Once a minute is plenty for a schedule measured in hours.
    if !ticks.is_multiple_of(150) && ticks != 5 {
        return;
    }
    let updates = cx.global::<Updates>();
    if !settings(cx).auto_update || !updates.available() || updates.busy() {
        return;
    }
    if matches!(updates.phase, UpdatePhase::Installed(_) | UpdatePhase::Available(_)) {
        return;
    }
    let now = SystemTime::now();
    let due = if ticks == 5 { updates.schedule.due_at_launch(now) } else { updates.schedule.is_due(now) };
    if due {
        check_for_updates(false, cx);
    }
}

/// Called from the app tick: once a minute, whether the bundle this run started from was replaced on
/// disk (an update made outside the app, say). Then the app is in the state an in-app install leaves
/// it in, `Installed`, so Settings offers the restart that finishes it (and "Restart When Idle").
/// Only another version counts (see `wormhole_core::update::replaced`): the same one copied again
/// changes nothing the running app depends on.
///
/// Only release builds are watched (`Updates::launched`): a dev build is rebuilt under its own feet
/// all day. The look is a few file system calls, which a slow volume could hang, so it is made off
/// the UI thread.
pub(crate) fn watch_bundle(ticks: u64, cx: &mut App) {
    if !ticks.is_multiple_of(150) {
        return;
    }
    let updates = cx.global_mut::<Updates>();
    if updates.watching || matches!(updates.phase, UpdatePhase::Installed(_) | UpdatePhase::Downloading) {
        return;
    }
    let Some(launched) = updates.launched.clone() else { return };
    updates.watching = true;
    let look = cx.background_executor().spawn(async move { launched.replaced() });
    cx.spawn(async move |cx| {
        let replaced = look.await;
        cx.update(|cx| {
            let updates = cx.global_mut::<Updates>();
            updates.watching = false;
            // An in-app install may have started meanwhile: it sets the phase itself.
            let free = !matches!(updates.phase, UpdatePhase::Installed(_) | UpdatePhase::Downloading);
            if let Some(replaced) = replaced.filter(|_| free) {
                tracing::info!(version = %replaced.version, "the app bundle was replaced while running");
                updates.phase = UpdatePhase::Installed(replaced.version);
                prompt::announce(cx);
            }
        });
    })
    .detach();
}

/// `manual`: the user asked, so show every result (even a skipped version).
pub(crate) fn check_for_updates(manual: bool, cx: &mut App) {
    let channel = settings(cx).update_channel;
    let counting = settings(cx).anonymous_count;
    let updates = cx.global_mut::<Updates>();
    let Some(mut config) = updates.config.clone() else {
        updates.phase = UpdatePhase::Failed("This is a development build; it doesn't update itself.".into());
        return;
    };
    if updates.busy() {
        return;
    }
    config.channel = channel;
    config.count = if counting { updates.schedule.count_due(SystemTime::now()) } else { None };
    let count = config.count;
    updates.phase = UpdatePhase::Checking;
    let check = cx.background_executor().spawn(async move { update::check_counted(&config) });
    let task = cx.spawn(async move |cx| {
        let result = check.await;
        cx.update(|cx| finish_check(result, manual, count, cx));
    });
    cx.global_mut::<Updates>().task = Some(task);
}

/// Whether a check that ended like this counts as having been made. Not after a network error
/// (`UpdateSchedule::record_check`): a laptop that launches before Wi-Fi is up must try again
/// at the next opportunity, not in six hours.
fn should_record<T>(result: &Result<T, UpdateError>) -> bool {
    !matches!(result, Err(UpdateError::Network(_)))
}

/// `count`: what the check said for the anonymous count, recorded with the check so each day and
/// month is flagged once.
fn finish_check(result: Result<Checked, UpdateError>, manual: bool, count: Option<Count>, cx: &mut App) {
    if should_record(&result) {
        let now = SystemTime::now();
        let schedule = &mut cx.global_mut::<Updates>().schedule;
        schedule.record_check(now);
        // Only a count that reached the download server: from GitHub's copy of the feed, the day
        // is counted at a later check.
        if let (Some(count), Ok(Checked { counted: true, .. })) = (count, &result) {
            schedule.record_count(count, now);
        }
        save_update_schedule(cx);
    }
    let result = result.map(|checked| checked.status);
    let quiet = match &result {
        Ok(status) if !manual => !cx.global::<Updates>().schedule.should_notify(status),
        _ => false,
    };
    let phase = match result {
        Ok(_) if quiet => UpdatePhase::Idle,
        Ok(UpdateStatus::UpToDate) => UpdatePhase::UpToDate,
        Ok(UpdateStatus::Unsupported { version, minimum_system_version }) => UpdatePhase::Unsupported(
            format!("Version {version} needs macOS {minimum_system_version} or later."),
        ),
        Ok(UpdateStatus::Available(release)) => {
            tracing::info!(version = %release.version, "update available");
            let updates = cx.global_mut::<Updates>();
            updates.phase = UpdatePhase::Available(Box::new(release.clone()));
            let failed_before = updates.failed_install.as_ref() == Some(&release.version);
            if settings(cx).auto_update && (manual || !failed_before) {
                install_update(release, cx);
            }
            return;
        }
        Err(err) => {
            tracing::warn!("update check failed: {err}");
            // Automatic checks fail quietly (offline laptops); manual ones say why.
            if manual { UpdatePhase::Failed(err.to_string()) } else { UpdatePhase::Idle }
        }
    };
    cx.global_mut::<Updates>().phase = phase;
}

/// Download, verify and swap in the new bundle; the running app keeps going until restart.
pub(crate) fn install_update(release: Release, cx: &mut App) {
    let updates = cx.global_mut::<Updates>();
    let Some(config) = updates.config.clone() else { return };
    let Some(bundle) = update::current_app_bundle() else {
        updates.phase = UpdatePhase::Failed("Wormhole isn't running from an app bundle.".into());
        return;
    };
    // Catch "move me to Applications" and permission problems before downloading.
    if let Err(err) = update::preflight(&bundle) {
        updates.phase = UpdatePhase::Failed(err.to_string());
        return;
    }
    updates.phase = UpdatePhase::Downloading;
    let called_off = Arc::new(AtomicBool::new(false));
    let progress = updates.progress.clone();
    progress.0.store(0, Ordering::Relaxed);
    progress.1.store(release.asset.size, Ordering::Relaxed);
    updates.installing =
        Some(Installing { version: release.version.clone(), called_off: called_off.clone() });

    let version = release.version.to_string();
    let semver = release.version.clone();
    let flag = called_off.clone();
    let work = cx.background_executor().spawn(async move {
        let downloaded: DownloadedUpdate = update::download(&config, &release, |done, _| {
            progress.0.store(done, Ordering::Relaxed);
        })?;
        update::install_unless(&config, &downloaded, &bundle, &flag)
    });
    let task = cx.spawn(async move |cx| {
        let result = work.await;
        cx.update(|cx| {
            let updates = cx.global_mut::<Updates>();
            // Called off by a change of channel: the person has moved on, so the result is not theirs.
            if called_off.load(Ordering::Acquire) {
                return;
            }
            updates.installing = None;
            updates.phase = match result {
                Ok(()) => UpdatePhase::Installed(version),
                Err(err) => {
                    tracing::error!("update failed: {err}");
                    if !matches!(err, UpdateError::Network(_)) {
                        updates.failed_install = Some(semver);
                    }
                    UpdatePhase::Failed(err.to_string())
                }
            };
            prompt::announce(cx);
        });
    });
    cx.global_mut::<Updates>().task = Some(task);
    // Redraw the progress while downloading.
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(Duration::from_millis(250)).await;
            let downloading =
                cx.update(|cx| matches!(cx.global::<Updates>().phase, UpdatePhase::Downloading));
            if !downloading {
                break;
            }
        }
    })
    .detach();
}

pub(crate) fn skip_update(release: &Release, cx: &mut App) {
    // The button isn't shown for these; a skipped security update would be offered again anyway.
    if release.is_security_update() {
        return;
    }
    let updates = cx.global_mut::<Updates>();
    updates.schedule.skip(&release.version);
    updates.phase = UpdatePhase::Idle;
    save_update_schedule(cx);
}

/// Restart into the installed update now, whatever runs (the callers have asked, or checked).
pub(crate) fn restart_to_update(cx: &mut App) {
    // Whatever happens next, the request is answered: a failed restart is reported, not retried.
    cx.global_mut::<Updates>().restart_when_idle = None;
    let Some(bundle) = update::current_app_bundle() else {
        return;
    };
    match update::relaunch(&bundle) {
        // Every window is saved on the way out.
        Ok(()) => cx.quit(),
        Err(err) => {
            cx.global_mut::<Updates>().phase = UpdatePhase::Failed(format!("Couldn't restart: {err}"));
        }
    }
}

/// Restart once the terminals have been idle for a while (the tick settles it).
pub(crate) fn restart_when_idle(cx: &mut App) {
    let updates = cx.global_mut::<Updates>();
    if matches!(updates.phase, UpdatePhase::Installed(_)) {
        updates.restart_when_idle = Some(IdleWatch::default());
    }
}

/// Drop the request to restart when idle.
pub(crate) fn cancel_restart_when_idle(cx: &mut App) {
    cx.global_mut::<Updates>().restart_when_idle = None;
}

/// devctl `update-installed`: Settings → Updates as it is once an update is installed, with
/// "Restart When Idle" chosen when `pending` (a dev build has no feed to install from).
#[cfg(feature = "devctl")]
pub(crate) fn devctl_installed(version: &str, pending: bool, cx: &mut App) {
    let updates = cx.global_mut::<Updates>();
    updates.phase = UpdatePhase::Installed(version.to_string());
    updates.restart_when_idle = pending.then(IdleWatch::default);
    prompt::announce(cx);
}

/// devctl `update-state <state>`: Settings → Updates as it is in one state, from fixtures, without a
/// feed, a download or an install (a dev build has none). The names are in `devctl/mod.rs`.
#[cfg(feature = "devctl")]
pub(crate) fn devctl_state(state: &str, cx: &mut App) -> Result<(), String> {
    use wormhole_core::update::Asset;
    let release = |version: &str| {
        Release {
        version: semver::Version::parse(version).expect("a fixture version"),
        notes: "## Added\n\n- Settings shows each page's **live preview** as a card.\n- A `config.json` key for \
                [borders](https://example.invalid/borders).\n\n## Fixed\n\n- After a restart, the window that was in \
                front is in front again.\n- Going back to Stable no longer keeps an Unstable build.\n\nThanks for \
                the reports."
            .into(),
        pub_date: None,
        minimum_system_version: None,
        asset: Asset { url: String::new(), sha256: String::new(), signature: String::new(), size: 100 },
        security: None,
    }
    };
    let updates = cx.global_mut::<Updates>();
    updates.channel_note = None;
    updates.installing = None;
    updates.restart_when_idle = None;
    updates.phase = match state {
        "idle" => UpdatePhase::Idle,
        "checking" => UpdatePhase::Checking,
        "up-to-date" => UpdatePhase::UpToDate,
        "available" => UpdatePhase::Available(Box::new(release("1.7.0"))),
        "downloading" => {
            updates.progress.0.store(40, Ordering::Relaxed);
            updates.progress.1.store(100, Ordering::Relaxed);
            UpdatePhase::Downloading
        }
        "installed" => UpdatePhase::Installed("1.7.0".into()),
        "failed" => UpdatePhase::Failed("network error: the server did not answer".into()),
        "removed" => {
            updates.channel_note = Some(ChannelNote::Removed("1.7.0-dev.12".into()));
            UpdatePhase::Idle
        }
        "stays" => {
            updates.channel_note = Some(ChannelNote::Stays("1.7.0-dev.12".into()));
            UpdatePhase::Installed("1.7.0-dev.12".into())
        }
        other => return Err(format!("unknown update state {other:?}")),
    };
    Ok(())
}

fn save_update_schedule(cx: &App) {
    let updates = cx.global::<Updates>();
    let Some(file) = &updates.schedule_file else { return };
    if let Err(err) = updates.schedule.save_shared(file) {
        tracing::warn!("saving update schedule: {err:#}");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use wormhole_core::update::{OnSwitch, UpdateChannel, UpdateError, UpdateStatus};

    use super::{ChannelNote, Installing, UpdatePhase, Updates, should_record};

    fn updates(phase: UpdatePhase) -> Updates {
        let mut updates = Updates::new(None, None);
        updates.phase = phase;
        updates
    }

    fn on_its_way(version: &str) -> (Updates, Arc<AtomicBool>) {
        let mut updates = updates(UpdatePhase::Downloading);
        let called_off = Arc::new(AtomicBool::new(false));
        updates.installing = Some(Installing {
            version: semver::Version::parse(version).expect("a version"),
            called_off: called_off.clone(),
        });
        (updates, called_off)
    }

    #[test]
    fn stable_calls_off_an_unstable_download_and_says_so() {
        let (mut updates, called_off) = on_its_way("1.7.0-dev.3");
        let (switch, version) = updates.on_switch(UpdateChannel::Stable);
        assert_eq!(switch, OnSwitch::CallOff);
        updates.call_off_install(version.expect("the version"));
        assert!(called_off.load(Ordering::Acquire), "the install is told before it swaps");
        assert!(matches!(updates.phase, UpdatePhase::Idle), "no Restart, no progress");
        assert_eq!(updates.channel_note, Some(ChannelNote::Removed("1.7.0-dev.3".into())));
        assert!(updates.installing.is_none());
    }

    #[test]
    fn a_download_the_new_channel_also_offers_goes_on() {
        let (updates, called_off) = on_its_way("1.7.0");
        assert_eq!(updates.on_switch(UpdateChannel::Stable).0, OnSwitch::Keep);
        let (updates, _) = on_its_way("1.7.0-dev.3");
        assert_eq!(updates.on_switch(UpdateChannel::Unstable).0, OnSwitch::Keep);
        assert!(!called_off.load(Ordering::Acquire));
    }

    #[test]
    fn an_unstable_build_already_installed_stays_and_is_not_rolled_back() {
        let held = updates(UpdatePhase::Installed("1.7.0-dev.3".into()));
        let (switch, version) = held.on_switch(UpdateChannel::Stable);
        assert_eq!(switch, OnSwitch::StaysInstalled);
        assert_eq!(version.map(|v| v.to_string()).as_deref(), Some("1.7.0-dev.3"));
        // A stable one, or one found on disk with only a plain version, is nothing to say.
        assert_eq!(
            updates(UpdatePhase::Installed("1.7.0".into())).on_switch(UpdateChannel::Stable).0,
            OnSwitch::Keep
        );
    }

    #[test]
    fn nothing_held_changes_nothing() {
        assert_eq!(updates(UpdatePhase::UpToDate).on_switch(UpdateChannel::Stable).0, OnSwitch::Keep);
        assert_eq!(updates(UpdatePhase::Idle).on_switch(UpdateChannel::Unstable).0, OnSwitch::Keep);
    }

    #[test]
    fn an_offline_check_is_not_recorded_so_it_is_retried() {
        assert!(!should_record::<UpdateStatus>(&Err(UpdateError::Network("dns error".into()))));
    }

    #[test]
    fn every_check_that_reached_the_server_is_recorded() {
        assert!(should_record(&Ok(UpdateStatus::UpToDate)));
        // A broken feed answered, so retrying in a minute would not help.
        assert!(should_record::<UpdateStatus>(&Err(UpdateError::Feed("not json".into()))));
        assert!(should_record::<UpdateStatus>(&Err(UpdateError::Signature("bad".into()))));
    }
}
