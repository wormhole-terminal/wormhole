//! How the system draws scrollbars, and what a click on a track does: System Settings →
//! Appearance → Show scroll bars (the style, read at start and followed while the app runs) and
//! "Click in the scroll bar to" (read at the click, nothing to follow).
//!
//! The terminal draws its scrollbar itself, so it asks here. The legacy style takes a gutter from
//! the pane, which is why a change of style re-lays out every window and nothing else does.

use gpui_kit::{App, Global};
use wormhole_core::terminal::scrollbar::{GUTTER_W, ScrollerStyle};

use super::os::scroller as os;
pub use super::os::scroller::StyleObserver;

/// The system's scrollbar style, kept as a global. Without one (tests) the overlay style is used.
pub struct ScrollerPrefs {
    system: ScrollerStyle,
    /// What devctl says in place of the system.
    stand_in: Option<ScrollerStyle>,
    /// Hears the style change while the app runs.
    _observer: Option<StyleObserver>,
}

impl Global for ScrollerPrefs {}

impl ScrollerPrefs {
    /// Read the style now and follow it from here on.
    pub fn install(cx: &mut App) {
        let (changed, changes) = smol::channel::unbounded::<()>();
        // Tests never ask the real system: the overlay style, nothing watched.
        let observer = if cfg!(test) {
            None
        } else {
            os::observe(move || {
                let _ = changed.try_send(());
            })
        };
        cx.set_global(Self { system: system_style(cx), stand_in: None, _observer: observer });
        cx.spawn(async move |cx| {
            while changes.recv().await.is_ok() {
                cx.update(|cx| Self::set_system(cx, system_style(cx)));
            }
        })
        .detach();
    }

    fn set_system(cx: &mut App, style: ScrollerStyle) {
        let before = Self::style(cx);
        cx.global_mut::<Self>().system = style;
        if Self::style(cx) != before {
            // The gutter changes the panes' width; every terminal lays out again.
            cx.refresh_windows();
        }
    }

    pub fn style(cx: &App) -> ScrollerStyle {
        cx.try_global::<Self>()
            .map_or_else(ScrollerStyle::default, |prefs| prefs.stand_in.unwrap_or(prefs.system))
    }

    /// The width a terminal gives up for the scrollbar: the gutter in the legacy style, which is
    /// always there so that showing or hiding the thumb never reflows the text.
    pub fn gutter(cx: &App) -> f32 {
        if Self::style(cx) == ScrollerStyle::Legacy { GUTTER_W } else { 0.0 }
    }

    /// devctl's `scroller`: say the system is in one style, or follow it again.
    #[cfg(feature = "devctl")]
    pub(crate) fn stand_in(cx: &mut App, style: Option<ScrollerStyle>) {
        cx.global_mut::<Self>().stand_in = style;
        cx.refresh_windows();
    }
}

/// Whether a click on a track jumps to the spot ("Jump to the spot that's clicked") rather than
/// pages ("Jump to the next page"); `false` where the system can't be asked.
pub fn jump_to_spot() -> bool {
    !cfg!(test) && os::jump_to_spot()
}

fn system_style(cx: &App) -> ScrollerStyle {
    if cfg!(test) { ScrollerStyle::default() } else { os::style(cx) }
}
