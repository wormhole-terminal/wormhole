//! The springs the motions run on, as the two numbers that make one. The mass is always 1, so the
//! app builds a GPUI `SpringConfig` from these and nothing else.

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
}
