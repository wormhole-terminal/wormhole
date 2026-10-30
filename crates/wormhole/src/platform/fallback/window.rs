//! `platform::window` where there is nothing to ask: no traffic lights drawn by the system, and
//! GPUI has no call for moving, ordering or hiding a window.

use gpui_kit::Window;
use wormhole_core::view::motion::live_resize::Dragged;
use wormhole_core::view::motion::window_animation::Behaviour;

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

pub fn miniaturize(_: &Window) {}

pub fn deminiaturize(_: &Window) {}

pub fn hide(_: &Window) {}

pub fn show(_: &Window) {}

pub fn order_back(_: &Window) {}

pub fn keep_behind(_: &Window) {}

pub fn number(_: &Window) -> Option<i64> {
    None
}

pub const HAS_SYSTEM_ANIMATION: bool = false;

pub fn set_animation(_: &Window, _: Behaviour) {}

#[cfg(feature = "devctl")]
pub fn scaling_state(_: &Window) -> Option<String> {
    None
}

/// Nothing is opened out of sight or dragged to a size here, so there is nothing to do with a handle.
pub struct Native;

impl Native {
    pub fn of(_: &Window) -> Option<Self> {
        None
    }

    pub fn paint(&self) {}

    pub fn show(&self, _: bool) {}

    pub fn settle(&self, _: (f32, f32), _: Dragged) -> bool {
        false
    }
}
