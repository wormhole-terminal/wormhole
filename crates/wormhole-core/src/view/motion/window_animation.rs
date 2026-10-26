//! Whether a window opens and closes with the system's own animation (`NSWindowAnimationBehavior`
//! `DocumentWindow`, which scales and fades it), which the design fixed. Only the windows that hold
//! terminals use it, and only when this build plays the fixed motions and the person has not asked for
//! Reduce Motion. A torn-off window grows from its chip instead, and the intro, Settings, the Theme
//! Studio, panels and pop-ups keep the system's default. Background mode keeps its windows behind
//! everyone's and still, so a test instance never animates.

/// What the decision is made from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// The Motion feature is on in this build.
    pub motion_feature: bool,
    /// The system asks for Reduce Motion.
    pub reduce_motion: bool,
    /// Background mode keeps new windows behind the others.
    pub background: bool,
    /// The window opens as a torn-off terminal's, with a growth of its own.
    pub grows: bool,
    /// The platform has an animation to ask for.
    pub platform: bool,
}

/// Whether the window opens and closes with the system's animation.
#[must_use]
pub fn animates(facts: Facts) -> bool {
    facts.platform && facts.motion_feature && !facts.reduce_motion && !facts.background && !facts.grows
}

#[cfg(test)]
mod tests {
    use super::*;

    const ON: Facts =
        Facts { motion_feature: true, reduce_motion: false, background: false, grows: false, platform: true };

    #[test]
    fn a_terminal_window_animates_with_the_feature_on() {
        assert!(animates(ON));
    }

    #[test]
    fn each_thing_that_asks_for_a_cut_makes_one() {
        assert!(!animates(Facts { motion_feature: false, ..ON }), "the feature is off: today's cut");
        assert!(!animates(Facts { reduce_motion: true, ..ON }), "Reduce Motion");
        assert!(!animates(Facts { background: true, ..ON }), "a test instance stays still");
        assert!(!animates(Facts { grows: true, ..ON }), "a torn-off window has its own growth");
        assert!(!animates(Facts { platform: false, ..ON }), "nothing to ask for");
    }
}
