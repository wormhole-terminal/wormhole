//! The springs as GPUI knows them. Core's `SpringSpec` is the design's two numbers; GPUI's
//! `SpringConfig` also takes a mass, which is always 1 here, so a spring is its stiffness and damping
//! and nothing else. Use them with `with_spring` (GPUI holds the position and velocity under the
//! element's id, so a retarget carries on from where it was); under Reduce Motion that element shows
//! its target at once, and anything driven by hand asks `cx.reduce_motion()` itself.

use gpui_kit::SpringConfig;
use wormhole_core::view::motion::SpringSpec;

/// The mass of every spring: the design names a spring by stiffness and damping alone.
const MASS: f32 = 1.0;

/// `spec` as a GPUI spring.
pub const fn config(spec: SpringSpec) -> SpringConfig {
    SpringConfig::new(spec.stiffness, spec.damping, MASS)
}

/// Quick, a hint of overshoot: a row snapping onto a target.
pub const SNAP: SpringConfig = config(SpringSpec::SNAP);
/// Slow and without overshoot: rows settling and columns sliding.
pub const SOFT: SpringConfig = config(SpringSpec::SOFT);
/// Loose: overshoots and rings.
pub const BOUNCY: SpringConfig = config(SpringSpec::BOUNCY);

#[cfg(test)]
mod tests {
    use wormhole_core::view::motion::SpringSpec;

    use super::{BOUNCY, SNAP, SOFT, config};

    #[test]
    fn a_spec_becomes_a_unit_mass_spring_with_its_numbers() {
        for (spec, spring) in
            [(SpringSpec::SNAP, SNAP), (SpringSpec::SOFT, SOFT), (SpringSpec::BOUNCY, BOUNCY)]
        {
            assert_eq!((spring.stiffness, spring.damping, spring.mass), (spec.stiffness, spec.damping, 1.0));
            assert_eq!(spring, config(spec));
            // GPUI and core agree on how damped it is, so a preset can't drift from its description.
            assert!((spring.canonical().1 - spec.damping_ratio()).abs() < 1e-6);
        }
    }
}
