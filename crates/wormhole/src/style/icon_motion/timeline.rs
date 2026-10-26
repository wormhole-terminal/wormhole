//! The maths of the motions' timelines. A motion is drawn from a function of its progress `t`,
//! 0 to 1, that is its rest at 1: the last frame GPUI shows under Reduce Motion, and the look the
//! icon keeps. These are small and pure so a test can say where each one starts and ends.

use crate::style::motion::CubicBezier;

/// How far `t` is through the stretch `from..to` of a timeline: 0 before it and 1 after.
pub(super) fn span(t: f32, from: f32, to: f32) -> f32 {
    ((t - from) / (to - from)).clamp(0.0, 1.0)
}

/// `x` along `curve`.
pub(super) fn eased(curve: CubicBezier, x: f32) -> f32 {
    curve.easing()(x)
}

#[cfg(test)]
mod tests {
    use super::span;

    #[test]
    fn a_span_is_before_it_0_after_it_1_and_between_in_proportion() {
        assert_eq!(span(0.1, 0.2, 0.6), 0.0);
        assert_eq!(span(0.7, 0.2, 0.6), 1.0);
        assert!((span(0.4, 0.2, 0.6) - 0.5).abs() < 1e-6);
    }
}
