//! The Dock icon: its badge (how many terminals need you), one informational bounce when a
//! terminal starts needing you while Wormhole is in the background, and whether Wormhole is
//! the active app at all. The Dock's menu goes through GPUI (`workspace::menu_bar`).
//!
//! Tests never touch the real Dock: they read what would have been applied from [`DockState`],
//! and neither does a test instance in background mode (devctl's `dock` reads it there).

use gpui_kit::{App, Global};

/// What was last applied, so an unchanged badge costs nothing (and tests can read it).
#[derive(Debug, Default)]
pub struct DockState {
    pub badge: Option<String>,
    pub bounces: usize,
}

impl Global for DockState {}

/// The red badge on the Dock icon; `None` clears it. The system draws it (it isn't themed).
pub fn set_badge(label: Option<String>, cx: &mut App) {
    let state = cx.default_global::<DockState>();
    if state.badge == label {
        return;
    }
    state.badge.clone_from(&label);
    if !cfg!(test) && !crate::background::on() {
        platform::set_badge(label.as_deref());
    }
}

/// One informational bounce (never the critical one that keeps going).
pub fn bounce(cx: &mut App) {
    cx.default_global::<DockState>().bounces += 1;
    if !cfg!(test) && !crate::background::on() {
        platform::bounce();
    }
}

/// Whether Wormhole is the active app. (GPUI's active window can't tell: a window stays the
/// main window while another app is in front.)
pub fn app_is_active(cx: &App) -> bool {
    if cfg!(test) {
        return cx.active_window().is_some();
    }
    platform::is_active()
}

#[cfg(target_os = "macos")]
mod platform {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSRequestUserAttentionType};
    use objc2_foundation::NSString;

    pub fn set_badge(label: Option<&str>) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let label = label.map(NSString::from_str);
        NSApplication::sharedApplication(mtm).dockTile().setBadgeLabel(label.as_deref());
    }

    pub fn bounce() {
        let Some(mtm) = MainThreadMarker::new() else { return };
        NSApplication::sharedApplication(mtm)
            .requestUserAttention(NSRequestUserAttentionType::InformationalRequest);
    }

    pub fn is_active() -> bool {
        MainThreadMarker::new().is_some_and(|mtm| NSApplication::sharedApplication(mtm).isActive())
    }
}

/// Off macOS there is no Dock; the app counts as active so nothing is told twice.
#[cfg(not(target_os = "macos"))]
mod platform {
    pub fn set_badge(_label: Option<&str>) {}

    pub fn bounce() {}

    pub fn is_active() -> bool {
        true
    }
}
