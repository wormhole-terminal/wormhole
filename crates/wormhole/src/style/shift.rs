//! A terminal's status changing, as the places that show it are told: the tile (its light or edge and
//! its badge), the card (a wash), the row (a nudge) and the second line (which rolls). They are drawn
//! from the poses of core (`view::motion::pose::status`), which a [`StatusShift`] hands out: the list
//! plays one as an animation of the element it belongs to ([`ShiftClock::Playing`]), and a Settings
//! preview, which owns its clock, gives the time ([`ShiftClock::At`]). Either way what is drawn is a
//! function of the pose, so the preview shows what the list does.
//!
//! Nothing here knows when a change begins or ends: the sidebar's `motion::status` decides that, and
//! drops the shift when it is over, which draws the status's resting look.

use std::time::Duration;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::StatusMotion;
use wormhole_core::view::motion::pose::status::{self, StatusPose, Wash};
use wormhole_core::view::sidebar::Urgency;

use super::motion::ANIMATION_FPS;
use super::status::StatusGlyph;
use crate::settings::UiColors;

/// How a status change is being shown: by an animation of its own, or at a time the caller gives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShiftClock {
    /// An animation that runs the change's duration once, from the first frame it is drawn. The number
    /// tells this change from the one before it, so a second one runs from its start.
    Playing(u64),
    /// `t` (0.0..=1.0) of the way through the change.
    At(f32),
}

/// A status change: what the tile showed before, how the change is played and how far along it is. What it
/// changed to is what the tile is drawn with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusShift {
    pub motion: StatusMotion,
    pub from: StatusGlyph,
    pub clock: ShiftClock,
}

/// How strong a wash is at its fullest: a tint of the status's colour over the row.
const WASH_TINT: f32 = 0.16;

impl StatusShift {
    /// How long the change takes to `to`: none when it is a cut.
    #[must_use]
    pub fn duration(self, to: StatusGlyph) -> Duration {
        status::duration(self.motion, self.from.into(), to.into())
    }

    /// The pose of the tile and the row `t` of the way through the change to `to`.
    #[must_use]
    pub fn pose(self, to: StatusGlyph, t: f32) -> StatusPose {
        status::pose(self.motion, self.from.into(), to.into(), t)
    }

    /// Whether there is anything to show: another status, and a variant that has something to say of it.
    #[must_use]
    pub fn is_shown(self, to: StatusGlyph) -> bool {
        self.from != to && !self.duration(to).is_zero()
    }

    /// `element` with `apply` run on it for the pose of the moment, which is moving by an animation of
    /// its own (keyed by `id` and the change, so each change runs from its start) or fixed at the time the
    /// clock gives. The element is drawn again every frame the animation runs, nothing else is.
    #[track_caller]
    pub(crate) fn drive<E: IntoElement + 'static>(
        self,
        element: E,
        id: impl Into<ElementId>,
        to: StatusGlyph,
        apply: impl Fn(E, StatusPose) -> E + 'static,
    ) -> AnyElement {
        match self.clock {
            ShiftClock::At(t) => apply(element, self.pose(to, t)).into_any_element(),
            ShiftClock::Playing(change) => element
                .with_animation(
                    (id.into(), change.to_string()),
                    Animation::new(self.duration(to)).with_max_fps(ANIMATION_FPS),
                    move |element, t| apply(element, self.pose(to, t)),
                )
                .into_any_element(),
        }
    }

    /// The wash over a row that got into `to`, `radius` round: a tint of the status's colour that comes
    /// up and fades away. `None` for a variant that does not wash, and for a status that gets none.
    #[track_caller]
    pub(crate) fn wash(
        self,
        id: impl Into<ElementId>,
        ui: UiColors,
        to: StatusGlyph,
        radius: f32,
    ) -> Option<AnyElement> {
        let kind = status::wash_of(Urgency::from(to)).filter(|_| self.motion == StatusMotion::Wash)?;
        let color = match kind {
            Wash::Attention => ui.agent_attention,
            Wash::Done => ui.agent_done,
            Wash::Danger => ui.danger,
            Wash::Accent => ui.agent_working,
        };
        let layer = div().absolute().top_0().left_0().size_full().rounded(px(radius));
        Some(self.drive(layer, id, to, move |layer, pose| layer.bg(color.opacity(WASH_TINT * pose.wash))))
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::motion::StatusMotion;

    use super::{ShiftClock, StatusShift};
    use crate::style::StatusGlyph;

    fn shift(motion: StatusMotion, from: StatusGlyph) -> StatusShift {
        StatusShift { motion, from, clock: ShiftClock::At(0.5) }
    }

    #[test]
    fn a_change_is_shown_when_the_status_is_another_and_the_variant_has_something_for_it() {
        let working = StatusGlyph::Working;
        assert!(shift(StatusMotion::Crossfade, StatusGlyph::Idle).is_shown(working));
        assert!(!shift(StatusMotion::Classic, StatusGlyph::Idle).is_shown(working), "a cut");
        assert!(!shift(StatusMotion::Crossfade, working).is_shown(working), "no change");
        assert!(
            !shift(StatusMotion::Nudge, StatusGlyph::Idle).is_shown(working),
            "a nudge is for needs input"
        );
        assert!(shift(StatusMotion::Nudge, working).is_shown(StatusGlyph::NeedsYou));
    }

    #[test]
    fn the_glyphs_map_onto_core_urgencies_the_poses_take() {
        // A flare is for the status that asks for input, however the app names it.
        let flare = shift(StatusMotion::Flare, StatusGlyph::Working);
        assert!(flare.duration(StatusGlyph::NeedsYou) > std::time::Duration::ZERO);
        assert!(flare.duration(StatusGlyph::Done).is_zero());
    }
}
