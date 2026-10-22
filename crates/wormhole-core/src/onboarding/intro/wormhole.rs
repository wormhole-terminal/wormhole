//! The wormhole run that Aurora and Tentacles share, in seconds: the funnel, the jump, the portal and
//! the window's arrival, up to the light that blooms behind the window, which is what tells the two
//! apart. The shader reads these same moments (`shader_constants`), so the window, the flow of the
//! funnel and the light never drift apart.
//!
//! ```text
//! 0 ─ gather ─ jump ────── cruise ────── exit ─ iris ─ bloom ─ hold ─ lift ─ end
//!     funnel    flash,      light speed    stop,   the portal   the light     it rises
//!     forms     push                       spring  opens        blooms       and fades
//! ```

use std::f32::consts::PI;

use super::{phase, smoothstep};

/// The whole run.
pub const SECONDS: f32 = 8.8;
/// By here the funnel is gathered round its throat: it starts quickly and settles.
pub const GATHER: f32 = 1.2;
/// The flash, and the push to light speed that follows it.
pub const JUMP: f32 = 1.25;
/// The drop out: speed ends dead, space springs back and the portal opens.
pub const EXIT: f32 = 4.6;
/// When the window arrives, just behind the flash of the drop out.
pub const ARRIVE: f32 = EXIT + 0.12;
/// When the light behind the window begins to rise away and fade.
pub const LIFT: f32 = 7.5;

// The moments come in order, the jump comes soon and the light has time to lift away: a change to
// one that breaks that fails the build, not only a test.
const _: () = assert!(0.0 < GATHER && GATHER <= JUMP && JUMP < EXIT && EXIT < ARRIVE && ARRIVE < LIFT);
const _: () = assert!(JUMP <= 1.5);
const _: () = assert!(SECONDS - LIFT >= 1.0);

/// The window's own motion at `seconds`: gone while the wormhole plays (leaving at the start when it
/// was showing, never there at all when it wasn't), then arriving with a spring, a touch small, past
/// full size and back.
pub fn window_motion(seconds: f32, start: super::Start) -> super::WindowMotion {
    let leave = match start {
        super::Start::Shown => 1.0 - smoothstep(0.0, 0.3, seconds),
        super::Start::Hidden => 0.0,
    };
    if seconds < ARRIVE {
        return super::WindowMotion { scale: 1.0, alpha: leave };
    }
    let since = seconds - ARRIVE;
    let spring = 1.0 - 0.14 * (-5.0 * since).exp() * (11.0 * since).cos();
    super::WindowMotion { scale: spring, alpha: leave.max(smoothstep(0.0, 0.25, since)) }
}

/// How fast the funnel's filaments flow at `seconds`, in log-depth units a second: a quick drift
/// inward that settles while the funnel gathers, a hard push into light speed at the jump, and at the
/// drop out a stop with a short pull the other way (the spring back).
pub fn speed(seconds: f32) -> f32 {
    let settling = 0.9 + 1.6 * (-seconds / 0.45).exp();
    let drift = -settling * (1.0 - smoothstep(JUMP - 0.45, JUMP + 0.15, seconds));
    // A hard push to speed and a cruise.
    let go = smoothstep(JUMP - 0.2, JUMP + 0.55, seconds);
    let rush = 13.0 * go * (1.0 - smoothstep(EXIT - 0.05, EXIT + 0.08, seconds));
    let back = phase(seconds, EXIT, EXIT + 0.45);
    let spring = if back > 0.0 && back < 1.0 { -4.0 * (PI * back).sin() } else { 0.0 };
    drift + rush + spring
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onboarding::intro::{Start, Style, travel, window_motion as motion, window_motion_from};

    fn t(seconds: f32) -> f32 {
        seconds / SECONDS
    }

    #[test]
    fn the_start_is_quick() {
        // Flowing inward from the first moment, settling as the funnel gathers.
        assert!(speed(0.0) < -2.0, "starts fast: {}", speed(0.0));
        assert!(speed(0.0) < speed(GATHER), "and settles");
    }

    #[test]
    fn the_jump_rushes_in_then_springs_back() {
        assert!(speed(1.0) < 0.0, "drifts inward while it gathers");
        assert!(speed(EXIT - 0.3) > 10.0, "rushes in");
        assert!(speed(EXIT + 0.2) < 0.0, "pulls back");
        assert_eq!(speed(EXIT + 1.0), 0.0, "and stops");
        let stopped = travel(Style::Aurora, t(EXIT));
        assert!(travel(Style::Aurora, t(EXIT + 1.0)) < stopped, "the spring back undoes a little");
    }

    #[test]
    fn the_window_arrives_after_the_jump_out() {
        assert_eq!(motion(Style::Aurora, t(EXIT)).alpha, 0.0);
        let arriving = motion(Style::Aurora, t(ARRIVE + 0.05));
        assert!(arriving.scale < 1.0 && arriving.alpha > 0.0);
        let settled = motion(Style::Aurora, t(ARRIVE + 1.5));
        assert!((settled.scale - 1.0).abs() < 0.01 && settled.alpha == 1.0);
    }

    #[test]
    fn a_window_that_was_never_shown_is_hidden_from_the_first_moment() {
        for style in [Style::Aurora, Style::Tentacles] {
            assert_eq!(window_motion_from(style, 0.0, Start::Hidden).alpha, 0.0, "{style:?}");
            assert_eq!(window_motion_from(style, 0.0, Start::Shown).alpha, 1.0, "{style:?}");
            // Either way it arrives the same.
            let late = t(ARRIVE + 1.5);
            assert_eq!(
                window_motion_from(style, late, Start::Hidden),
                window_motion_from(style, late, Start::Shown)
            );
        }
    }

    #[test]
    fn the_window_is_gone_for_the_whole_jump() {
        for seconds in [0.4, 1.0, JUMP, 3.0, EXIT - 0.1] {
            assert_eq!(motion(Style::Aurora, t(seconds)).alpha, 0.0, "at {seconds}");
        }
    }
}
