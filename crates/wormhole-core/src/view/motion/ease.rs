//! The curves a motion eases along: CSS-style `cubic-bezier(x1, y1, x2, y2)`, and the four the design
//! uses. They are here, not in the app's style layer, because a pose (`pose`) is a function of the
//! motion's own time and eases inside it: the sidebar and the Settings previews then ask the same
//! function and the curve has one home. The app's `style::motion` re-exports them and hands
//! [`CubicBezier::easing`] to GPUI.

/// A CSS `cubic-bezier(x1, y1, x2, y2)` timing curve from (0, 0) to (1, 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier(pub f32, pub f32, pub f32, pub f32);

/// Decelerates: things arriving.
pub const EASE_OUT: CubicBezier = CubicBezier(0.2, 0.0, 0.0, 1.0);
/// Accelerates: things leaving.
pub const EASE_IN: CubicBezier = CubicBezier(0.3, 0.0, 1.0, 1.0);
/// Size changes in place.
pub const EASE_IN_OUT: CubicBezier = CubicBezier(0.4, 0.0, 0.2, 1.0);
/// The spinner's rotation, and nothing else.
pub const LINEAR: CubicBezier = CubicBezier(0.0, 0.0, 1.0, 1.0);

impl CubicBezier {
    /// How far along the curve is when `progress` (0.0..=1.0) of the time has passed: 0 at the start,
    /// 1 at the end, and `progress` itself outside of them.
    #[must_use]
    pub fn ease(self, progress: f32) -> f32 {
        if progress <= 0.0 || progress >= 1.0 {
            return progress.clamp(0.0, 1.0);
        }
        let CubicBezier(x1, y1, x2, y2) = self;
        let at = |a: f32, b: f32, s: f32| {
            3.0 * a * s * (1.0 - s) * (1.0 - s) + 3.0 * b * s * s * (1.0 - s) + s * s * s
        };
        let slope = |a: f32, b: f32, s: f32| {
            3.0 * a * (1.0 - s) * (1.0 - s) + 6.0 * (b - a) * s * (1.0 - s) + 3.0 * (1.0 - b) * s * s
        };
        // Newton's method finds the curve's parameter whose x is `progress`; where the curve is too
        // flat for it to be steady, bisection takes over, which always arrives.
        let mut s = progress;
        for _ in 0..8 {
            let error = at(x1, x2, s) - progress;
            if error.abs() < 1e-6 {
                return at(y1, y2, s);
            }
            let flatness = slope(x1, x2, s);
            if flatness.abs() < 1e-4 {
                break;
            }
            s -= error / flatness;
        }
        let (mut low, mut high) = (0.0_f32, 1.0_f32);
        s = progress;
        for _ in 0..24 {
            let x = at(x1, x2, s);
            if (x - progress).abs() < 1e-6 {
                break;
            }
            if x < progress {
                low = s
            } else {
                high = s
            }
            s = (low + high) / 2.0;
        }
        at(y1, y2, s)
    }

    /// The curve as a function, for what takes an easing (GPUI's `Animation::with_easing`).
    pub fn easing(self) -> impl Fn(f32) -> f32 + 'static {
        move |progress| self.ease(progress)
    }
}

#[cfg(test)]
mod tests {
    use super::{CubicBezier, EASE_IN, EASE_IN_OUT, EASE_OUT, LINEAR};

    #[test]
    fn curves_start_at_rest_end_in_place_and_never_turn_back() {
        for curve in [EASE_OUT, EASE_IN, EASE_IN_OUT, LINEAR] {
            assert_eq!(curve.ease(0.0), 0.0, "{curve:?}");
            assert_eq!(curve.ease(1.0), 1.0, "{curve:?}");
            let mut last = 0.0;
            for step in 1..=100 {
                let value = curve.ease(step as f32 / 100.0);
                assert!(value >= last - 1e-4, "{curve:?} turns back at {step}");
                last = value;
            }
        }
    }

    #[test]
    fn arriving_is_quick_at_first_and_leaving_slow() {
        assert!(EASE_OUT.ease(0.25) > 0.5);
        assert!(EASE_IN.ease(0.25) < 0.15);
        assert!((LINEAR.ease(0.25) - 0.25).abs() < 1e-3);
    }

    #[test]
    fn outside_the_time_it_holds_the_ends() {
        assert_eq!((EASE_OUT.ease(-1.0), EASE_OUT.ease(2.0)), (0.0, 1.0));
    }

    #[test]
    fn a_steep_start_still_finds_its_point() {
        // The curve is nearly vertical at the start: Newton's step is unsteady there.
        let steep = CubicBezier(0.0, 0.0, 0.0, 1.0);
        let early = steep.ease(0.001);
        assert!((0.0..=1.0).contains(&early), "{early}");
        assert!(steep.ease(0.5) > steep.ease(0.1));
    }

    #[test]
    fn the_function_form_is_the_same_curve() {
        let ease = EASE_IN_OUT.easing();
        assert_eq!(ease(0.3), EASE_IN_OUT.ease(0.3));
    }
}
