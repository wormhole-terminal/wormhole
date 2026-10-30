//! Saving and restoring where a window was.
use gpui_kit::*;
use serde::{Deserialize, Serialize};

/// Smaller than the windows' minimum size means the record is garbage.
use wormhole_core::view::layout::{WINDOW_MIN_HEIGHT as MIN_HEIGHT, WINDOW_MIN_WIDTH as MIN_WIDTH};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Windowed,
    Maximized,
    Fullscreen,
}

/// For maximized and full-screen windows the bounds are the size to restore to. The bounds are in the space
/// of the display the window was on (GPUI's windows are placed that way), so the display goes with them;
/// a record from before has none, and was always restored on the primary display.
#[derive(Serialize, Deserialize, Debug)]
struct Saved {
    mode: Mode,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    display: Option<u64>,
    /// The window was in the Dock. The bounds are then the ones it will come back to; a record from
    /// before has none and was never restored minimised.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    minimised: bool,
}

/// Where a window opened from another one stands: its bounds and the display they are on (`None`: the
/// primary one), which bounds mean nothing without.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Source {
    pub(crate) bounds: Bounds<Pixels>,
    pub(crate) display: Option<DisplayId>,
}

impl Source {
    /// Where `window` is.
    pub(crate) fn of(window: &Window, cx: &App) -> Self {
        Self { bounds: window.bounds(), display: window.display(cx).map(|display| display.id()) }
    }
}

pub(super) fn encode(bounds: WindowBounds, display: Option<DisplayId>, minimised: bool) -> Option<String> {
    let (mode, b) = match bounds {
        WindowBounds::Windowed(b) => (Mode::Windowed, b),
        WindowBounds::Maximized(b) => (Mode::Maximized, b),
        WindowBounds::Fullscreen(b) => (Mode::Fullscreen, b),
    };
    let saved = Saved {
        mode,
        x: f32::from(b.origin.x),
        y: f32::from(b.origin.y),
        width: f32::from(b.size.width),
        height: f32::from(b.size.height),
        display: display.map(u64::from),
        minimised,
    };
    serde_json::to_string(&saved).ok()
}

/// Whether a saved window was minimised when it was last seen. Garbage and old records say no.
pub(super) fn minimised(json: &str) -> bool {
    serde_json::from_str::<Saved>(json).is_ok_and(|saved| saved.minimised)
}

/// The window to bring forward once the saved ones are open: the last of them, in the order they
/// opened, that is not minimised. `windows` is that order (`front_last` has put the one that was in
/// front last), as `(id, was minimised)`. A minimised window is never the one to come forward, since
/// that would take it out of the Dock; when the one that was in front was minimised, the one opened
/// before it takes its place. `None` when every window stays in the Dock.
pub(super) fn front_after_restore(windows: &[(i64, bool)]) -> Option<i64> {
    windows.iter().rev().find(|(_, minimised)| !minimised).map(|(id, _)| *id)
}

/// A saved window's bounds and the display they were on.
pub(super) fn decode(json: &str) -> Option<(WindowBounds, Option<DisplayId>)> {
    let saved: Saved = serde_json::from_str(json).ok()?;
    let finite = [saved.x, saved.y, saved.width, saved.height].iter().all(|v| v.is_finite());
    if !finite || saved.width < MIN_WIDTH || saved.height < MIN_HEIGHT {
        return None;
    }
    let bounds = Bounds::new(point(px(saved.x), px(saved.y)), size(px(saved.width), px(saved.height)));
    let bounds = match saved.mode {
        Mode::Windowed => WindowBounds::Windowed(bounds),
        Mode::Maximized => WindowBounds::Maximized(bounds),
        Mode::Fullscreen => WindowBounds::Fullscreen(bounds),
    };
    Some((bounds, saved.display.map(DisplayId::from)))
}

/// Notices that a window moved, resized or went into the Dock or came out. It is asked every tick, and the bounds are encoded
/// (to JSON) only when they differ from the last ones seen, not to compare them every time.
#[derive(Default)]
pub(super) struct BoundsWatch {
    seen: Option<(WindowBounds, bool)>,
}

impl BoundsWatch {
    /// The record to save when the window is somewhere new: `now` is not what was last seen and
    /// its encoding is not already `saved`. `display` (which display it is on) is asked only when the
    /// bounds are new: it is a lookup, and this runs every tick of every window. (A window moved to
    /// another display at the very same place on it is seen when it next moves.)
    pub(super) fn moved(
        &mut self,
        now: WindowBounds,
        display: impl FnOnce() -> Option<DisplayId>,
        minimised: bool,
        saved: Option<&str>,
    ) -> Option<String> {
        if self.seen == Some((now, minimised)) {
            return None;
        }
        let display = display();
        self.seen = Some((now, minimised));
        let encoded = encode(now, display, minimised)?;
        (saved != Some(encoded.as_str())).then_some(encoded)
    }
}

/// Where a window opened from `source` goes: a little down and to the right, or back to the corner of
/// the display when that would run off it. `display` is the space of `source`'s display, its bounds.
pub(super) fn cascade(source: Bounds<Pixels>, display: Bounds<Pixels>) -> Bounds<Pixels> {
    const STEP: f32 = 26.0;
    const MARGIN: f32 = 40.0;
    let shifted = Bounds::new(source.origin + point(px(STEP), px(STEP)), source.size);
    if display.contains(&shifted.origin) && display.contains(&shifted.bottom_right()) {
        return shifted;
    }
    // Wrap to the top-left of the display, which is never bigger than it.
    Bounds::new(
        display.origin + point(px(MARGIN), px(MARGIN)),
        size(
            source.size.width.min(display.size.width - px(MARGIN * 2.0)),
            source.size.height.min(display.size.height - px(MARGIN * 2.0)),
        ),
    )
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Bounds, DisplayId, WindowBounds, point, px, size};

    use super::{BoundsWatch, cascade, decode, encode, front_after_restore, minimised};

    fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<gpui_kit::Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }

    #[test]
    fn bounds_round_trip_for_every_mode() {
        let b = bounds(120.0, 80.0, 1320.0, 820.0);
        for original in [WindowBounds::Windowed(b), WindowBounds::Maximized(b), WindowBounds::Fullscreen(b)] {
            let json = encode(original, None, false).expect("encodes");
            assert_eq!(decode(&json), Some((original, None)));
            // The display is part of the place: bounds are in its space.
            let json = encode(original, Some(DisplayId::new(7)), false).expect("encodes");
            assert_eq!(decode(&json), Some((original, Some(DisplayId::new(7)))));
        }
    }

    #[test]
    fn a_record_from_before_displays_were_saved_has_none() {
        let old = r#"{"mode":"windowed","x":10.0,"y":20.0,"width":900.0,"height":700.0}"#;
        assert_eq!(decode(old), Some((WindowBounds::Windowed(bounds(10.0, 20.0, 900.0, 700.0)), None)));
        let json =
            encode(WindowBounds::Windowed(bounds(10.0, 20.0, 900.0, 700.0)), None, false).expect("encodes");
        assert!(!json.contains("display"), "nothing written for none: {json}");
    }

    #[test]
    fn minimised_is_saved_only_when_it_is_so_and_seen_as_a_change() {
        let here = WindowBounds::Windowed(bounds(10.0, 20.0, 1000.0, 700.0));
        let docked = encode(here, None, true).expect("encodes");
        assert!(minimised(&docked));
        assert_eq!(decode(&docked), Some((here, None)), "the place is kept for coming back to");
        let shown = encode(here, None, false).expect("encodes");
        assert!(!minimised(&shown) && !shown.contains("minimised"), "{shown}");
        assert!(!minimised("not json"));

        // Going into the Dock at the same place is a change worth saving; coming out is too.
        let mut watch = BoundsWatch::default();
        assert_eq!(watch.moved(here, || None, false, Some(shown.as_str())), None);
        assert_eq!(watch.moved(here, || None, true, Some(shown.as_str())), Some(docked.clone()));
        assert_eq!(watch.moved(here, || None, false, Some(docked.as_str())), Some(shown));
    }

    #[test]
    fn a_minimised_window_is_never_the_one_that_comes_forward() {
        // In front last (`front_last`): normally it is the last that opened.
        assert_eq!(front_after_restore(&[(1, false), (2, false), (3, false)]), Some(3));
        // The one that was in front was minimised: the one opened before it comes forward.
        assert_eq!(front_after_restore(&[(1, false), (2, false), (3, true)]), Some(2));
        assert_eq!(front_after_restore(&[(1, false), (2, true), (3, true)]), Some(1));
        // All in the Dock: nothing comes forward, and nothing leaves the Dock.
        assert_eq!(front_after_restore(&[(1, true), (2, true)]), None);
        assert_eq!(front_after_restore(&[]), None);
    }

    #[test]
    fn nonsense_is_rejected() {
        assert_eq!(decode("not json"), None);
        assert_eq!(decode(r#"{"mode":"windowed","x":0,"y":0,"width":10,"height":10}"#), None);
        assert_eq!(decode(r#"{"mode":"floating","x":0,"y":0,"width":900,"height":700}"#), None);
        assert_eq!(decode(r#"{"mode":"windowed","x":0,"y":0,"width":900}"#), None);
    }

    #[test]
    fn new_windows_step_down_and_right_and_wrap_at_the_screen_edge() {
        let screen = bounds(0.0, 0.0, 1920.0, 1080.0);
        let source = bounds(100.0, 100.0, 1000.0, 700.0);
        assert_eq!(cascade(source, screen), bounds(126.0, 126.0, 1000.0, 700.0));

        // No room to step: start again near the corner instead of stacking exactly.
        let low = bounds(900.0, 400.0, 1000.0, 700.0);
        assert_eq!(cascade(low, screen).origin, point(px(40.0), px(40.0)));

        // Never bigger than the screen it wraps onto.
        let huge = bounds(0.0, 0.0, 3000.0, 2000.0);
        assert_eq!(cascade(huge, screen).size, size(px(1840.0), px(1000.0)));
    }

    #[test]
    fn a_window_steps_within_the_display_it_is_on() {
        // A small display beside a big one: the window on it wraps at its edge, not at the big one's.
        let small = bounds(0.0, 0.0, 1280.0, 720.0);
        let source = bounds(300.0, 100.0, 900.0, 600.0);
        assert_eq!(cascade(source, small).origin, point(px(40.0), px(40.0)));
        assert_eq!(cascade(source, bounds(0.0, 0.0, 2560.0, 1440.0)).origin, point(px(326.0), px(126.0)));
    }

    #[test]
    fn a_window_that_has_not_moved_is_not_encoded_again() {
        let mut watch = BoundsWatch::default();
        let here = WindowBounds::Windowed(bounds(10.0, 20.0, 1000.0, 700.0));
        let there = WindowBounds::Windowed(bounds(30.0, 20.0, 1000.0, 700.0));

        // The first look compares with what was saved: the same place is nothing to save...
        let saved = encode(here, None, false);
        assert_eq!(watch.moved(here, || None, false, saved.as_deref()), None);
        // ...a different one is saved, once.
        assert_eq!(watch.moved(there, || None, false, saved.as_deref()), encode(there, None, false));
        // Still there: the saved record is stale (the caller has not stored it yet) but the
        // window has not moved since it was last seen.
        assert_eq!(watch.moved(there, || None, false, saved.as_deref()), None);
        // Back to the saved place: nothing new to save.
        assert_eq!(watch.moved(here, || None, false, saved.as_deref()), None);
        // Maximizing is a change even at the same size.
        let maximized = WindowBounds::Maximized(bounds(10.0, 20.0, 1000.0, 700.0));
        assert_eq!(watch.moved(maximized, || None, false, saved.as_deref()), encode(maximized, None, false));
        // The display is asked about when the bounds are new, and only then.
        let other = Some(DisplayId::new(2));
        let mut asked = 0;
        let new = WindowBounds::Maximized(bounds(0.0, 0.0, 1000.0, 700.0));
        assert_eq!(
            watch.moved(
                new,
                || {
                    asked += 1;
                    other
                },
                false,
                saved.as_deref()
            ),
            encode(new, other, false)
        );
        assert_eq!(
            watch.moved(new, || unreachable!("not asked for bounds seen"), false, saved.as_deref()),
            None
        );
        assert_eq!(asked, 1);
    }
}
