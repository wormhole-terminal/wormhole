//! A terminal's tile and row when its status changes, one function of the variant, the two statuses
//! and how far along the change is. The tile's light or edge and its badge, the card's wash, the
//! row's nudge and the second line's roll are fields of one [`StatusPose`], so the sidebar and the
//! Settings preview draw the same numbers. The stages (milliseconds, from the change):
//!
//! | Variant | What moves |
//! |---|---|
//! | Classic | nothing: the tile and the line change at once |
//! | Crossfade | the old light, edge and badge fade out while the new ones fade in, 160 |
//! | Badge pops | the old parts fade out, 80; then the new light or edge fades in, 160, and the badge grows from nothing on the bouncy spring |
//! | Card wash | the card is washed in the new status's colour, 120, and the wash fades away, 900 |
//! | Amber flare | needs input only: a ring leaves the tile's edge and fades, 700 |
//! | Nudge | needs input only: the row moves 4 pt aside and back, twice, 420 |
//! | Line rolls | the tile's parts crossfade, 120, as the second line rolls up to the new words, 160 |
//!
//! The three that are about the tile's parts (Crossfade, Badge pops, Line rolls) start from the old
//! status and play into the new one; the other three swap at once and add something on top. Either way
//! a change ends in the new status's resting look, which is [`StatusPose::REST`]. A variant that has
//! nothing to say for a change (a flare for a status that isn't asking for input) has no duration: the
//! change is a cut.
//!
//! `from` and `to` are [`Urgency`], what the tile says in the terms the sidebar picks by; the app
//! maps its glyphs onto them.

use std::time::Duration;

use super::{millis, progress};
use crate::view::motion::ease::{EASE_IN_OUT, EASE_OUT};
use crate::view::motion::{SpringSpec, StatusMotion};
use crate::view::sidebar::Urgency;

/// Crossfade: the parts changing places.
const CROSSFADE: f32 = 160.0;
/// Badge pops: the old parts leaving, the new light or edge fading in after them, and how long the
/// bouncy spring takes to come to rest within [`SETTLED`] (a test holds it to the spring).
const POP_OUT: f32 = 80.0;
const POP_IN: f32 = 160.0;
const POP_SPRING: f32 = 715.0;
/// How close to its place a spring is counted as arrived.
pub const SETTLED: f32 = 0.002;
/// Card wash: the tint coming up and going away.
const WASH_UP: f32 = 120.0;
const WASH_DOWN: f32 = 900.0;
/// Amber flare: the ring's time, how far it reaches (points) and how strong it starts.
const FLARE: f32 = 700.0;
const FLARE_REACH: f32 = 8.0;
const FLARE_START: f32 = 0.7;
/// Nudge: its time, and the row's offset (points) at the quarter marks of it, one per
/// `(share of the time, offset)`; between them the row moves in a straight line.
const NUDGE: f32 = 420.0;
const NUDGE_STEPS: [(f32, f32); 5] = [(0.0, 0.0), (0.2, 4.0), (0.45, 0.0), (0.65, 3.0), (1.0, 0.0)];
/// Line rolls: the parts' crossfade and the line's roll.
const ROLL_PARTS: f32 = 120.0;
const ROLL_LINE: f32 = 160.0;

/// How a tile and its row stand at a moment of a status change. At rest it is [`StatusPose::REST`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusPose {
    /// How much of the old status's light or edge and its badge shows (0.0 nothing).
    pub leaving: f32,
    /// How much of the new status's light or edge shows.
    pub ring: f32,
    /// The new status's badge.
    pub badge: Part,
    /// The ring that leaves the tile's edge.
    pub flare: Flare,
    /// How strong the wash over the card is (0.0 none, 1.0 its full tint).
    pub wash: f32,
    /// How far the row is moved sideways from its place, in points.
    pub dx: f32,
    /// How far the second line has rolled from the old words to the new: 0.0 the old, 1.0 the new.
    pub roll: f32,
}

/// A part of the tile at a moment: how much of it shows and how big it is next to its own size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    pub opacity: f32,
    /// 1.0 at its size, 0.0 nothing; a spring takes it past 1.0.
    pub scale: f32,
}

impl Part {
    pub const WHOLE: Self = Self { opacity: 1.0, scale: 1.0 };
    pub const NONE: Self = Self { opacity: 0.0, scale: 0.0 };
}

/// The ring a flare sends out from the tile's edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flare {
    /// How far outside the tile's edge the ring reaches, in points; its line is as thick.
    pub reach: f32,
    /// How strong it is (0.0 gone).
    pub strength: f32,
}

impl Flare {
    pub const NONE: Self = Self { reach: 0.0, strength: 0.0 };
}

impl StatusPose {
    /// The new status, standing.
    pub const REST: Self = Self {
        leaving: 0.0,
        ring: 1.0,
        badge: Part::WHOLE,
        flare: Flare::NONE,
        wash: 0.0,
        dx: 0.0,
        roll: 1.0,
    };

    /// Whether it shows the old status and none of the new: the picture before a change that is about
    /// the tile's parts.
    #[must_use]
    pub fn is_before(&self) -> bool {
        self.leaving == 1.0
            && self.ring == 0.0
            && (self.badge.opacity == 0.0 || self.badge.scale == 0.0)
            && self.flare == Flare::NONE
            && self.wash == 0.0
            && self.dx == 0.0
    }
}

/// The colour a wash is of: which of the status colours of the design it takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wash {
    /// Asking for input, or a bell: amber.
    Attention,
    /// Finished: green.
    Done,
    /// Failed.
    Danger,
    /// Working: the accent.
    Accent,
}

/// The wash a terminal that got into `to` gets, if it is a status worth one: the ones that say
/// something. A terminal that went quiet again, or whose command just runs, gets none.
#[must_use]
pub fn wash_of(to: Urgency) -> Option<Wash> {
    match to {
        Urgency::NeedsYou | Urgency::Bell => Some(Wash::Attention),
        Urgency::Done => Some(Wash::Done),
        Urgency::Failed => Some(Wash::Danger),
        Urgency::Working => Some(Wash::Accent),
        Urgency::NewOutput | Urgency::Running | Urgency::Idle => None,
    }
}

/// How long the change takes to play: none when it is a cut (Classic, no change at all, or a variant
/// with nothing to show for `to`).
#[must_use]
pub fn duration(variant: StatusMotion, from: Urgency, to: Urgency) -> Duration {
    if from == to {
        return Duration::ZERO;
    }
    let asking = to == Urgency::NeedsYou;
    millis(match variant {
        StatusMotion::Classic => 0.0,
        StatusMotion::Crossfade => CROSSFADE,
        StatusMotion::BadgePop => POP_OUT + POP_SPRING,
        StatusMotion::Wash if wash_of(to).is_some() => WASH_UP + WASH_DOWN,
        StatusMotion::Wash => 0.0,
        StatusMotion::Flare if asking => FLARE,
        StatusMotion::Nudge if asking => NUDGE,
        StatusMotion::Flare | StatusMotion::Nudge => 0.0,
        StatusMotion::LineRoll => ROLL_LINE,
    })
}

/// The tile and the row `t` (0.0..=1.0) of the way through the change's [`duration`].
#[must_use]
pub fn pose(variant: StatusMotion, from: Urgency, to: Urgency, t: f32) -> StatusPose {
    let total = duration(variant, from, to).as_secs_f32() * 1000.0;
    if total <= 0.0 || t >= 1.0 {
        return StatusPose::REST;
    }
    let at = t.max(0.0) * total;
    match variant {
        StatusMotion::Classic => StatusPose::REST,
        StatusMotion::Crossfade => {
            let across = EASE_IN_OUT.ease(progress(at, 0.0, CROSSFADE));
            StatusPose {
                leaving: 1.0 - across,
                ring: across,
                badge: Part { opacity: across, scale: 1.0 },
                ..StatusPose::REST
            }
        }
        StatusMotion::BadgePop => {
            let out = progress(at, 0.0, POP_OUT);
            let grown = progress(at, POP_OUT, POP_SPRING) * POP_SPRING / 1000.0;
            let popped = if at < POP_OUT { 0.0 } else { SpringSpec::BOUNCY.step(grown) };
            StatusPose {
                leaving: 1.0 - out,
                ring: EASE_OUT.ease(progress(at, POP_OUT, POP_IN)),
                badge: Part { opacity: 1.0, scale: popped },
                ..StatusPose::REST
            }
        }
        StatusMotion::Wash => {
            let up = EASE_OUT.ease(progress(at, 0.0, WASH_UP));
            let down = progress(at, WASH_UP, WASH_DOWN);
            StatusPose { wash: up * (1.0 - down), ..StatusPose::REST }
        }
        StatusMotion::Flare => {
            let out = EASE_OUT.ease(progress(at, 0.0, FLARE));
            StatusPose {
                flare: Flare { reach: FLARE_REACH * out, strength: FLARE_START * (1.0 - out) },
                ..StatusPose::REST
            }
        }
        StatusMotion::Nudge => StatusPose { dx: nudge_at(EASE_IN_OUT.ease(at / NUDGE)), ..StatusPose::REST },
        StatusMotion::LineRoll => {
            let across = EASE_IN_OUT.ease(progress(at, 0.0, ROLL_PARTS));
            StatusPose {
                leaving: 1.0 - across,
                ring: across,
                badge: Part { opacity: across, scale: 1.0 },
                roll: EASE_OUT.ease(progress(at, 0.0, ROLL_LINE)),
                ..StatusPose::REST
            }
        }
    }
}

/// The nudge's offset `share` of the way through it: a straight line between [`NUDGE_STEPS`].
fn nudge_at(share: f32) -> f32 {
    NUDGE_STEPS.windows(2).find(|pair| share <= pair[1].0).map_or(0.0, |pair| {
        let [(from, was), (to, now)] = [pair[0], pair[1]];
        was + (now - was) * (share - from) / (to - from)
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        Flare, NUDGE_STEPS, POP_OUT, POP_SPRING, SETTLED, StatusPose, Wash, duration, nudge_at, pose, wash_of,
    };
    use crate::view::motion::{MotionChoice, SpringSpec, StatusMotion};
    use crate::view::sidebar::Urgency;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// Every change between two statuses, for each variant.
    fn changes() -> impl Iterator<Item = (StatusMotion, Urgency, Urgency)> {
        StatusMotion::ALL.iter().flat_map(|&variant| {
            Urgency::ORDER
                .into_iter()
                .flat_map(move |from| Urgency::ORDER.into_iter().map(move |to| (variant, from, to)))
        })
    }

    #[test]
    fn a_change_ends_in_the_new_status_standing_and_starts_in_the_old_or_in_a_cut_to_the_new() {
        for (variant, from, to) in changes() {
            assert_eq!(pose(variant, from, to, 1.0), StatusPose::REST, "{variant:?} {from:?} {to:?}: end");
            let first = pose(variant, from, to, 0.0);
            // A flare starts with no reach: a ring of nothing is the new status standing.
            let unflared = StatusPose { flare: Flare::NONE, ..first };
            assert!(
                unflared == StatusPose::REST || first.is_before(),
                "{variant:?} {from:?} {to:?}: starts as {first:?}"
            );
            if duration(variant, from, to).is_zero() {
                assert_eq!(first, StatusPose::REST, "{variant:?} {from:?} {to:?}: a cut");
            }
        }
    }

    #[test]
    fn the_variants_that_are_about_the_parts_start_from_the_old_ones() {
        for variant in [StatusMotion::Crossfade, StatusMotion::BadgePop, StatusMotion::LineRoll] {
            let first = pose(variant, Urgency::Idle, Urgency::Working, 0.0);
            assert!(first.is_before(), "{variant:?}: {first:?}");
            assert_eq!(
                first.roll == 0.0,
                variant == StatusMotion::LineRoll,
                "{variant:?}: only a line that rolls starts on the old words"
            );
        }
        // The others swap at once and only add something on top.
        for variant in [StatusMotion::Wash, StatusMotion::Flare, StatusMotion::Nudge] {
            let first = pose(variant, Urgency::Working, Urgency::NeedsYou, 0.0);
            assert_eq!((first.leaving, first.ring, first.roll), (0.0, 1.0, 1.0), "{variant:?}");
        }
    }

    #[test]
    fn nothing_is_in_use_beyond_its_range_but_the_springs_overshoot() {
        for (variant, from, to) in changes() {
            for step in 0..=100 {
                let p = pose(variant, from, to, step as f32 / 100.0);
                for (name, value) in
                    [("leaving", p.leaving), ("ring", p.ring), ("roll", p.roll), ("wash", p.wash)]
                {
                    assert!((0.0..=1.0).contains(&value), "{variant:?} {name} {value}");
                }
                assert!((0.0..=1.0).contains(&p.badge.opacity), "{variant:?}");
                assert!(p.badge.scale >= 0.0 && p.badge.scale < 1.5, "{variant:?} {}", p.badge.scale);
            }
        }
    }

    #[test]
    fn the_durations_are_the_picks() {
        let (idle, working, asking, done) =
            (Urgency::Idle, Urgency::Working, Urgency::NeedsYou, Urgency::Done);
        assert_eq!(duration(StatusMotion::Classic, working, asking), Duration::ZERO);
        assert_eq!(duration(StatusMotion::Crossfade, working, asking), ms(160));
        assert_eq!(duration(StatusMotion::Wash, idle, done), ms(1020));
        assert_eq!(duration(StatusMotion::Flare, working, asking), ms(700));
        assert_eq!(duration(StatusMotion::Nudge, working, asking), ms(420));
        assert_eq!(duration(StatusMotion::LineRoll, idle, working), ms(160));
        assert_eq!(
            duration(StatusMotion::BadgePop, idle, done),
            ms(POP_OUT as u64 + POP_SPRING as u64),
            "the old parts leave, then the spring"
        );
    }

    #[test]
    fn a_variant_with_nothing_to_show_for_a_status_is_a_cut() {
        // Only asking for input flares and nudges.
        for variant in [StatusMotion::Flare, StatusMotion::Nudge] {
            assert!(duration(variant, Urgency::Idle, Urgency::Done).is_zero(), "{variant:?}");
            assert!(!duration(variant, Urgency::Idle, Urgency::NeedsYou).is_zero(), "{variant:?}");
        }
        // A terminal going quiet again is not washed.
        assert!(duration(StatusMotion::Wash, Urgency::Done, Urgency::Idle).is_zero());
        // No change is no motion.
        for &variant in StatusMotion::ALL {
            assert!(duration(variant, Urgency::Done, Urgency::Done).is_zero(), "{variant:?}");
        }
    }

    #[test]
    fn the_badge_waits_for_the_old_parts_then_springs_past_its_size_and_back() {
        let (idle, done) = (Urgency::Idle, Urgency::Done);
        let total = duration(StatusMotion::BadgePop, idle, done).as_secs_f32() * 1000.0;
        let at = |ms: f32| pose(StatusMotion::BadgePop, idle, done, ms / total);
        assert_eq!(at(40.0).badge.scale, 0.0, "nothing of it while the old parts leave");
        assert_eq!(at(40.0).leaving, 0.5);
        assert!(at(POP_OUT + 60.0).badge.scale > 0.3);
        let peak = (0..200).map(|n| at(POP_OUT + n as f32 * 3.0).badge.scale).fold(0.0, f32::max);
        assert!((1.1..1.3).contains(&peak), "the bouncy spring overshoots: {peak}");
        assert!((at(total - 1.0).badge.scale - 1.0).abs() < 0.01, "and it has settled");
    }

    #[test]
    fn the_pop_lasts_as_long_as_the_bouncy_spring_takes_to_settle() {
        let settle = SpringSpec::BOUNCY.settle(SETTLED).as_secs_f32() * 1000.0;
        assert!(
            (settle - POP_SPRING).abs() < 15.0,
            "the spring settles in {settle} ms, the pop takes {POP_SPRING}"
        );
    }

    #[test]
    fn the_wash_comes_up_quickly_and_fades_slowly() {
        let (idle, done) = (Urgency::Idle, Urgency::Done);
        let at = |ms: f32| pose(StatusMotion::Wash, idle, done, ms / 1020.0).wash;
        assert_eq!(at(0.0), 0.0);
        assert!(at(60.0) > 0.5 && at(120.0) > 0.999, "up in 120");
        assert!(at(570.0) > 0.4 && at(570.0) < 0.6, "half gone half way down");
        assert!(at(1000.0) < 0.03);
    }

    #[test]
    fn the_flare_sends_a_ring_out_that_fades() {
        let (working, asking) = (Urgency::Working, Urgency::NeedsYou);
        let at = |ms: f32| pose(StatusMotion::Flare, working, asking, ms / 700.0).flare;
        assert_eq!((at(0.0).reach, at(0.0).strength), (0.0, 0.7));
        let mid = at(200.0);
        assert!(mid.reach > 3.0 && mid.strength < 0.35, "{mid:?}: most of the way out, half gone");
        assert!(at(699.0).reach > 7.9 && at(699.0).strength < 0.01);
    }

    #[test]
    fn the_nudge_goes_to_one_side_and_back_twice() {
        // The steps are the mock's: 4 pt at a fifth, home at 45 %, 3 pt at 65 %, home.
        assert_eq!(nudge_at(0.0), 0.0);
        assert_eq!(nudge_at(0.2), 4.0);
        assert!((nudge_at(0.325) - 2.0).abs() < 1e-4, "straight between steps");
        assert_eq!(nudge_at(0.45), 0.0);
        assert_eq!(nudge_at(0.65), 3.0);
        assert_eq!(nudge_at(1.0), 0.0);
        assert_eq!(NUDGE_STEPS.len(), 5);
        let (working, asking) = (Urgency::Working, Urgency::NeedsYou);
        let peaks = (0..100)
            .map(|n| pose(StatusMotion::Nudge, working, asking, n as f32 / 100.0).dx)
            .collect::<Vec<_>>();
        assert!(peaks.iter().all(|&dx| (0.0..=4.0 + 1e-4).contains(&dx)), "always to the one side");
        let turns = peaks.windows(3).filter(|w| w[1] > w[0] && w[1] > w[2]).count();
        assert_eq!(turns, 2, "twice");
    }

    #[test]
    fn the_line_rolls_while_the_parts_cross() {
        let (idle, working) = (Urgency::Idle, Urgency::Working);
        let half = pose(StatusMotion::LineRoll, idle, working, 0.5);
        assert!(half.roll > 0.5, "the roll eases out: more than half way at half the time: {}", half.roll);
        assert_eq!(half.leaving + half.ring, 1.0, "the parts cross");
        let parts_done = pose(StatusMotion::LineRoll, idle, working, 0.76);
        assert_eq!((parts_done.leaving, parts_done.ring), (0.0, 1.0), "their 120 are over at 75 % of 160");
        assert!(parts_done.roll < 1.0, "and the line goes on");
    }

    #[test]
    fn only_the_statuses_that_say_something_are_washed() {
        assert_eq!(wash_of(Urgency::NeedsYou), Some(Wash::Attention));
        assert_eq!(wash_of(Urgency::Bell), Some(Wash::Attention));
        assert_eq!(wash_of(Urgency::Done), Some(Wash::Done));
        assert_eq!(wash_of(Urgency::Failed), Some(Wash::Danger));
        assert_eq!(wash_of(Urgency::Working), Some(Wash::Accent));
        for quiet in [Urgency::Idle, Urgency::Running, Urgency::NewOutput] {
            assert_eq!(wash_of(quiet), None, "{quiet:?}");
        }
    }
}
