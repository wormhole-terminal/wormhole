//! How a motion eases: CSS-style cubic-bezier curves as GPUI easing functions.

/// A CSS `cubic-bezier(x1, y1, x2, y2)` timing curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier(pub f32, pub f32, pub f32, pub f32);

impl CubicBezier {
    /// The curve as an easing function for `Animation::with_easing`.
    pub fn easing(self) -> impl Fn(f32) -> f32 + 'static {
        let CubicBezier(x1, y1, x2, y2) = self;
        gpui_kit::base::animation::cubic_bezier(x1, y1, x2, y2)
    }
}

/// Decelerates: things arriving.
pub const EASE_OUT: CubicBezier = CubicBezier(0.2, 0.0, 0.0, 1.0);
/// Accelerates: things leaving.
pub const EASE_IN: CubicBezier = CubicBezier(0.3, 0.0, 1.0, 1.0);
/// Size changes in place.
pub const EASE_IN_OUT: CubicBezier = CubicBezier(0.4, 0.0, 0.2, 1.0);
/// The spinner's rotation, and nothing else.
pub const LINEAR: CubicBezier = CubicBezier(0.0, 0.0, 1.0, 1.0);

#[cfg(test)]
mod tests {
    use super::{EASE_IN, EASE_IN_OUT, EASE_OUT, LINEAR};

    #[test]
    fn curves_start_at_rest_end_in_place_and_never_turn_back() {
        for curve in [EASE_OUT, EASE_IN, EASE_IN_OUT, LINEAR] {
            let ease = curve.easing();
            assert!(ease(0.0).abs() < 1e-3, "{curve:?}");
            assert!((ease(1.0) - 1.0).abs() < 1e-3, "{curve:?}");
            let mut last = 0.0;
            for step in 1..=100 {
                let value = ease(step as f32 / 100.0);
                assert!(value >= last - 1e-4, "{curve:?} turns back at {step}");
                last = value;
            }
        }
    }

    #[test]
    fn arriving_is_quick_at_first_and_leaving_slow() {
        assert!(EASE_OUT.easing()(0.25) > 0.5);
        assert!(EASE_IN.easing()(0.25) < 0.15);
        assert!((LINEAR.easing()(0.25) - 0.25).abs() < 1e-3);
    }
}
