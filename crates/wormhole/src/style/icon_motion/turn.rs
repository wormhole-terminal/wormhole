//! Icons that turn: the + a quarter, the arrows of Refresh and Reset a whole turn, and the
//! chevron beside Open in… over while its menu is open. A plus and a whole turn of arrows look
//! as they did when they are done, so only the way there shows; the chevron comes to rest
//! pointing the other way.

use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::*;

use super::canvas::Canvas;
use crate::style::motion::{CubicBezier, EASE_IN, EASE_IN_OUT, EASE_OUT, MOTION_BASE};

/// The + on a click: quick, since it only turns a quarter.
const PLUS_TURN: Duration = Duration::from_millis(220);
/// Refresh's and Reset's arrows once round: slow enough to be followed.
const ARROWS_SPIN: Duration = Duration::from_millis(520);
/// The chevron turning over: a turn in place, like the section chevron's.
const CHEVRON_TURN: Duration = MOTION_BASE;

/// Which way the arrows go round, the way their icon curls: Refresh's clockwise, Reset's back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spin {
    Clockwise,
    CounterClockwise,
}

/// The + a quarter round on its `run`th click.
pub(super) fn quarter(icon: IconName, size: f32, run: u64) -> AnyElement {
    turning(icon, size, run, PLUS_TURN, EASE_OUT, quarter_angle)
}

/// The arrows once round, `way` round, on their `run`th click.
pub(super) fn spin(icon: IconName, size: f32, way: Spin, run: u64) -> AnyElement {
    turning(icon, size, run, ARROWS_SPIN, EASE_IN_OUT, move |t| spin_angle(way, t))
}

/// The chevron pointing up while `open` and down otherwise, turning over on the `run`th change.
/// A menu opens as things arrive (decelerating) and closes as they leave.
pub(super) fn chevron(icon: IconName, size: f32, open: bool, run: u64) -> AnyElement {
    let curve = if open { EASE_OUT } else { EASE_IN };
    turning(icon, size, run, CHEVRON_TURN, curve, move |t| chevron_angle(open, t))
}

/// `icon` turned by `angle(t)` over `duration` along `curve`. `angle` gives the icon's rest at 1,
/// which is also how it is drawn before any run.
fn turning(
    icon: IconName,
    size: f32,
    run: u64,
    duration: Duration,
    curve: CubicBezier,
    angle: impl Fn(f32) -> f32 + 'static,
) -> AnyElement {
    let drawn = move |angle: f32| {
        let icon = Canvas::new(size).icon(icon);
        // An icon that isn't turned is drawn plainly, as every other icon is.
        if angle == 0.0 { icon } else { icon.rotate(radians(angle)) }
    };
    if run == 0 {
        return drawn(angle(1.0)).into_any_element();
    }
    Canvas::new(size)
        .icon(icon)
        .with_animation(
            ("icon-turn", run),
            Animation::new(duration).with_easing(curve.easing()),
            move |_, t| drawn(angle(t)),
        )
        .into_any_element()
}

/// A quarter turn, back to 0 at the end: a + looks the same turned, and the end is its rest.
fn quarter_angle(t: f32) -> f32 {
    if t >= 1.0 { 0.0 } else { FRAC_PI_2 * t }
}

/// A whole turn, which ends at 0 as the icon rests.
fn spin_angle(way: Spin, t: f32) -> f32 {
    let turn = if t >= 1.0 { 0.0 } else { TAU * t };
    match way {
        Spin::Clockwise => turn,
        Spin::CounterClockwise => -turn,
    }
}

/// Half a turn between the chevron pointing down (0) and up (π). `open` is where it is going.
fn chevron_angle(open: bool, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    PI * if open { t } else { 1.0 - t }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI};

    use super::{Spin, chevron_angle, quarter_angle, spin_angle};

    #[test]
    fn a_plus_and_arrows_end_unturned_as_they_began() {
        assert_eq!(quarter_angle(0.0), 0.0);
        assert_eq!(quarter_angle(1.0), 0.0);
        assert!((quarter_angle(0.999) - FRAC_PI_2).abs() < 0.01);
        for way in [Spin::Clockwise, Spin::CounterClockwise] {
            assert_eq!(spin_angle(way, 1.0), 0.0);
        }
        assert!(spin_angle(Spin::Clockwise, 0.5) > 0.0);
        assert!(spin_angle(Spin::CounterClockwise, 0.5) < 0.0);
    }

    #[test]
    fn the_chevron_ends_where_its_menu_state_rests() {
        // Open it rests pointing up, closed down: the rest the icon is drawn in with no run.
        assert_eq!(chevron_angle(true, 1.0), PI);
        assert_eq!(chevron_angle(false, 1.0), 0.0);
        assert_eq!(chevron_angle(true, 0.0), 0.0);
        assert_eq!(chevron_angle(false, 0.0), PI);
    }
}
