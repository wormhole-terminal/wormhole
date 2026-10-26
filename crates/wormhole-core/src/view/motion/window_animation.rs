//! How a terminal window opens and closes (`NSWindowAnimationBehavior`), which the design fixed. Only the
//! windows that hold terminals use the system's own animation (`DocumentWindow`, which scales and fades the
//! window), and only when this build plays the fixed motions. Under Reduce Motion they are cut. A torn-off
//! window grows from its chip instead; the intro, Settings, the Theme Studio, panels and pop-ups keep the
//! system's default. Background mode keeps its windows behind everyone's and still, so a test instance is
//! never touched.

/// What the decision is made from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// The Motion feature is on in this build.
    pub motion_feature: bool,
    /// The system asks for Reduce Motion.
    pub reduce_motion: bool,
    /// Background mode keeps new windows behind the others.
    pub background: bool,
    /// The window is a torn-off terminal's, which has a growth of its own.
    pub grows: bool,
    /// The platform has an animation to ask for.
    pub platform: bool,
}

/// What a window is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behaviour {
    /// The system's own: whatever AppKit does for a window nobody asked anything of.
    Default,
    /// The document window's scale and fade.
    System,
    /// None: a cut.
    Cut,
}

/// The behaviour a terminal window has with these facts.
#[must_use]
pub fn behaviour(facts: Facts) -> Behaviour {
    if !facts.platform || !facts.motion_feature || facts.background || facts.grows {
        Behaviour::Default
    } else if facts.reduce_motion {
        Behaviour::Cut
    } else {
        Behaviour::System
    }
}

/// Whether the window opens and closes with the system's animation.
#[must_use]
pub fn animates(facts: Facts) -> bool {
    behaviour(facts) == Behaviour::System
}

#[cfg(test)]
mod tests {
    use super::*;

    const ON: Facts =
        Facts { motion_feature: true, reduce_motion: false, background: false, grows: false, platform: true };

    #[test]
    fn a_terminal_window_animates_with_the_feature_on() {
        assert!(animates(ON));
        assert_eq!(behaviour(ON), Behaviour::System);
    }

    #[test]
    fn without_the_feature_in_background_mode_or_growing_the_system_decides_as_it_always_did() {
        for facts in [
            Facts { motion_feature: false, ..ON },
            Facts { background: true, ..ON },
            Facts { grows: true, ..ON },
            Facts { platform: false, ..ON },
        ] {
            assert_eq!(behaviour(facts), Behaviour::Default, "{facts:?}");
            assert!(!animates(facts));
        }
    }

    #[test]
    fn reduce_motion_is_a_cut_that_is_asked_for() {
        assert_eq!(behaviour(Facts { reduce_motion: true, ..ON }), Behaviour::Cut);
        assert!(!animates(Facts { reduce_motion: true, ..ON }));
        assert_eq!(
            behaviour(Facts { reduce_motion: true, motion_feature: false, ..ON }),
            Behaviour::Default,
            "with the feature off nothing is asked, Reduce Motion or not"
        );
    }
}
