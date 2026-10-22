//! `platform::scroller` from GPUI: whether scrollbars hide on their own is GPUI's
//! `should_auto_hide_scrollbars`, the overlay style; a click on a track pages; nothing is watched
//! (a change shows from the next start).

use gpui_kit::App;
use wormhole_term::scrollbar::ScrollerStyle;

/// Never made here: there is nothing to observe.
pub struct StyleObserver;

pub fn style(cx: &App) -> ScrollerStyle {
    if cx.should_auto_hide_scrollbars() { ScrollerStyle::Overlay } else { ScrollerStyle::Legacy }
}

pub fn jump_to_spot() -> bool {
    false
}

pub fn observe(changed: impl Fn() + Send + Sync + 'static) -> Option<StyleObserver> {
    let _ = changed;
    None
}
