//! A terminal's status changing: one row between two quiet ones goes from idle to working, to asking
//! for input, to done and back to idle, as the chosen `StatusMotion` plays each change. The row is the
//! sidebar's own, drawn from the pose core gives (`pose::status`) at the time the loop says, so its
//! tile's light, edge and badge, the card's wash, the row's nudge and the second line's roll are what
//! the list does. Classic is a cut: the row has the new status from the first moment of each change.

use gpui_kit::*;
use wormhole_core::view::motion::StatusMotion;

use super::{IDLE, RUNNING, Spot, metrics, progress, row, since};
use crate::settings::{UiColors, settings};
use crate::settings_window::sections::samples::{FLAKY_LOGIN, RATE_LIMITS, RELEASE_NOTES, Sample};
use crate::style::{ShiftClock, StatusShift, TileFace, preview_stage};

/// The terminal whose status changes, in each status it goes through: what the agents' sample list
/// shows of its three agents, as one agent's.
const STATES: [Sample; 4] = [
    Sample { glyph: IDLE.glyph, detail: IDLE.detail, tone: IDLE.tone, selected: false, ..RATE_LIMITS },
    Sample { selected: false, ..RATE_LIMITS },
    Sample {
        glyph: FLAKY_LOGIN.glyph,
        detail: FLAKY_LOGIN.detail,
        tone: FLAKY_LOGIN.tone,
        selected: false,
        ..RATE_LIMITS
    },
    Sample {
        glyph: RELEASE_NOTES.glyph,
        detail: RELEASE_NOTES.detail,
        tone: RELEASE_NOTES.tone,
        selected: false,
        ..RATE_LIMITS
    },
];

/// When each change begins, as shares of the loop: to working, to asking, to done, and back to idle,
/// each a second apart (`LOOP` is four), so the longest of them (the wash's second) has gone when the next
/// begins. The loop starts idle, the picture before anything moves.
const BEGINS: [f32; 4] = [0.1, 0.35, 0.6, 0.85];

/// The status the row is in at `t` of the loop (its place among [`STATES`]) and, while the change into it
/// plays, what it came from and how far along the change is.
fn at(motion: StatusMotion, t: f32) -> (usize, Option<StatusShift>) {
    let changes = BEGINS.iter().filter(|&&begins| t >= begins).count();
    let now = changes % STATES.len();
    if changes == 0 {
        return (now, None);
    }
    let from = &STATES[(now + STATES.len() - 1) % STATES.len()];
    let began = BEGINS[changes - 1];
    let shift = StatusShift { motion, from: from.glyph, clock: ShiftClock::At(0.0) };
    let total = shift.duration(STATES[now].glyph);
    let clock = ShiftClock::At(progress(since(t, began), total));
    (now, Some(StatusShift { clock, ..shift }).filter(|shift| shift.is_shown(STATES[now].glyph)))
}

pub(super) fn stage(ui: UiColors, t: f32, cx: &App) -> Div {
    let row_h = metrics().row_h;
    let (now, shift) = at(settings(cx).motion.status, t);
    let sample = &STATES[now];
    let nudge = shift.map_or(0.0, |shift| match shift.clock {
        ShiftClock::At(done) => shift.pose(sample.glyph, done).dx,
        ShiftClock::Playing(_) => 0.0,
    });
    let was = shift.map(|shift| {
        let from = STATES.iter().find(|state| state.glyph == shift.from).unwrap_or(&STATES[0]);
        (from, shift)
    });
    let changing = Spot::row(row_h, row_h)
        .moved(nudge, 0.0)
        .put(|| sample.row_shifting("motion-status", ui, metrics(), (&TileFace::Terminal, false), was, cx));
    preview_stage(ui, 3.0 * row_h)
        .children(Spot::row(0.0, row_h).put(|| row(&IDLE, "motion-status-above", ui, cx)))
        .children(changing)
        .children(Spot::row(2.0 * row_h, row_h).put(|| row(&RUNNING, "motion-status-below", ui, cx)))
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::motion::pose::status;
    use wormhole_core::view::motion::{MotionChoice, StatusMotion};

    use super::{BEGINS, STATES, at};
    use crate::settings_window::sections::motion_preview::LOOP;
    use crate::style::ShiftClock;

    fn clock(shift: Option<crate::style::StatusShift>) -> f32 {
        match shift.map(|shift| shift.clock) {
            Some(ShiftClock::At(t)) => t,
            other => panic!("a preview gives the time, not an animation: {other:?}"),
        }
    }

    #[test]
    fn the_loop_starts_idle_and_every_variant_comes_back_to_it() {
        for &motion in StatusMotion::ALL {
            assert_eq!(at(motion, 0.0), (0, None), "{motion:?}: the picture before anything moves");
            let (last, _) = at(motion, 0.999);
            assert_eq!(last, 0, "{motion:?}: idle again at the end, so the loop is seamless");
        }
    }

    #[test]
    fn the_row_goes_through_working_asking_and_done_in_order() {
        let order: Vec<usize> =
            BEGINS.iter().map(|&begins| at(StatusMotion::Classic, begins + 0.001).0).collect();
        assert_eq!(order, [1, 2, 3, 0]);
    }

    #[test]
    fn a_change_is_played_by_the_pose_core_gives_at_the_time_the_loop_says() {
        for &motion in StatusMotion::ALL {
            // A quarter of the way through the change to asking for input (every variant has something for it).
            let (from, to) = (STATES[1].glyph, STATES[2].glyph);
            let total = status::duration(motion, from.into(), to.into());
            if total.is_zero() {
                assert_eq!(at(motion, BEGINS[1] + 0.05).1, None, "{motion:?}: a cut");
                continue;
            }
            let begun = BEGINS[1] + 0.25 * total.as_secs_f32() / LOOP.as_secs_f32();
            let (now, shift) = at(motion, begun);
            assert_eq!(now, 2, "{motion:?}");
            assert!((clock(shift) - 0.25).abs() < 1e-3, "{motion:?}: {}", clock(shift));
            assert_eq!(shift.map(|shift| shift.from), Some(from), "{motion:?}: from working");
        }
    }

    #[test]
    fn a_change_has_all_but_gone_when_the_next_begins() {
        // The wash is the longest, a few ms over the second each change has: what is left of it is nothing.
        for &motion in StatusMotion::ALL {
            let (now, shift) = at(motion, BEGINS[1] - 0.0001);
            let Some(shift) = shift else { continue };
            let left = shift.pose(STATES[now].glyph, clock(Some(shift)));
            assert!(left.wash < 0.03 && left.dx == 0.0 && left.flare.strength < 0.01, "{motion:?}: {left:?}");
            assert!(left.leaving == 0.0 && left.ring > 0.99 && left.roll == 1.0, "{motion:?}: {left:?}");
        }
    }
}
