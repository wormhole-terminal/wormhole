//! Zen mode: the window drawn with [`Preset::Zen`] for a while, without saving it.
//!
//! It is a layer of the settings in force, applied last in [`SettingsStore::derive`], after the
//! project's overrides and the system's say, the way those are: `config.json` and [`Config`] never
//! hear of it. A mutator saves from `Config`'s own settings (`Config::update` clones those, never the
//! derived ones), so a setting changed while Zen mode is on writes the person's real values, and ending
//! it restores exactly what was there because nothing was changed. It is the app's, not a profile's
//! (every open profile's store takes it, [`ParkedSettings`]) and not remembered across launches.
//!
//! The right panel is not a setting but a window's own state, which Zen mode must not close: the window
//! draws without it while this is on (`window::layout`) and keeps what it was asked for.
//!
//! The settings window shows the person's own values meanwhile ([`SettingsStore::settings_without_zen`]),
//! with a note that the window shows less.

use gpui_kit::App;
use wormhole_core::config::Settings;
use wormhole_core::config::presets::{Preset, Shown};
use wormhole_text::keys::{ZEN_OFF, ZEN_ON};
use wormhole_text::text;

use super::{ParkedSettings, SettingsStore};

/// The settings Zen mode shows: `settings` with [`Preset::Zen`]'s title row, and without the status
/// bar, the minimap and the lines between the columns. Everything else the preset holds (the panel, the
/// tools) is not a setting Zen mode touches: the panel is the window's, and a tool switched off would be
/// a setting changed for good.
pub(super) fn layer(settings: &mut Settings) {
    let zen = Shown::of(Preset::Zen);
    settings.title_row = Some(zen.title.row());
    settings.status_bar = zen.status_bar;
    settings.minimap = zen.minimap;
    settings.column_dividers = zen.column_dividers;
}

/// The settings in force, and (while Zen mode is on) the ones they were before it: what the person's own
/// settings and the project's make of them.
pub(super) fn layered(base: Settings, on: bool) -> (Settings, Option<Settings>) {
    if !on {
        return (base, None);
    }
    let mut shown = base.clone();
    layer(&mut shown);
    (shown, Some(base))
}

impl SettingsStore {
    /// Whether Zen mode is on.
    pub fn zen(&self) -> bool {
        self.zen
    }

    /// The settings as the person's own and the project's make them, without Zen mode's layer: what the
    /// settings window shows and edits. The same as `settings` while Zen mode is off.
    pub fn settings_without_zen(&self) -> &Settings {
        self.unzen.as_ref().unwrap_or(&self.settings)
    }

    /// This store, fresh from disk, in the app's state of Zen mode (`app`'s, the shown profile's store):
    /// a profile opened beside the shown one while Zen mode is on is drawn with it from its first
    /// frame, not from the next tick's `follow`.
    pub(crate) fn following(mut self, app: &SettingsStore) -> Self {
        self.take_zen(app.zen);
        self
    }

    /// Turn Zen mode on or off for the app: this store, and the stores of the other open profiles.
    /// Returns whether it changed.
    pub fn set_zen(cx: &mut App, on: bool) -> bool {
        if cx.global::<SettingsStore>().zen == on {
            return false;
        }
        Self::mutate(cx, |store| store.zen = on);
        if cx.has_global::<ParkedSettings>() {
            cx.global_mut::<ParkedSettings>().set_zen(on);
        }
        true
    }
}

/// [`SettingsStore::zen`] of the shown profile's store, which is the app's.
pub fn zen(cx: &App) -> bool {
    cx.global::<SettingsStore>().zen
}

/// [`super::settings`] without Zen mode's layer.
pub fn settings_without_zen(cx: &App) -> &Settings {
    cx.global::<SettingsStore>().settings_without_zen()
}

/// Turn Zen mode on or off from the person's own action (the menu, a key, the settings window's button),
/// saying so to VoiceOver as the windows change shape. Every window draws again; the menus follow on the
/// tick, from the same flag.
pub fn set_zen_mode(cx: &mut App, on: bool) {
    if SettingsStore::set_zen(cx, on) {
        crate::platform::a11y::announce(text(if on { ZEN_ON } else { ZEN_OFF }), false);
        cx.refresh_windows();
    }
}

/// The toggle: View → Zen Mode.
pub fn toggle_zen_mode(cx: &mut App) {
    let on = zen(cx);
    set_zen_mode(cx, !on);
}
