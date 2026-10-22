//! `platform::dock` without a Dock: no badge or icon to change; the bounce is GPUI's request for
//! attention on the active window (or the first one); the app counts as active so nothing is told
//! twice.

use gpui_kit::App;

pub fn set_badge(_label: Option<&str>) {}

pub fn bounce(cx: &mut App) {
    let Some(handle) = cx.active_window().or_else(|| cx.windows().first().copied()) else { return };
    let _ = handle.update(cx, |_, window, _| window.request_attention());
}

pub fn is_active() -> bool {
    true
}

pub fn hide_icon() {}
