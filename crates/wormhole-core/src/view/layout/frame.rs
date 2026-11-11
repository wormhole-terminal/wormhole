//! Where a window is: saved with the window, brought back at launch, stepped from the one a new window is
//! opened from. Plain numbers in the shell's own space (y grows downwards, the primary display first), so
//! the GPUI app's `window_bounds` and the Swift app's `NSWindow` frames share the rules.
//!
//! The JSON is the one the GPUI app saves (`window/window_bounds.rs`'s `Saved`).

use serde::{Deserialize, Serialize};

use super::{WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};

/// How far a window opened from another steps down and to the right.
const CASCADE_STEP: f32 = 26.0;
/// How far from the display's edges a window that wrapped round is put, and what must stay reachable of one
/// that was saved half off a display (a corner to grab).
const MARGIN: f32 = 40.0;
const REACHABLE: (f32, f32) = (120.0, 60.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameMode {
    Windowed,
    Maximized,
    Fullscreen,
}

/// A window's place. For maximised and full-screen windows the rectangle is the size to restore to.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowFrame {
    pub mode: FrameMode,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// The display it was on; the rectangle is in the shell's global space, which this only disambiguates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<u64>,
    /// It was in the Dock. The rectangle is then where it comes back to.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub minimised: bool,
}

/// A display's usable area (menu bar and Dock taken off).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayArea {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl DisplayArea {
    fn overlap(&self, frame: &WindowFrame) -> (f32, f32) {
        let w = (self.x + self.width).min(frame.x + frame.width) - self.x.max(frame.x);
        let h = (self.y + self.height).min(frame.y + frame.height) - self.y.max(frame.y);
        (w.max(0.0), h.max(0.0))
    }

    fn holds(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

impl WindowFrame {
    pub fn windowed(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { mode: FrameMode::Windowed, x, y, width, height, display: None, minimised: false }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// A saved record; one that is not finite or smaller than a window can be is garbage (`None`).
    pub fn from_json(json: &str) -> Option<Self> {
        let frame: Self = serde_json::from_str(json).ok()?;
        let finite = [frame.x, frame.y, frame.width, frame.height].iter().all(|v| v.is_finite());
        (finite && frame.width >= WINDOW_MIN_WIDTH && frame.height >= WINDOW_MIN_HEIGHT).then_some(frame)
    }

    /// Where to open a window saved as `self` with these displays attached (the primary first). One that is
    /// still reachable stays; one whose display went away, or that sits so far off the ones left that nothing
    /// of it could be grabbed, is put on its own display (else the primary) with room round it.
    pub fn fitted(mut self, displays: &[DisplayArea]) -> Self {
        if self.mode != FrameMode::Windowed || displays.is_empty() {
            return self;
        }
        let reachable = |d: &DisplayArea| {
            let (w, h) = d.overlap(&self);
            w >= REACHABLE.0.min(self.width) && h >= REACHABLE.1.min(self.height)
        };
        if displays.iter().any(reachable) {
            return self;
        }
        let home = self.display.and_then(|id| displays.iter().find(|d| d.id == id)).unwrap_or(&displays[0]);
        self.width = self.width.min(home.width - 2.0 * MARGIN).max(WINDOW_MIN_WIDTH);
        self.height = self.height.min(home.height - 2.0 * MARGIN).max(WINDOW_MIN_HEIGHT);
        self.x = home.x + MARGIN;
        self.y = home.y + MARGIN;
        self.display = Some(home.id);
        self
    }

    /// Where a window opened from this one goes: a little down and to the right, or back to the corner of the
    /// display when that would run off it.
    pub fn cascaded(&self, display: &DisplayArea) -> Self {
        let (x, y) = (self.x + CASCADE_STEP, self.y + CASCADE_STEP);
        let fits = display.holds(x, y) && display.holds(x + self.width, y + self.height);
        if fits {
            return Self {
                x,
                y,
                mode: FrameMode::Windowed,
                minimised: false,
                display: Some(display.id),
                ..*self
            };
        }
        Self {
            mode: FrameMode::Windowed,
            x: display.x + MARGIN,
            y: display.y + MARGIN,
            width: self.width.min(display.width - 2.0 * MARGIN),
            height: self.height.min(display.height - 2.0 * MARGIN),
            display: Some(display.id),
            minimised: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: DisplayArea = DisplayArea { id: 1, x: 0.0, y: 25.0, width: 1440.0, height: 875.0 };
    const SIDE: DisplayArea = DisplayArea { id: 2, x: 1440.0, y: 0.0, width: 1920.0, height: 1080.0 };

    #[test]
    fn a_record_round_trips_and_garbage_is_refused() {
        let frame = WindowFrame {
            display: Some(2),
            minimised: true,
            ..WindowFrame::windowed(10.0, 20.0, 900.0, 600.0)
        };
        assert_eq!(WindowFrame::from_json(&frame.to_json()), Some(frame));
        assert_eq!(WindowFrame::from_json("{"), None);
        assert_eq!(WindowFrame::from_json(&WindowFrame::windowed(0.0, 0.0, 100.0, 100.0).to_json()), None);
        assert_eq!(
            WindowFrame::from_json(r#"{"mode":"windowed","x":null,"y":0,"width":900,"height":600}"#),
            None
        );
    }

    #[test]
    fn the_gpui_records_read_as_they_are() {
        let json = r#"{"mode":"maximized","x":0.0,"y":0.0,"width":1320.0,"height":820.0}"#;
        let frame = WindowFrame::from_json(json).expect("a GPUI record");
        assert_eq!((frame.mode, frame.display, frame.minimised), (FrameMode::Maximized, None, false));
    }

    #[test]
    fn a_reachable_window_stays_where_it_was() {
        let frame = WindowFrame::windowed(1500.0, 100.0, 1000.0, 700.0);
        assert_eq!(frame.fitted(&[MAIN, SIDE]), frame);
        // Half off the edge of a display, a corner still in reach.
        let half = WindowFrame::windowed(-500.0, 100.0, 1000.0, 700.0);
        assert_eq!(half.fitted(&[MAIN]), half);
    }

    #[test]
    fn a_window_whose_display_went_comes_back_on_the_one_that_is_left() {
        let gone = WindowFrame { display: Some(2), ..WindowFrame::windowed(2000.0, 200.0, 1000.0, 700.0) };
        let fitted = gone.fitted(&[MAIN]);
        assert_eq!((fitted.x, fitted.y, fitted.display), (40.0, 65.0, Some(1)));
        assert_eq!((fitted.width, fitted.height), (1000.0, 700.0));
    }

    #[test]
    fn a_window_bigger_than_the_display_left_is_cut_to_it_but_never_below_a_window() {
        let big = WindowFrame::windowed(5000.0, 0.0, 3000.0, 2000.0).fitted(&[MAIN]);
        assert_eq!((big.width, big.height), (1360.0, 795.0));
    }

    #[test]
    fn maximised_and_full_screen_windows_are_the_system_s_to_place() {
        let full =
            WindowFrame { mode: FrameMode::Fullscreen, ..WindowFrame::windowed(9000.0, 0.0, 900.0, 600.0) };
        assert_eq!(full.fitted(&[MAIN]), full);
    }

    #[test]
    fn a_new_window_steps_down_and_right_or_wraps_to_the_corner() {
        let from = WindowFrame::windowed(100.0, 100.0, 800.0, 600.0);
        let next = from.cascaded(&MAIN);
        assert_eq!((next.x, next.y, next.width), (126.0, 126.0, 800.0));
        let low = WindowFrame::windowed(100.0, 400.0, 800.0, 600.0).cascaded(&MAIN);
        assert_eq!((low.x, low.y), (40.0, 65.0));
        assert_eq!(low.height, 600.0);
        let from_minimised = WindowFrame { minimised: true, ..from }.cascaded(&MAIN);
        assert!(!from_minimised.minimised);
    }
}
