//! The text size easing to a new one when ⌘+, ⌘− or ⌘0 is pressed, which the design fixed: 120 ms,
//! ease-out, and a key pressed again while it moves starts from the size it has reached. The sizes it passes
//! through are quantised to [`QUANTUM`], so the glyph atlas and the shaping cache hold a bounded set of
//! sizes however often the keys are pressed; the size it ends on is exactly the one asked for. Sizes are the
//! zoom's: points over the size in Settings.

use std::time::{Duration, Instant};

use super::ease::EASE_OUT;

/// How long the size takes to arrive.
pub const DURATION: Duration = Duration::from_millis(120);

/// The size is a multiple of this, in points, on its way.
pub const QUANTUM: f32 = 0.25;

/// A size on its way from one value to another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ease {
    from: f32,
    to: f32,
    began: Instant,
}

/// The size at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub size: f32,
    /// It has arrived: `size` is the target, and nothing moves any more.
    pub done: bool,
}

impl Ease {
    /// From `from` to `to`, beginning at `began`.
    #[must_use]
    pub fn new(from: f32, to: f32, began: Instant) -> Self {
        Self { from, to, began }
    }

    /// The size where the ease stands at `now`.
    #[must_use]
    pub fn at(&self, now: Instant) -> Sample {
        let t = now.saturating_duration_since(self.began).as_secs_f32() / DURATION.as_secs_f32();
        if t >= 1.0 {
            return Sample { size: self.to, done: true };
        }
        Sample { size: quantise(self.from + (self.to - self.from) * EASE_OUT.ease(t)), done: false }
    }

    /// The same ease turned to a new target at `now`: it goes on from the size it has reached (not rounded to
    /// the quantum, so a size does not jump), over a whole [`DURATION`].
    #[must_use]
    pub fn retarget(&self, now: Instant, to: f32) -> Self {
        let t = now.saturating_duration_since(self.began).as_secs_f32() / DURATION.as_secs_f32();
        let here = if t >= 1.0 { self.to } else { self.from + (self.to - self.from) * EASE_OUT.ease(t) };
        Self { from: here, to, began: now }
    }

    /// Where it is going.
    #[must_use]
    pub fn target(&self) -> f32 {
        self.to
    }
}

/// `size` to the nearest multiple of [`QUANTUM`].
#[must_use]
pub fn quantise(size: f32) -> f32 {
    (size / QUANTUM).round() * QUANTUM
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: Instant, ms: u64) -> Instant {
        start + Duration::from_millis(ms)
    }

    #[test]
    fn it_starts_where_it_was_and_ends_exactly_on_the_target() {
        let start = Instant::now();
        let ease = Ease::new(0.0, 1.0, start);
        assert_eq!(ease.at(start), Sample { size: 0.0, done: false });
        assert_eq!(ease.at(at(start, 120)), Sample { size: 1.0, done: true });
        assert_eq!(ease.at(at(start, 900)), Sample { size: 1.0, done: true });
    }

    #[test]
    fn the_end_is_not_rounded_to_the_quantum() {
        let start = Instant::now();
        assert_eq!(Ease::new(0.0, 0.3, start).at(at(start, 120)).size, 0.3);
    }

    #[test]
    fn the_sizes_on_the_way_are_multiples_of_a_quarter_point_and_only_grow() {
        let start = Instant::now();
        let ease = Ease::new(2.0, 5.0, start);
        let mut before = 2.0;
        for ms in 0..120 {
            let sample = ease.at(at(start, ms));
            assert!(!sample.done);
            assert_eq!(sample.size, quantise(sample.size), "at {ms} ms: {}", sample.size);
            assert!(sample.size >= before && sample.size <= 5.0, "at {ms} ms: {}", sample.size);
            before = sample.size;
        }
    }

    #[test]
    fn a_zoom_out_only_shrinks_and_the_number_of_sizes_is_bounded() {
        let start = Instant::now();
        let ease = Ease::new(3.0, 2.0, start);
        let mut seen = Vec::new();
        for ms in 0..120 {
            let size = ease.at(at(start, ms)).size;
            if seen.last() != Some(&size) {
                seen.push(size);
            }
        }
        assert!(seen.windows(2).all(|pair| pair[1] < pair[0]), "{seen:?}");
        assert!(seen.len() <= 5, "a point is four quarters: {seen:?}");
    }

    #[test]
    fn it_eases_out_so_most_of_the_way_is_gone_by_the_middle() {
        let start = Instant::now();
        let halfway = Ease::new(0.0, 1.0, start).at(at(start, 60)).size;
        assert!(halfway > 0.5, "{halfway}");
    }

    #[test]
    fn a_key_pressed_again_goes_on_from_the_size_reached() {
        let start = Instant::now();
        let first = Ease::new(0.0, 1.0, start);
        let now = at(start, 40);
        let reached = first.at(now).size;
        let second = first.retarget(now, 2.0);
        assert_eq!(second.target(), 2.0);
        let begin = second.at(now).size;
        assert!((begin - reached).abs() <= QUANTUM, "no jump: {begin} after {reached}");
        assert_eq!(
            second.at(at(now, 120)),
            Sample { size: 2.0, done: true },
            "a whole duration from the press"
        );
        assert!(!second.at(at(now, 119)).done);
    }

    #[test]
    fn a_key_pressed_after_it_arrived_starts_from_the_target() {
        let start = Instant::now();
        let first = Ease::new(0.0, 1.0, start);
        let second = first.retarget(at(start, 500), 0.0);
        assert_eq!(second.at(at(start, 500)).size, 1.0);
    }
}
