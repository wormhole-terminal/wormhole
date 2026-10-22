//! `platform::look` with nothing to ask: no appearance known (the theme's own stands), no
//! display options on, nothing to watch.

use wormhole_core::tokens::Appearance;

/// Never made here: there is nothing to observe.
pub struct DisplayObserver;

pub fn appearance() -> Option<Appearance> {
    None
}

pub fn reduce_transparency() -> bool {
    false
}

pub fn increase_contrast() -> bool {
    false
}

pub fn reduce_motion() -> Option<bool> {
    None
}

pub fn observe(changed: impl Fn() + Send + Sync + 'static) -> Option<DisplayObserver> {
    let _ = changed;
    None
}
