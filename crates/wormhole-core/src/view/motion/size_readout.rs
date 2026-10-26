//! The chip that says a terminal's size (`120 × 40`) while its window is being resized and after the text
//! was zoomed, which the design fixed. It stays [`HOLD`] after the last change, then fades over [`FADE`];
//! under Reduce Motion it is cut instead of fading. It is never shown when a window opens: only a change
//! starts it. These are its timing; the words and the drawing are the app's.

use std::time::Duration;

/// How long the chip stays whole after the last change of size.
pub const HOLD: Duration = Duration::from_millis(700);
/// How long it takes to fade away after that.
pub const FADE: Duration = Duration::from_millis(150);

/// Where the chip is, `since` the last change of size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Whole, for this much longer.
    Holding(Duration),
    /// Fading, this much of it still shows (1.0 down to 0.0).
    Fading(f32),
    /// Gone.
    Gone,
}

/// The chip's phase `since` the last change; `cut` (Reduce Motion) goes from whole to gone without fading.
#[must_use]
pub fn phase(since: Duration, cut: bool) -> Phase {
    if since < HOLD {
        return Phase::Holding(HOLD - since);
    }
    let into = since - HOLD;
    if cut || into >= FADE {
        return Phase::Gone;
    }
    Phase::Fading(1.0 - into.as_secs_f32() / FADE.as_secs_f32())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn the_chip_is_whole_for_700_ms_after_the_last_change() {
        assert_eq!(phase(ms(0), false), Phase::Holding(ms(700)));
        assert_eq!(phase(ms(699), false), Phase::Holding(ms(1)));
    }

    #[test]
    fn it_fades_over_150_ms_and_is_gone_after() {
        assert_eq!(phase(ms(700), false), Phase::Fading(1.0));
        let Phase::Fading(half) = phase(ms(775), false) else { panic!("fading") };
        assert!((half - 0.5).abs() < 0.001, "{half}");
        assert_eq!(phase(ms(850), false), Phase::Gone);
        assert_eq!(phase(ms(5000), false), Phase::Gone);
    }

    #[test]
    fn under_reduce_motion_it_is_cut_at_700_ms() {
        assert_eq!(phase(ms(699), true), Phase::Holding(ms(1)));
        assert_eq!(phase(ms(700), true), Phase::Gone);
        assert_eq!(phase(ms(720), true), Phase::Gone);
    }

    #[test]
    fn it_only_ever_fades_out() {
        let mut before = 1.0;
        for at in (700..850).step_by(10) {
            let Phase::Fading(now) = phase(ms(at), false) else { panic!("fading at {at}") };
            assert!(now <= before);
            before = now;
        }
    }
}
