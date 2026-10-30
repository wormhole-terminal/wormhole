//! The look the app shares across windows: what the system says about appearance, Reduce
//! Transparency, Increase Contrast and Reduce Motion (or what devctl says in its place), and the
//! resolved theme pushed to GPUI Kit and to every window's material.
//!
//! Part of the shared state: it touches only `AppState`'s fields,
//! the settings store, `theme_bridge` and `platform`, so the windows and the profile swap call it
//! without reaching up to the app tick.

use gpui_kit::*;
use wormhole_core::theme::tokens::Appearance;

use crate::app_state::{app, app_mut};
use crate::platform::vibrancy;
use crate::settings::{SettingsStore, SystemLook};

/// The theme of the window that opened a menu. Auxiliary windows use the
/// shown profile; a workspace can belong to a different, parked profile.
pub(crate) fn menu_appearance(window: &Window, cx: &App) -> Option<Appearance> {
    let settings = cx.try_global::<SettingsStore>()?;
    let profile = cx
        .try_global::<crate::app_state::AppState>()
        .and_then(|state| {
            state.windows.iter().find(|open| open.handle.window_id() == window.window_handle().window_id())
        })
        .map(|open| &open.profile);
    Some(profile.map(|profile| crate::settings::store_in(profile, cx)).unwrap_or(settings).tokens.appearance)
}

/// Stand-ins for what the system says about how windows look; `None` follows the system.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct LookOverride {
    pub appearance: Option<Appearance>,
    pub reduce_transparency: Option<bool>,
    pub increase_contrast: Option<bool>,
    pub reduce_motion: Option<bool>,
}

/// Push the resolved theme to GPUI Kit (its theme is global, the settings and
/// Studio windows use it too), and the theme's appearance to every window's material.
pub(crate) fn apply_theme(cx: &mut App) {
    let store = cx.global::<SettingsStore>();
    let revision = store.revision;
    if app(cx).applied_theme == Some(revision) {
        return;
    }
    let (tokens, fonts) = (store.tokens.clone(), store.fonts.clone());
    crate::theme_bridge::apply(&tokens, &fonts, cx);
    vibrancy::set_material_appearance(tokens.appearance);
    app_mut(cx).applied_theme = Some(revision);
    cx.refresh_windows();
}

/// Hand the settings store what the system says now about how windows look (or what devctl
/// says in its place): the appearance Match system follows, Reduce Transparency and Increase
/// Contrast; and set GPUI's Reduce Motion flag, which its animations hold still under.
pub(crate) fn sync_system_look(cx: &mut App) {
    let known = cx.global::<SettingsStore>().system();
    let stand_in = app(cx).look_override;
    let look = SystemLook {
        appearance: stand_in
            .appearance
            .or_else(crate::platform::look::system_appearance)
            .unwrap_or(known.appearance),
        reduce_transparency: stand_in
            .reduce_transparency
            .unwrap_or_else(crate::platform::look::reduce_transparency),
        increase_contrast: stand_in
            .increase_contrast
            .unwrap_or_else(crate::platform::look::increase_contrast),
    };
    // GPUI Kit reads Reduce Motion once, at start, and stops following the system once the app
    // has set the flag itself (devctl does); reading it here keeps both cases in step.
    if let Some(still) = stand_in.reduce_motion.or_else(crate::platform::look::reduce_motion) {
        cx.set_reduce_motion(still);
    }
    if SettingsStore::set_system(cx, look) {
        apply_theme(cx);
    }
}

/// devctl's `appearance`, `reduce-transparency`, `increase-contrast` and `reduce-motion`: say
/// this in place of the system (or follow it again), and apply it now.
#[cfg(feature = "devctl")]
pub(crate) fn override_look(cx: &mut App, change: impl FnOnce(&mut LookOverride)) -> SystemLook {
    change(&mut app_mut(cx).look_override);
    sync_system_look(cx);
    cx.global::<SettingsStore>().system()
}
