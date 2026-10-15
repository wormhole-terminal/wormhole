//! The terminal's scrollbar, behaving as macOS's does: where the thumb sits, what a click on the
//! track does, how a drag maps to the history, and when an overlay scrollbar shows and fades.
//! The app only paints it and forwards the pointer.
//!
//! Everything is a function of numbers the view already has (the history's length, the rows on
//! screen, how far the view is scrolled back), so a frame costs a few multiplications.

use std::time::{Duration, Instant};

/// How the system draws scrollbars (System Settings → Appearance → Show scroll bars).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollerStyle {
    /// Thin thumbs over the content that show while scrolling.
    #[default]
    Overlay,
    /// "Always": a gutter with a track, taking its width from the content.
    Legacy,
}

/// Width the pane gives up for the legacy gutter.
pub const GUTTER_W: f32 = 14.0;
/// Width of the overlay thumb at rest, and under the pointer (with the track behind it).
pub const THUMB_W: f32 = 6.0;
pub const THUMB_WIDE_W: f32 = 10.0;
/// Room between the thumb and the pane's right edge.
pub const EDGE: f32 = 2.0;
/// How near the right edge the pointer brings the overlay scrollbar up.
pub const ZONE_W: f32 = 16.0;
/// The shortest the thumb gets, so it stays something to grab in a long history.
pub const MIN_THUMB: f32 = 24.0;

/// How long the overlay scrollbar stays after the last scroll, and how long it takes to fade.
pub const HOLD: Duration = Duration::from_millis(1000);
pub const FADE: Duration = Duration::from_millis(250);

/// The rows the scrollbar stands for and where the view is among them; every position follows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollbarGeometry {
    /// Lines above the screen.
    pub history: usize,
    /// Lines on screen.
    pub rows: usize,
    /// How many lines the view is scrolled back from the live screen.
    pub offset: usize,
    /// The track's height in points.
    pub track: f32,
}

/// What a point on the track is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Thumb,
    /// On the track above the thumb: toward older lines.
    Above,
    Below,
}

impl ScrollbarGeometry {
    /// Whether there is anything to scroll and room to show it.
    pub fn scrollable(&self) -> bool {
        self.history > 0 && self.rows > 0 && self.track > 0.0
    }

    /// The thumb's length: the share of the lines on screen, but not less than [`MIN_THUMB`]
    /// (nor more than the track).
    pub fn thumb_len(&self) -> f32 {
        let all = (self.history + self.rows) as f32;
        (self.track * self.rows as f32 / all).max(MIN_THUMB).min(self.track)
    }

    /// How far the thumb's top can move.
    fn travel(&self) -> f32 {
        self.track - self.thumb_len()
    }

    /// The thumb's top, from the track's top: the oldest line at 0, the live screen at the end.
    pub fn thumb_top(&self) -> f32 {
        let scrolled = self.history - self.offset.min(self.history);
        self.travel() * scrolled as f32 / self.history.max(1) as f32
    }

    /// What `y`, from the track's top, is.
    pub fn hit(&self, y: f32) -> Hit {
        let top = self.thumb_top();
        if y < top {
            Hit::Above
        } else if y >= top + self.thumb_len() {
            Hit::Below
        } else {
            Hit::Thumb
        }
    }

    /// How far back the view is with the thumb's top at `top`.
    pub fn offset_for_top(&self, top: f32) -> usize {
        let travel = self.travel();
        if travel <= 0.0 {
            return self.offset.min(self.history);
        }
        let scrolled = (top.clamp(0.0, travel) / travel * self.history as f32).round() as usize;
        self.history - scrolled.min(self.history)
    }

    /// The offset a drag gives with the pointer at `y` and `grab` points below the thumb's top
    /// where it took hold.
    pub fn offset_for_drag(&self, y: f32, grab: f32) -> usize {
        self.offset_for_top(y - grab)
    }

    /// A click on the track at `y` (not on the thumb). `jump` brings the thumb to the pointer;
    /// otherwise the view moves a page toward it, keeping a line of the old page in view.
    pub fn track_click(&self, y: f32, jump: bool) -> TrackClick {
        if jump {
            let grab = self.thumb_len() / 2.0;
            TrackClick { offset: self.offset_for_drag(y, grab), grab: Some(grab) }
        } else {
            let page = self.rows.saturating_sub(1).max(1);
            let offset = match self.hit(y) {
                Hit::Above => (self.offset + page).min(self.history),
                Hit::Below | Hit::Thumb => self.offset.saturating_sub(page),
            };
            TrackClick { offset, grab: None }
        }
    }
}

/// What a click on the track does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackClick {
    /// Where the view goes.
    pub offset: usize,
    /// For a jump: how far below the thumb's top the pointer is now, so that a drag goes on from
    /// there. A page leaves the thumb where it was.
    pub grab: Option<f32>,
}

/// Whether a click on the track jumps to the spot (the "Click in the scroll bar to" setting) or
/// pages, with Option held doing the other one, as macOS does.
pub fn jumps(jump_to_spot: bool, option: bool) -> bool {
    jump_to_spot != option
}

/// How the overlay scrollbar's show and fade goes on, for the app to wait on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reveal {
    /// 0 gone, 1 fully there.
    pub opacity: f32,
    /// What to wait for before it changes again.
    pub next: Next,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Nothing changes until something scrolls.
    Nothing,
    /// The fade begins after this long: sleep until then.
    After(Duration),
    /// It is fading: draw the next frame.
    Frame,
}

/// When the overlay scrollbar was last asked to show, and where the view was.
#[derive(Clone, Copy, Debug, Default)]
pub struct Visibility {
    shown_at: Option<Instant>,
    offset: usize,
}

impl Visibility {
    /// The scrollbar shows now and the idle time starts again.
    pub fn show(&mut self, now: Instant) {
        self.shown_at = Some(now);
    }

    /// The frame's offset: scrolling, by whatever means (wheel, keys, a jump to a match, a drag),
    /// shows the scrollbar. Returns whether it did.
    pub fn watch(&mut self, offset: usize, now: Instant) -> bool {
        if offset == self.offset {
            return false;
        }
        self.offset = offset;
        self.show(now);
        true
    }

    /// How the scrollbar looks at `now`. Held by the pointer (`held`) it stays; with Reduce Motion
    /// it goes without fading.
    pub fn reveal(&self, now: Instant, held: bool, reduce_motion: bool) -> Reveal {
        let gone = Reveal { opacity: 0.0, next: Next::Nothing };
        if held {
            return Reveal { opacity: 1.0, next: Next::Nothing };
        }
        let Some(shown_at) = self.shown_at else { return gone };
        let idle = now.saturating_duration_since(shown_at);
        if idle < HOLD {
            return Reveal { opacity: 1.0, next: Next::After(HOLD - idle) };
        }
        let faded = idle - HOLD;
        if reduce_motion || faded >= FADE {
            return gone;
        }
        Reveal { opacity: 1.0 - faded.as_secs_f32() / FADE.as_secs_f32(), next: Next::Frame }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry(history: usize, offset: usize) -> ScrollbarGeometry {
        ScrollbarGeometry { history, rows: 20, offset, track: 400.0 }
    }

    #[test]
    fn nothing_to_scroll_without_history() {
        assert!(!geometry(0, 0).scrollable());
        assert!(!ScrollbarGeometry { track: 0.0, ..geometry(10, 0) }.scrollable());
        assert!(geometry(1, 0).scrollable());
    }

    #[test]
    fn the_thumb_is_the_share_of_lines_on_screen_with_a_least_length() {
        assert_eq!(geometry(20, 0).thumb_len(), 200.0, "half of the lines");
        assert_eq!(geometry(100_000, 0).thumb_len(), MIN_THUMB);
        let tiny = ScrollbarGeometry { track: 10.0, ..geometry(100_000, 0) };
        assert_eq!(tiny.thumb_len(), 10.0, "never longer than the track");
    }

    #[test]
    fn the_thumb_sits_at_the_bottom_when_live_and_at_the_top_at_the_oldest_line() {
        let live = geometry(180, 0);
        assert_eq!(live.thumb_len(), 40.0);
        assert_eq!(live.thumb_top(), 360.0);
        assert_eq!(geometry(180, 180).thumb_top(), 0.0);
        assert_eq!(geometry(180, 90).thumb_top(), 180.0);
        assert_eq!(geometry(180, 500).thumb_top(), 0.0, "an offset past the history");
    }

    #[test]
    fn a_drag_maps_the_thumb_back_to_lines() {
        let g = geometry(180, 90);
        assert_eq!(g.offset_for_drag(g.thumb_top() + 7.0, 7.0), 90, "grabbing keeps the offset");
        assert_eq!(g.offset_for_top(0.0), 180);
        assert_eq!(g.offset_for_top(-50.0), 180, "above the track");
        assert_eq!(g.offset_for_top(360.0), 0);
        assert_eq!(g.offset_for_top(9999.0), 0, "below the track");
        for offset in [0, 1, 37, 90, 179, 180] {
            let g = geometry(180, offset);
            assert_eq!(g.offset_for_top(g.thumb_top()), offset, "round trip at {offset}");
        }
    }

    #[test]
    fn hits_tell_the_thumb_from_the_track_around_it() {
        let g = geometry(180, 90);
        assert_eq!(g.hit(10.0), Hit::Above);
        assert_eq!(g.hit(g.thumb_top()), Hit::Thumb);
        assert_eq!(g.hit(g.thumb_top() + 39.9), Hit::Thumb);
        assert_eq!(g.hit(g.thumb_top() + 40.0), Hit::Below);
    }

    #[test]
    fn a_track_click_pages_toward_the_pointer_or_jumps_to_it() {
        let g = geometry(180, 90);
        assert_eq!(g.track_click(10.0, false), TrackClick { offset: 109, grab: None });
        assert_eq!(g.track_click(390.0, false), TrackClick { offset: 71, grab: None });
        assert_eq!(geometry(180, 175).track_click(0.0, false).offset, 180, "stops at the oldest");
        assert_eq!(geometry(180, 5).track_click(399.0, false).offset, 0, "stops at the live screen");
        // A jump centers the thumb on the pointer and keeps hold of its middle.
        let jump = g.track_click(100.0, true);
        assert_eq!(jump.grab, Some(20.0));
        assert_eq!(jump.offset, g.offset_for_top(80.0));
        assert_eq!(g.track_click(0.0, true).offset, 180, "the top of the track is the oldest line");
        assert_eq!(g.track_click(400.0, true).offset, 0);
    }

    #[test]
    fn option_does_the_other_click() {
        assert!(!jumps(false, false));
        assert!(jumps(false, true));
        assert!(jumps(true, false));
        assert!(!jumps(true, true));
    }

    #[test]
    fn the_overlay_shows_on_scrolling_holds_a_second_and_fades() {
        let t0 = Instant::now();
        let mut v = Visibility::default();
        assert_eq!(v.reveal(t0, false, false).opacity, 0.0, "hidden until something scrolls");
        assert!(!v.watch(0, t0), "the first frame at the bottom is no scroll");
        assert!(v.watch(3, t0));
        assert!(!v.watch(3, t0), "an unmoved view is no scroll");
        let shown = v.reveal(t0 + Duration::from_millis(400), false, false);
        assert_eq!((shown.opacity, shown.next), (1.0, Next::After(Duration::from_millis(600))));
        let fading = v.reveal(t0 + HOLD + FADE / 2, false, false);
        assert!((fading.opacity - 0.5).abs() < 1e-3);
        assert_eq!(fading.next, Next::Frame);
        let gone = v.reveal(t0 + HOLD + FADE, false, false);
        assert_eq!((gone.opacity, gone.next), (0.0, Next::Nothing), "no frames are asked for once gone");
    }

    #[test]
    fn the_pointer_holds_it_and_reduce_motion_cuts_the_fade() {
        let t0 = Instant::now();
        let mut v = Visibility::default();
        v.show(t0);
        let late = t0 + Duration::from_secs(60);
        assert_eq!(v.reveal(late, true, false), Reveal { opacity: 1.0, next: Next::Nothing });
        assert_eq!(v.reveal(t0 + HOLD + FADE / 4, false, true).opacity, 0.0, "no fade");
        assert_eq!(v.reveal(t0 + HOLD / 2, false, true).opacity, 1.0);
    }
}
