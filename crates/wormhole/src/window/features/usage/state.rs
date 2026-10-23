//! The plan usage limits the coding agents last reported: one entity for the whole app, which every
//! window's Usage tab reads and watches. The reports are the account's, not a window's or a terminal's
//! (Claude Code's status line says them for the account), and a profile has its own: each profile's
//! reports are kept here for as long as the app runs and in that profile's database, which is read the
//! first time the profile is shown.
use std::collections::HashMap;

use gpui_kit::*;
use wormhole_core::agents::usage::{self, Usage};
use wormhole_core::storage::paths::ProfileId;

use crate::app_state::app;
use crate::window::feature::FeatureEvent;

/// The latest report of each agent that has reported, newest last, for each profile the app has been
/// in. The shown profile's are the ones the windows draw and the ones a report adds to.
#[derive(Default)]
pub(crate) struct UsageReports {
    by_profile: HashMap<ProfileId, Vec<Usage>>,
}

// Every window holds this entity as a feature's state and repaints when it changes; it has nothing to
// tell a window by event.
impl EventEmitter<FeatureEvent> for UsageReports {}

impl UsageReports {
    /// An agent's latest report replaces its last one in the shown profile's reports, and is kept
    /// across restarts in that profile's database.
    fn record(&mut self, report: Usage, cx: &mut Context<Self>) {
        let all = self.by_profile.entry(app(cx).active_profile().clone()).or_default();
        usage::record(all, report);
        if let Some(store) = &app(cx).store
            && let Err(err) = store.set_meta(usage::STORE_KEY, &usage::to_json(all))
        {
            tracing::warn!("remembering the usage limits: {err:#}");
        }
        cx.notify();
    }

    /// The shown profile's reports. The database is read only the first time the profile is seen;
    /// after that what is in memory is the truth, as a profile's other data is when it is parked
    /// (including a report whose write failed, and a profile with no database).
    fn show(&mut self, cx: &mut Context<Self>) {
        let profile = app(cx).active_profile().clone();
        self.by_profile.entry(profile).or_insert_with(|| {
            let json = app(cx).store.as_ref().and_then(|store| store.meta(usage::STORE_KEY).ok().flatten());
            json.map(|json| usage::from_json(&json)).unwrap_or_default()
        });
        cx.notify();
    }
}

/// The app's one [`UsageReports`], as a global so that the windows, which are made by code that
/// names no feature, and the hook socket's handler all reach the same entity.
struct Shared(Entity<UsageReports>);

impl Global for Shared {}

/// Makes the entity. Before any window is, so each finds it.
pub(crate) fn install(cx: &mut App) {
    let reports = cx.new(|_| UsageReports::default());
    cx.set_global(Shared(reports));
}

/// The entity every window watches.
pub(super) fn shared(cx: &App) -> Entity<UsageReports> {
    cx.global::<Shared>().0.clone()
}

/// The shown profile's reports as they are, newest last; none before the app has installed the
/// feature.
pub(crate) fn reports(cx: &App) -> &[Usage] {
    cx.try_global::<Shared>().map_or(&[], |shared| {
        shared.0.read(cx).by_profile.get(app(cx).active_profile()).map_or(&[], Vec::as_slice)
    })
}

/// An agent reported (the hook socket's `Message::Usage`): kept, and every window repaints.
pub(crate) fn record(report: Usage, cx: &mut App) {
    shared(cx).update(cx, |reports, cx| reports.record(report, cx));
}

/// The shown profile is in `AppState`: take this profile's reports. A hidden profile's windows draw
/// the shown profile's, as they always have.
pub(crate) fn profile_shown(cx: &mut App) {
    shared(cx).update(cx, |reports, cx| reports.show(cx));
}

/// A profile was deleted: its reports go, so a profile made later under the same id starts with none
/// and never writes these into its own database.
pub(crate) fn profile_deleted(profile: &ProfileId, cx: &mut App) {
    shared(cx).update(cx, |reports, _| {
        reports.by_profile.remove(profile);
    });
}
