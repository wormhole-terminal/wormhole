//! `platform::window` where there is nothing to ask: no traffic lights drawn by the system, and
//! GPUI has no call for moving, ordering or hiding a window.

use gpui_kit::Window;

pub fn lights_span(_: &Window) -> Option<f32> {
    None
}

pub fn set_lights_shown(_: &Window, _: bool) {}

pub fn follow_theme(_: &Window) {}

#[cfg(feature = "devctl")]
pub fn lights_state(_: &Window) -> Option<String> {
    None
}

#[cfg(feature = "devctl")]
pub fn lights_opacity(_: &Window) -> Option<(f32, bool)> {
    None
}

pub fn set_top_left(_: &Window, _: f32, _: f32) {}

pub fn set_global_top_left(_: &Window, _: f32, _: f32) {}

pub fn make_ghost(_: &Window) {}

pub fn set_shadow(_: &Window, _: bool) {}

pub fn is_minimised(_: &Window) -> bool {
    false
}

pub fn allow_fullscreen(_: &Window) {}

pub fn deminiaturize(_: &Window) {}

pub fn hide(_: &Window) {}

pub fn show(_: &Window) {}

pub fn order_back(_: &Window) {}

pub fn keep_behind(_: &Window) {}

pub fn number(_: &Window) -> Option<i64> {
    None
}

pub fn set_resize_increments(_: &Window, _: Option<(f32, f32)>) {}

#[cfg(feature = "devctl")]
pub fn scaling_state(_: &Window) -> Option<String> {
    None
}

pub fn in_live_resize(_: &Window) -> bool {
    false
}

pub fn settle(_: &Window, _: (f32, f32)) -> bool {
    false
}
