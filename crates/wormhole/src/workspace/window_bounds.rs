//! Saving and restoring where a window was.
use gpui_kit::*;
use serde::{Deserialize, Serialize};

/// Smaller than the windows' minimum size means the record is garbage.
use wormhole_core::layout::{WINDOW_MIN_HEIGHT as MIN_HEIGHT, WINDOW_MIN_WIDTH as MIN_WIDTH};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Windowed,
    Maximized,
    Fullscreen,
}

/// For maximized and full-screen windows the bounds are the size to restore to.
#[derive(Serialize, Deserialize, Debug)]
struct Saved {
    mode: Mode,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

pub(super) fn encode(bounds: WindowBounds) -> Option<String> {
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
    };
    serde_json::to_string(&saved).ok()
}

pub(super) fn decode(json: &str) -> Option<WindowBounds> {
    let saved: Saved = serde_json::from_str(json).ok()?;
    let finite = [saved.x, saved.y, saved.width, saved.height].iter().all(|v| v.is_finite());
    if !finite || saved.width < MIN_WIDTH || saved.height < MIN_HEIGHT {
        return None;
    }
    let bounds = Bounds::new(point(px(saved.x), px(saved.y)), size(px(saved.width), px(saved.height)));
    Some(match saved.mode {
        Mode::Windowed => WindowBounds::Windowed(bounds),
        Mode::Maximized => WindowBounds::Maximized(bounds),
        Mode::Fullscreen => WindowBounds::Fullscreen(bounds),
    })
}

/// Notices that a window moved or resized. It is asked every tick, and the bounds are encoded
/// (to JSON) only when they differ from the last ones seen, not to compare them every time.
#[derive(Default)]
pub(super) struct BoundsWatch {
    seen: Option<WindowBounds>,
}

impl BoundsWatch {
    /// The record to save when the window is somewhere new: `now` is not what was last seen and
    /// its encoding is not already `saved`.
    pub(super) fn moved(&mut self, now: WindowBounds, saved: Option<&str>) -> Option<String> {
        if self.seen == Some(now) {
            return None;
        }
        self.seen = Some(now);
        let encoded = encode(now)?;
        (saved != Some(encoded.as_str())).then_some(encoded)
    }
}

/// Where a window opened from `source` goes: a little down and to the right,
/// or back to the corner of the screen when that would run off it.
pub(super) fn cascade(source: Bounds<Pixels>, displays: &[Bounds<Pixels>]) -> Bounds<Pixels> {
    const STEP: f32 = 26.0;
    const MARGIN: f32 = 40.0;
    let shifted = Bounds::new(source.origin + point(px(STEP), px(STEP)), source.size);
    let fits = |display: &Bounds<Pixels>| {
        display.contains(&shifted.origin) && display.contains(&shifted.bottom_right())
    };
    if displays.iter().any(fits) {
        return shifted;
    }
    // Wrap to the top-left of the screen the source is on (or the first one).
    let centre = source.center();
    let display = displays.iter().find(|d| d.contains(&centre)).or(displays.first());
    match display {
        Some(display) => Bounds::new(
            display.origin + point(px(MARGIN), px(MARGIN)),
            size(
                source.size.width.min(display.size.width - px(MARGIN * 2.0)),
                source.size.height.min(display.size.height - px(MARGIN * 2.0)),
            ),
        ),
        None => shifted,
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Bounds, WindowBounds, point, px, size};

    use super::{BoundsWatch, cascade, decode, encode};

    fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<gpui_kit::Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }

    #[test]
    fn bounds_round_trip_for_every_mode() {
        let b = bounds(120.0, 80.0, 1320.0, 820.0);
        for original in [WindowBounds::Windowed(b), WindowBounds::Maximized(b), WindowBounds::Fullscreen(b)] {
            let json = encode(original).expect("encodes");
            assert_eq!(decode(&json), Some(original));
        }
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
        assert_eq!(cascade(source, &[screen]), bounds(126.0, 126.0, 1000.0, 700.0));

        // No room to step: start again near the corner instead of stacking exactly.
        let low = bounds(900.0, 400.0, 1000.0, 700.0);
        assert_eq!(cascade(low, &[screen]).origin, point(px(40.0), px(40.0)));

        // Never bigger than the screen it wraps onto.
        let huge = bounds(0.0, 0.0, 3000.0, 2000.0);
        assert_eq!(cascade(huge, &[screen]).size, size(px(1840.0), px(1000.0)));
    }

    #[test]
    fn a_window_that_has_not_moved_is_not_encoded_again() {
        let mut watch = BoundsWatch::default();
        let here = WindowBounds::Windowed(bounds(10.0, 20.0, 1000.0, 700.0));
        let there = WindowBounds::Windowed(bounds(30.0, 20.0, 1000.0, 700.0));

        // The first look compares with what was saved: the same place is nothing to save...
        let saved = encode(here);
        assert_eq!(watch.moved(here, saved.as_deref()), None);
        // ...a different one is saved, once.
        assert_eq!(watch.moved(there, saved.as_deref()), encode(there));
        // Still there: the saved record is stale (the caller has not stored it yet) but the
        // window has not moved since it was last seen.
        assert_eq!(watch.moved(there, saved.as_deref()), None);
        // Back to the saved place: nothing new to save.
        assert_eq!(watch.moved(here, saved.as_deref()), None);
        // Maximizing is a change even at the same size.
        let maximized = WindowBounds::Maximized(bounds(10.0, 20.0, 1000.0, 700.0));
        assert_eq!(watch.moved(maximized, saved.as_deref()), encode(maximized));
    }
}
