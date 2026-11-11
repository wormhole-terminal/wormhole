//! The updater's backend: core's check, download (size, sha256 and minisign verified), install (the bundle
//! swapped in beside the running one) and relaunch, run as jobs on threads of their own, with the phase machine
//! the GPUI app keeps in its `update` module as one state Swift reads after a wake (`UpdateStateRecord`).
//!
//! Nothing here quits the app: `restart` starts core's relaunch helper (it waits for this process to exit) and
//! the Swift side terminates right after. The feed is unsigned and only says where to look; the archive's
//! signature, against the key given at construction, is the authority (`wormhole_core::update`).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime};

use wormhole_core::storage::paths::DataRoot;
use wormhole_core::update::{
    self, Checked, Count, Held, OnSwitch, Release, UpdateChannel, UpdateError, UpdateSchedule, UpdateStatus,
    UpdaterConfig,
};

use super::common::{AuxError, AuxListener, Waker, locked, resolve_data_dir};
use super::update_records::{
    UpdateChannelNote, UpdateChannelRecord, UpdatePhaseRecord, UpdateReleaseRecord, UpdateSetup,
    UpdateStateRecord,
};

/// Progress wakes at most this often (the download reads in 64 KiB chunks).
const PROGRESS_EVERY: std::time::Duration = std::time::Duration::from_millis(100);

enum Phase {
    Idle,
    Checking,
    UpToDate,
    Available(Box<Release>),
    Unsupported(String),
    Downloading,
    Installed(String),
    Failed(String),
}

/// The download-and-install in flight, so a change of channel can call it off.
struct Installing {
    version: semver::Version,
    called_off: Arc<AtomicBool>,
}

struct State {
    phase: Phase,
    channel: UpdateChannel,
    /// Bumped when the channel changes: a check that began before it does not speak.
    generation: u64,
    installing: Option<Installing>,
    note: Option<(String, bool)>,
    /// A version whose install failed for a reason a retry will not fix.
    failed_install: Option<semver::Version>,
}

struct Shared {
    config: UpdaterConfig,
    bundle: Option<PathBuf>,
    schedule_file: PathBuf,
    schedule: Mutex<UpdateSchedule>,
    state: Mutex<State>,
    received: AtomicU64,
    expected: AtomicU64,
    /// `anonymous_count` and `auto_update`, as Settings says now (both on until told).
    counting: AtomicBool,
    auto_install: AtomicBool,
    waker: Waker,
}

#[derive(uniffi::Object)]
pub struct UpdateEngine {
    shared: Arc<Shared>,
}

fn core_channel(channel: UpdateChannelRecord) -> UpdateChannel {
    match channel {
        UpdateChannelRecord::Stable => UpdateChannel::Stable,
        UpdateChannelRecord::Preview => UpdateChannel::Unstable,
    }
}

fn record_channel(channel: UpdateChannel) -> UpdateChannelRecord {
    match channel {
        UpdateChannel::Stable => UpdateChannelRecord::Stable,
        UpdateChannel::Unstable => UpdateChannelRecord::Preview,
    }
}

fn release_record(release: &Release) -> UpdateReleaseRecord {
    UpdateReleaseRecord {
        version: release.version.to_string(),
        notes: release.notes.clone(),
        published: release.pub_date.clone(),
        is_security: release.is_security_update(),
    }
}

#[uniffi::export]
impl UpdateEngine {
    #[uniffi::constructor]
    pub fn new(setup: UpdateSetup, listener: Arc<dyn AuxListener>) -> Result<Arc<Self>, AuxError> {
        let version = semver::Version::parse(&setup.current_version).map_err(AuxError::failed)?;
        let mut config = UpdaterConfig::new(&setup.base_url, setup.public_key, version, setup.bundle_id);
        config.team_id = setup.team_id;
        config.verify_codesign = setup.verify_codesign;
        config.fallback_url = setup.fallback_url;
        config.channel = core_channel(setup.channel);
        let schedule_file = match setup.schedule_file {
            Some(file) => PathBuf::from(file),
            None => DataRoot::new(resolve_data_dir(None)?).update_schedule_file(),
        };
        let mut schedule = UpdateSchedule::load_shared(&schedule_file, || None);
        // The wait before this copy is counted starts the first time a version that counts runs.
        if schedule.start_count_grace(SystemTime::now())
            && let Err(err) = schedule.save_shared(&schedule_file)
        {
            tracing::warn!("saving update schedule: {err:#}");
        }
        Ok(Arc::new(Self {
            shared: Arc::new(Shared {
                bundle: setup.app_bundle.map(PathBuf::from),
                schedule_file,
                schedule: Mutex::new(schedule),
                state: Mutex::new(State {
                    phase: Phase::Idle,
                    channel: config.channel,
                    generation: 0,
                    installing: None,
                    note: None,
                    failed_install: None,
                }),
                config,
                received: AtomicU64::new(0),
                expected: AtomicU64::new(0),
                counting: AtomicBool::new(true),
                auto_install: AtomicBool::new(true),
                waker: Waker::new(listener),
            }),
        }))
    }

    /// What the window shows now; the next change wakes again.
    pub fn state(&self) -> UpdateStateRecord {
        let shared = &self.shared;
        shared.waker.rearm();
        let state = locked(&shared.state);
        let phase = match &state.phase {
            Phase::Idle => UpdatePhaseRecord::Idle,
            Phase::Checking => UpdatePhaseRecord::Checking,
            Phase::UpToDate => UpdatePhaseRecord::UpToDate,
            Phase::Available(release) => UpdatePhaseRecord::Available { release: release_record(release) },
            Phase::Unsupported(reason) => UpdatePhaseRecord::Unsupported { reason: reason.clone() },
            Phase::Downloading => UpdatePhaseRecord::Downloading {
                received: shared.received.load(Ordering::Relaxed),
                expected: shared.expected.load(Ordering::Relaxed),
            },
            Phase::Installed(version) => UpdatePhaseRecord::Installed { version: version.clone() },
            Phase::Failed(message) => UpdatePhaseRecord::Failed { message: message.clone() },
        };
        UpdateStateRecord {
            phase,
            channel: record_channel(state.channel),
            last_checked: locked(&shared.schedule).last_check,
            channel_note: state
                .note
                .clone()
                .map(|(version, stays_installed)| UpdateChannelNote { version, stays_installed }),
        }
    }

    /// Asks the feeds. `manual`: the person pressed Check, so every result shows (a skipped version too, a
    /// failure said); an automatic check stays quiet about what it need not say.
    pub fn check(&self, manual: bool) {
        let generation = {
            let mut state = locked(&self.shared.state);
            if matches!(state.phase, Phase::Checking | Phase::Downloading) {
                return;
            }
            // The schedule's own check leaves an offered or installed update as it is (GPUI's `schedule_check`):
            // it would withdraw the chips, drop a pending Restart When Idle and fetch the same release again.
            if !manual && matches!(state.phase, Phase::Available(_) | Phase::Installed(_)) {
                return;
            }
            state.phase = Phase::Checking;
            state.generation
        };
        self.shared.waker.notify();
        let shared = Arc::clone(&self.shared);
        spawn("check", move || {
            let mut config = shared.config.clone();
            config.channel = locked(&shared.state).channel;
            // The anonymous count: the first check of a day and of a month says so, once the grace is over.
            let count = if shared.counting.load(Ordering::Relaxed) {
                locked(&shared.schedule).count_due(SystemTime::now())
            } else {
                None
            };
            config.count = count;
            let result = update::check_counted(&config);
            finish_check(&shared, generation, manual, count, result);
        });
    }

    /// Downloads, verifies and swaps in the release `version` the last check offered; the running app goes on
    /// until `restart`.
    pub fn install(&self, version: String) -> Result<(), AuxError> {
        let shared = &self.shared;
        let release = match &locked(&shared.state).phase {
            Phase::Available(release) if release.version.to_string() == version => (**release).clone(),
            _ => return Err(AuxError::failed(format!("version {version} is not on offer"))),
        };
        start_install(&self.shared, release)
    }

    /// Whether the settings let a check install what it finds by itself (`auto_update`).
    pub fn set_auto_update(&self, on: bool) {
        self.shared.auto_install.store(on, Ordering::Relaxed);
    }

    /// Whether a check carries the anonymous count (`anonymous_count`).
    pub fn set_anonymous_count(&self, on: bool) {
        self.shared.counting.store(on, Ordering::Relaxed);
    }

    /// Stays quiet about `version` from now on (a security update is never skipped).
    pub fn skip(&self, version: String) -> Result<(), AuxError> {
        let parsed = semver::Version::parse(&version).map_err(AuxError::failed)?;
        let shared = &self.shared;
        let secure = matches!(&locked(&shared.state).phase, Phase::Available(r) if r.version == parsed && r.is_security_update());
        if secure {
            return Ok(());
        }
        let mut schedule = locked(&shared.schedule);
        schedule.skip(&parsed);
        if let Err(err) = schedule.save_shared(&shared.schedule_file) {
            tracing::warn!("saving update schedule: {err:#}");
        }
        drop(schedule);
        set_phase(shared, Phase::Idle);
        Ok(())
    }

    /// Switches the feeds. A build of the channel left that is on its way is called off, one already installed
    /// stays and is said so; otherwise the new channel is asked at once.
    pub fn set_channel(&self, channel: UpdateChannelRecord) {
        let channel = core_channel(channel);
        let shared = &self.shared;
        let check_now = {
            let mut state = locked(&shared.state);
            if state.channel == channel {
                return;
            }
            state.channel = channel;
            state.generation += 1;
            state.note = None;
            let installed = match &state.phase {
                Phase::Installed(version) => semver::Version::parse(version).ok(),
                _ => None,
            };
            let on_its_way = state.installing.as_ref().filter(|_| matches!(state.phase, Phase::Downloading));
            let held = match (&installed, on_its_way) {
                (Some(version), _) => Held::Installed(version),
                (None, Some(installing)) => Held::OnItsWay(&installing.version),
                (None, None) => Held::Nothing,
            };
            let way = update::on_channel_switch(held, channel);
            let version = installed.clone().or_else(|| on_its_way.map(|i| i.version.clone()));
            let mut check = true;
            match (way, version) {
                (OnSwitch::CallOff, Some(version)) => {
                    if let Some(installing) = state.installing.take() {
                        installing.called_off.store(true, Ordering::Release);
                    }
                    state.phase = Phase::Idle;
                    state.note = Some((version.to_string(), false));
                }
                (OnSwitch::StaysInstalled, Some(version)) => {
                    state.note = Some((version.to_string(), true));
                    check = false;
                }
                _ => {}
            }
            check && !matches!(state.phase, Phase::Checking | Phase::Downloading | Phase::Installed(_))
        };
        shared.waker.notify();
        if check_now {
            self.check(true);
        }
    }

    /// Seconds until the next automatic check is due (0: now), from the schedule's six-hour rhythm; arm one
    /// timer for it, not a poll.
    pub fn next_check_in_seconds(&self) -> u64 {
        locked(&self.shared.schedule).next_check_in(SystemTime::now()).as_secs()
    }

    /// Whether a check is due right after launch (not within an hour of the last).
    pub fn due_at_launch(&self) -> bool {
        locked(&self.shared.schedule).due_at_launch(SystemTime::now())
    }

    /// Starts core's relaunch helper (it opens the bundle again once this process has exited); the caller
    /// terminates the app right after. Fails when the app does not run from a bundle.
    pub fn restart(&self) -> Result<(), AuxError> {
        let Some(bundle) = self.shared.bundle.clone() else { return Err(not_bundled(&self.shared)) };
        update::relaunch(&bundle).map_err(|err| {
            set_phase(&self.shared, Phase::Failed(format!("Couldn't restart: {err}")));
            AuxError::failed(err)
        })
    }
}

/// Download, verify and swap in `release` (`install_update` of the GPUI app), on a thread of its own: by a person's
/// click, or by a check itself while `auto_update` is on.
fn start_install(shared: &Arc<Shared>, release: Release) -> Result<(), AuxError> {
    let Some(bundle) = shared.bundle.clone() else { return Err(not_bundled(shared)) };
    if let Err(err) = update::preflight(&bundle) {
        set_phase(shared, Phase::Failed(err.to_string()));
        return Ok(());
    }
    let called_off = Arc::new(AtomicBool::new(false));
    shared.received.store(0, Ordering::Relaxed);
    shared.expected.store(release.asset.size, Ordering::Relaxed);
    {
        let mut state = locked(&shared.state);
        state.installing =
            Some(Installing { version: release.version.clone(), called_off: Arc::clone(&called_off) });
        state.phase = Phase::Downloading;
    }
    shared.waker.notify();
    let shared = Arc::clone(shared);
    spawn("install", move || {
        let mut last = Instant::now();
        let result = update::download(&shared.config, &release, |done, _| {
            shared.received.store(done, Ordering::Relaxed);
            if last.elapsed() >= PROGRESS_EVERY {
                last = Instant::now();
                shared.waker.notify();
            }
        })
        .and_then(|downloaded| update::install_unless(&shared.config, &downloaded, &bundle, &called_off));
        // Called off by a change of channel: the person has moved on, so the result is not theirs.
        if called_off.load(Ordering::Acquire) {
            return;
        }
        let mut state = locked(&shared.state);
        state.installing = None;
        state.phase = match result {
            Ok(()) => Phase::Installed(release.version.to_string()),
            Err(err) => {
                tracing::error!("update failed: {err}");
                if !matches!(err, UpdateError::Network(_)) {
                    state.failed_install = Some(release.version.clone());
                }
                Phase::Failed(err.to_string())
            }
        };
        drop(state);
        shared.waker.notify();
    });
    Ok(())
}

/// A build that does not run from an app bundle has nothing to replace or reopen: said in the window too, so the
/// button is not silent (the GPUI app says it for an install).
fn not_bundled(shared: &Shared) -> AuxError {
    set_phase(shared, Phase::Failed("Wormhole isn't running from an app bundle.".into()));
    AuxError::failed(UpdateError::NotBundled)
}

fn spawn(name: &str, job: impl FnOnce() + Send + 'static) {
    if let Err(err) = std::thread::Builder::new().name(format!("wormhole-ffi-update-{name}")).spawn(job) {
        tracing::error!("could not start the update {name} job: {err}");
    }
}

fn set_phase(shared: &Shared, phase: Phase) {
    locked(&shared.state).phase = phase;
    shared.waker.notify();
}

/// The check's answer as a phase (`finish_check` of the GPUI app): an automatic check says nothing when the
/// version was skipped, and nothing of a failure.
fn finish_check(
    shared: &Arc<Shared>,
    generation: u64,
    manual: bool,
    count: Option<Count>,
    checked: Result<Checked, UpdateError>,
) {
    if locked(&shared.state).generation != generation {
        return;
    }
    // Not after a network error: a laptop that launches before Wi-Fi is up tries again at the next chance.
    if !matches!(checked, Err(UpdateError::Network(_))) {
        let mut schedule = locked(&shared.schedule);
        let now = SystemTime::now();
        schedule.record_check(now);
        // Only a count that reached the download server: from the fallback's copy of the feed the day is counted later.
        if let (Some(count), Ok(Checked { counted: true, .. })) = (count, &checked) {
            schedule.record_count(count, now);
        }
        if let Err(err) = schedule.save_shared(&shared.schedule_file) {
            tracing::warn!("saving update schedule: {err:#}");
        }
    }
    let result = checked.map(|checked| checked.status);
    let quiet = match &result {
        Ok(status) if !manual => !locked(&shared.schedule).should_notify(status),
        _ => false,
    };
    let phase = match result {
        Ok(_) if quiet => Phase::Idle,
        Ok(UpdateStatus::UpToDate) => Phase::UpToDate,
        Ok(UpdateStatus::Unsupported { version, minimum_system_version }) => {
            Phase::Unsupported(format!("Version {version} needs macOS {minimum_system_version} or later."))
        }
        Ok(UpdateStatus::Available(release)) => {
            // With auto_update on, a check installs what it finds (not again a version that failed to install, unless asked).
            let failed_before = locked(&shared.state).failed_install.as_ref() == Some(&release.version);
            if shared.auto_install.load(Ordering::Relaxed) && (manual || !failed_before) {
                let version = release.version.to_string();
                let on_offer = Phase::Available(Box::new(release.clone()));
                {
                    // A channel switched during the check called it off: what it found is the left channel's.
                    let mut state = locked(&shared.state);
                    if state.generation != generation {
                        return;
                    }
                    state.phase = on_offer;
                }
                if let Err(err) = start_install(shared, release) {
                    tracing::warn!("automatic install of {version}: {err}");
                }
                return;
            }
            Phase::Available(Box::new(release))
        }
        Err(err) => {
            tracing::warn!("update check failed: {err}");
            if manual { Phase::Failed(err.to_string()) } else { Phase::Idle }
        }
    };
    let mut state = locked(&shared.state);
    if state.generation == generation {
        state.phase = phase;
    }
    drop(state);
    shared.waker.notify();
}
