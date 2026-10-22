//! The plan usage limits the coding agents last reported: one entity for the whole app, which every
//! window's Usage tab reads and watches. The reports are the account's, not a window's or a terminal's
//! (Claude Code's status line says them for the account), and a profile has its own: they are kept in the
//! shown profile's database and read again when the profile changes.
use gpui_kit::*;
use wormhole_core::agents::usage::{self, Usage};

use crate::app_state::app;

/// The latest report of each agent that has reported, newest last.
#[derive(Default)]
pub(crate) struct UsageReports {
    all: Vec<Usage>,
}

impl UsageReports {
    /// An agent's latest report replaces its last one, and is kept across restarts.
    fn record(&mut self, report: Usage, cx: &mut Context<Self>) {
        usage::record(&mut self.all, report);
        if let Some(store) = &app(cx).store
            && let Err(err) = store.set_meta(usage::STORE_KEY, &usage::to_json(&self.all))
        {
            tracing::warn!("remembering the usage limits: {err:#}");
        }
        cx.notify();
    }

    /// What the shown profile's database remembers, in place of what was here.
    fn load(&mut self, cx: &mut Context<Self>) {
        let json = app(cx).store.as_ref().and_then(|store| store.meta(usage::STORE_KEY).ok().flatten());
        self.all = json.map(|json| usage::from_json(&json)).unwrap_or_default();
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

/// The reports as they are, newest last; none before the app has installed the feature.
pub(crate) fn reports(cx: &App) -> &[Usage] {
    cx.try_global::<Shared>().map_or(&[], |shared| shared.0.read(cx).all.as_slice())
}

/// An agent reported (the hook socket's `Message::Usage`): kept, and every window shows it.
pub(crate) fn record(report: Usage, cx: &mut App) {
    shared(cx).update(cx, |reports, cx| reports.record(report, cx));
    cx.refresh_windows();
}

/// The shown profile's data is in `AppState`: take this profile's reports. A hidden profile's windows
/// draw the shown profile's, as they always have.
pub(crate) fn profile_shown(cx: &mut App) {
    shared(cx).update(cx, |reports, cx| reports.load(cx));
}
