//! A looping scene as spans of its own time: when a thing starts to move and when it has arrived,
//! as shares of the loop's length. The Settings → Motion previews loop one scene each; this is the
//! arithmetic they share, so it is tested here and the app only draws what it says.

/// A stretch of a loop, `start` to `end`, both as shares of its length (0.0..=1.0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub start: f32,
    pub end: f32,
}

impl Span {
    pub const fn new(start: f32, end: f32) -> Self {
        Self { start, end }
    }

    /// How far `t` is through the span: 0 before it, 1 after it, linear between. An empty span is a
    /// cut: 0 before its start, 1 from it on.
    #[must_use]
    pub fn progress(self, t: f32) -> f32 {
        if t < self.start {
            0.0
        } else if t >= self.end {
            1.0
        } else {
            (t - self.start) / (self.end - self.start)
        }
    }

    /// Whether `t` is inside the span (its start included, its end not).
    #[must_use]
    pub fn contains(self, t: f32) -> bool {
        (self.start..self.end).contains(&t)
    }
}

/// How much of something is there at `t`: 0 until `rise` starts, up to 1 across `rise`, 1 until
/// `fall` starts, back to 0 across `fall`, 0 after; `ease` shapes each of the two ways (`|p| p` for
/// linear). The loop's start is at rest (0), so a still frame (Reduce Motion shows the first) is the
/// "before" picture.
#[must_use]
pub fn presence(t: f32, rise: Span, fall: Span, ease: impl Fn(f32) -> f32) -> f32 {
    ease(rise.progress(t)) - ease(fall.progress(t))
}

#[cfg(test)]
mod tests {
    use super::{Span, presence};

    fn linear(progress: f32) -> f32 {
        progress
    }

    #[test]
    fn progress_is_zero_before_one_after_and_linear_between() {
        let span = Span::new(0.2, 0.4);
        assert_eq!(span.progress(0.0), 0.0);
        assert_eq!(span.progress(0.2), 0.0);
        assert!((span.progress(0.3) - 0.5).abs() < 1e-6);
        assert_eq!(span.progress(0.4), 1.0);
        assert_eq!(span.progress(0.99), 1.0);
    }

    #[test]
    fn an_empty_span_is_a_cut() {
        let cut = Span::new(0.5, 0.5);
        assert_eq!((cut.progress(0.49), cut.progress(0.5), cut.progress(0.9)), (0.0, 1.0, 1.0));
        assert!(!cut.contains(0.5), "nothing is inside it");
    }

    #[test]
    fn presence_comes_in_stays_and_goes_and_the_loop_starts_without_it() {
        let (rise, fall) = (Span::new(0.1, 0.2), Span::new(0.6, 0.7));
        assert_eq!(presence(0.0, rise, fall, linear), 0.0, "the first frame is the picture before");
        assert!((presence(0.15, rise, fall, linear) - 0.5).abs() < 1e-6);
        assert_eq!(presence(0.4, rise, fall, linear), 1.0);
        assert!((presence(0.65, rise, fall, linear) - 0.5).abs() < 1e-6);
        assert_eq!(presence(0.9, rise, fall, linear), 0.0);
        // An easing shapes each way, not the ends.
        let squared = |p: f32| p * p;
        assert!((presence(0.15, rise, fall, squared) - 0.25).abs() < 1e-6);
        assert_eq!(presence(0.4, rise, fall, squared), 1.0);
        // Whatever the time, it stays between nothing and all of it.
        for step in 0..=100 {
            let at = presence(step as f32 / 100.0, rise, fall, linear);
            assert!((0.0..=1.0).contains(&at), "{at}");
        }
    }
}
