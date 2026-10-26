//! The springs the motions run on, as the two numbers that make one. The mass is always 1, so the
//! app builds a GPUI `SpringConfig` from these and nothing else.
//!
//! [`SpringSpec::step`] is the spring's answer as a formula, the same one GPUI integrates (a test in the
//! app holds the two together): it is what a Settings preview, which has no element to hold a spring's
//! state, draws, and what [`SpringSpec::settle`] reads to say when a list move can stop redrawing.

use std::time::Duration;

/// A spring's stiffness and damping (mass 1): how hard it pulls and how much it resists.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringSpec {
    pub stiffness: f32,
    pub damping: f32,
}

impl SpringSpec {
    /// Quick and just under critical: a row snapping onto a target, with a hint of overshoot.
    pub const SNAP: Self = Self::new(400.0, 30.0);
    /// Slow and critically damped: things that settle without ever overshooting.
    pub const SOFT: Self = Self::new(170.0, 26.0);
    /// Loose: it overshoots and rings before it rests.
    pub const BOUNCY: Self = Self::new(300.0, 16.0);

    pub const fn new(stiffness: f32, damping: f32) -> Self {
        Self { stiffness, damping }
    }

    /// Where a spring that started at rest at 0, pulled toward 1, is after `seconds`: 0 at the start, near
    /// 1 at the end (past it on the way, for a spring under 1 in [`Self::damping_ratio`]).
    #[must_use]
    pub fn step(self, seconds: f32) -> f32 {
        if seconds <= 0.0 {
            return 0.0;
        }
        let natural = self.stiffness.sqrt();
        let ratio = self.damping_ratio();
        let decay = ratio * natural;
        let gone = (-decay * seconds).exp();
        if (ratio - 1.0).abs() < 1e-4 {
            1.0 - gone * (1.0 + natural * seconds)
        } else if ratio < 1.0 {
            let rate = natural * (1.0 - ratio * ratio).sqrt();
            1.0 - gone * ((rate * seconds).cos() + decay / rate * (rate * seconds).sin())
        } else {
            let rate = natural * (ratio * ratio - 1.0).sqrt();
            1.0 - gone * ((rate * seconds).cosh() + decay / rate * (rate * seconds).sinh())
        }
    }

    /// How long until the spring is within `tolerance` of its target (as a share of the distance it
    /// travels) for good: a list move that long has arrived, so its timer and its animation may end.
    #[must_use]
    pub fn settle(self, tolerance: f32) -> Duration {
        const STEP: f32 = 0.005;
        const LIMIT: f32 = 5.0;
        let mut last_away = 0.0;
        let mut at = STEP;
        while at < LIMIT {
            if (1.0 - self.step(at)).abs() > tolerance {
                last_away = at;
            }
            at += STEP;
        }
        Duration::from_secs_f32(last_away + STEP)
    }

    /// How damped the spring is: below 1 it overshoots (the lower, the longer it rings), at 1 it
    /// arrives as fast as it can without overshooting, above 1 it creeps in.
    #[must_use]
    pub fn damping_ratio(self) -> f32 {
        self.damping / (2.0 * self.stiffness.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::SpringSpec;

    /// The spring by small steps of time (semi-implicit Euler), the way a frame loop would run it.
    fn integrated(spec: SpringSpec, seconds: f32) -> f32 {
        let (mut position, mut velocity) = (0.0_f32, 0.0_f32);
        let dt = 1e-4;
        for _ in 0..(seconds / dt) as usize {
            velocity += (spec.stiffness * (1.0 - position) - spec.damping * velocity) * dt;
            position += velocity * dt;
        }
        position
    }

    #[test]
    fn the_presets_are_the_numbers_the_design_picked() {
        assert_eq!((SpringSpec::SNAP.stiffness, SpringSpec::SNAP.damping), (400.0, 30.0));
        assert_eq!((SpringSpec::SOFT.stiffness, SpringSpec::SOFT.damping), (170.0, 26.0));
        assert_eq!((SpringSpec::BOUNCY.stiffness, SpringSpec::BOUNCY.damping), (300.0, 16.0));
    }

    #[test]
    fn soft_never_overshoots_snap_barely_does_and_bouncy_rings() {
        assert!(SpringSpec::SOFT.damping_ratio() >= 0.99, "{}", SpringSpec::SOFT.damping_ratio());
        let snap = SpringSpec::SNAP.damping_ratio();
        assert!((0.7..1.0).contains(&snap), "{snap}");
        assert!(SpringSpec::BOUNCY.damping_ratio() < 0.5, "{}", SpringSpec::BOUNCY.damping_ratio());
    }

    #[test]
    fn the_step_is_the_spring_the_frame_loop_would_run() {
        for spec in [SpringSpec::SNAP, SpringSpec::SOFT, SpringSpec::BOUNCY, SpringSpec::new(100.0, 40.0)] {
            for seconds in [0.05, 0.1, 0.2, 0.4, 0.8] {
                let (formula, loop_) = (spec.step(seconds), integrated(spec, seconds));
                assert!((formula - loop_).abs() < 0.01, "{spec:?} at {seconds}: {formula} against {loop_}");
            }
        }
    }

    #[test]
    fn a_step_starts_at_rest_and_arrives() {
        for spec in [SpringSpec::SNAP, SpringSpec::SOFT, SpringSpec::BOUNCY] {
            assert_eq!(spec.step(0.0), 0.0);
            assert!((spec.step(5.0) - 1.0).abs() < 1e-3, "{spec:?}");
        }
    }

    #[test]
    fn soft_arrives_from_below_snap_barely_passes_and_bouncy_rings() {
        let peak = |spec: SpringSpec| (1..400).map(|n| spec.step(n as f32 * 0.005)).fold(0.0, f32::max);
        assert!(peak(SpringSpec::SOFT) <= 1.0 + 1e-3, "{}", peak(SpringSpec::SOFT));
        let snap = peak(SpringSpec::SNAP);
        assert!((1.005..1.1).contains(&snap), "{snap}");
        assert!(peak(SpringSpec::BOUNCY) > 1.15, "{}", peak(SpringSpec::BOUNCY));
    }

    #[test]
    fn a_spring_over_critical_creeps_in_without_passing() {
        let heavy = SpringSpec::new(100.0, 40.0);
        assert!(heavy.damping_ratio() > 1.0);
        assert!((1..400).all(|n| heavy.step(n as f32 * 0.005) < 1.0));
    }

    #[test]
    fn it_settles_in_a_fraction_of_a_second_and_the_softer_spring_takes_longer() {
        let (snap, soft) = (SpringSpec::SNAP.settle(0.002), SpringSpec::SOFT.settle(0.002));
        assert!((0.15..0.6).contains(&snap.as_secs_f32()), "{snap:?}");
        assert!((0.3..0.9).contains(&soft.as_secs_f32()), "{soft:?}");
        assert!(soft > snap);
        // For good: it is within the tolerance from then on.
        assert!(
            (1.0 - SpringSpec::BOUNCY.step(SpringSpec::BOUNCY.settle(0.002).as_secs_f32() + 0.01)).abs()
                <= 0.002
        );
    }
}
