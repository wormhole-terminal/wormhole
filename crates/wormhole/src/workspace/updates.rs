//! Self-updates: scheduled and manual checks against the release feed,
//! download + install in the background, restart when the user is ready.
//!
//! The feed URL and Apple team come from the release build's environment
//! (`WORMHOLE_UPDATE_URL`, `WORMHOLE_TEAM_ID`); development builds have no
//! URL and never check. The public key is `packaging/updater.pub`, the same
//! file `scripts/package.sh` checks the signing key against.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use wormhole_core::update::{
    self, DownloadedUpdate, Release, UpdateError, UpdateSchedule, UpdateStatus, UpdaterConfig,
};

use wormhole_core::store::Store;

use super::app::{app, app_mut};
use super::*;

const BUNDLE_ID: &str = "app.getwormhole.terminal";
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
    /// Installed over the running app; takes effect on restart.
    Installed(String),
    Failed(String),
}

pub(crate) struct Updates {
    config: Option<UpdaterConfig>,
    schedule: UpdateSchedule,
    pub phase: UpdatePhase,
    /// Bytes downloaded / total (0 = unknown), written from the download thread.
    progress: Arc<(AtomicU64, AtomicU64)>,
    task: Option<Task<()>>,
}

impl Updates {
    pub fn new(store: Option<&Store>) -> Self {
        let schedule = store
            .and_then(|s| s.meta(SCHEDULE_KEY).ok().flatten())
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        Self {
            config: feed_config(),
            schedule,
            phase: UpdatePhase::Idle,
            progress: Default::default(),
            task: None,
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

    /// When the last check that reached the feed ended (Unix seconds).
    pub fn last_check(&self) -> Option<u64> {
        self.schedule.last_check
    }
}

fn feed_config() -> Option<UpdaterConfig> {
    let url = option_env!("WORMHOLE_UPDATE_URL").filter(|s| !s.is_empty())?;
    let key = include_str!("../../../../packaging/updater.pub");
    let version = semver::Version::parse(env!("CARGO_PKG_VERSION")).ok()?;
    let mut config = UpdaterConfig::new(url, key, version, BUNDLE_ID);
    config.team_id = option_env!("WORMHOLE_TEAM_ID").filter(|s| !s.is_empty()).map(str::to_string);
    Some(config)
}

/// Called from the app tick: automatic checks at launch and every few hours.
pub(super) fn schedule_check(ticks: u64, cx: &mut App) {
    // Once a minute is plenty for a schedule measured in hours.
    if !ticks.is_multiple_of(150) && ticks != 5 {
        return;
    }
    let updates = &app(cx).updates;
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

/// `manual`: the user asked, so show every result (even a skipped version).
pub(crate) fn check_for_updates(manual: bool, cx: &mut App) {
    let updates = &mut app_mut(cx).updates;
    let Some(config) = updates.config.clone() else {
        updates.phase = UpdatePhase::Failed("This is a development build; it doesn't update itself.".into());
        notify_settings(cx);
        return;
    };
    if updates.busy() {
        return;
    }
    updates.phase = UpdatePhase::Checking;
    notify_settings(cx);
    let check = cx.background_executor().spawn(async move { update::check(&config) });
    let task = cx.spawn(async move |cx| {
        let result = check.await;
        cx.update(|cx| finish_check(result, manual, cx));
    });
    app_mut(cx).updates.task = Some(task);
}

/// Whether a check that ended like this counts as having been made. Not after a network error
/// (`UpdateSchedule::record_check`): a laptop that launches before Wi-Fi is up must try again
/// at the next opportunity, not in six hours.
fn should_record(result: &Result<UpdateStatus, UpdateError>) -> bool {
    !matches!(result, Err(UpdateError::Network(_)))
}

/// Only the settings window shows the updater's state, so it is the only one to redraw (and not
/// every window, several times a second while downloading). Notifying by id rather than updating
/// the entity: this is called from the window's own click handlers, which have it leased.
fn notify_settings(cx: &mut App) {
    if let Some((_, view)) = &app(cx).settings_window {
        let id = view.entity_id();
        cx.notify(id);
    }
}

fn finish_check(result: Result<UpdateStatus, UpdateError>, manual: bool, cx: &mut App) {
    if should_record(&result) {
        app_mut(cx).updates.schedule.record_check(SystemTime::now());
        save_update_schedule(cx);
    }
    let quiet = match &result {
        Ok(status) if !manual => !app(cx).updates.schedule.should_notify(status),
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
            app_mut(cx).updates.phase = UpdatePhase::Available(Box::new(release.clone()));
            if settings(cx).auto_update {
                install_update(release, cx);
            }
            notify_settings(cx);
            return;
        }
        Err(err) => {
            tracing::warn!("update check failed: {err}");
            // Automatic checks fail quietly (offline laptops); manual ones say why.
            if manual { UpdatePhase::Failed(err.to_string()) } else { UpdatePhase::Idle }
        }
    };
    app_mut(cx).updates.phase = phase;
    notify_settings(cx);
}

/// Download, verify and swap in the new bundle; the running app keeps going until restart.
pub(crate) fn install_update(release: Release, cx: &mut App) {
    let updates = &mut app_mut(cx).updates;
    let Some(config) = updates.config.clone() else { return };
    let Some(bundle) = update::current_app_bundle() else {
        updates.phase = UpdatePhase::Failed("Wormhole isn't running from an app bundle.".into());
        notify_settings(cx);
        return;
    };
    // Catch "move me to Applications" and permission problems before downloading.
    if let Err(err) = update::preflight(&bundle) {
        updates.phase = UpdatePhase::Failed(err.to_string());
        notify_settings(cx);
        return;
    }
    updates.phase = UpdatePhase::Downloading;
    let progress = updates.progress.clone();
    progress.0.store(0, Ordering::Relaxed);
    progress.1.store(release.asset.size, Ordering::Relaxed);
    notify_settings(cx);

    let version = release.version.to_string();
    let work = cx.background_executor().spawn(async move {
        let downloaded: DownloadedUpdate = update::download(&config, &release, |done, _| {
            progress.0.store(done, Ordering::Relaxed);
        })?;
        update::install(&config, &downloaded, &bundle)
    });
    let task = cx.spawn(async move |cx| {
        let result = work.await;
        cx.update(|cx| {
            app_mut(cx).updates.phase = match result {
                Ok(()) => UpdatePhase::Installed(version),
                Err(err) => {
                    tracing::error!("update failed: {err}");
                    UpdatePhase::Failed(err.to_string())
                }
            };
            notify_settings(cx);
        });
    });
    app_mut(cx).updates.task = Some(task);
    // Redraw the progress while downloading.
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(Duration::from_millis(250)).await;
            let downloading = cx.update(|cx| {
                notify_settings(cx);
                matches!(app(cx).updates.phase, UpdatePhase::Downloading)
            });
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
    let updates = &mut app_mut(cx).updates;
    updates.schedule.skip(&release.version);
    updates.phase = UpdatePhase::Idle;
    save_update_schedule(cx);
    notify_settings(cx);
}

pub(crate) fn restart_to_update(cx: &mut App) {
    let Some(bundle) = update::current_app_bundle() else { return };
    match update::relaunch(&bundle) {
        // Every window is saved on the way out.
        Ok(()) => cx.quit(),
        Err(err) => {
            app_mut(cx).updates.phase = UpdatePhase::Failed(format!("Couldn't restart: {err}"));
            notify_settings(cx);
        }
    }
}

fn save_update_schedule(cx: &App) {
    let state = app(cx);
    let Some(store) = &state.store else { return };
    let json = serde_json::to_string(&state.updates.schedule).unwrap_or_default();
    if let Err(err) = store.set_meta(SCHEDULE_KEY, &json) {
        tracing::warn!("saving update schedule: {err:#}");
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::update::{UpdateError, UpdateStatus};

    use super::should_record;

    #[test]
    fn an_offline_check_is_not_recorded_so_it_is_retried() {
        assert!(!should_record(&Err(UpdateError::Network("dns error".into()))));
    }

    #[test]
    fn every_check_that_reached_the_server_is_recorded() {
        assert!(should_record(&Ok(UpdateStatus::UpToDate)));
        // A broken feed answered, so retrying in a minute would not help.
        assert!(should_record(&Err(UpdateError::Feed("not json".into()))));
        assert!(should_record(&Err(UpdateError::Signature("bad".into()))));
    }
}
