//! How the wormhole run hands over to the terminal welcome (`Wormhole welcome`, a program in the
//! first terminal) instead of to the intro window's page. Three hand-overs are on trial:
//!
//! - **Cursor**: Voyage's ship swoops in, turns into `>` and docks on the terminal cell where the
//!   program draws its `>_` mark; the text starts typing from there.
//! - **Portal**: no ship and no dock. The window comes out of the wormhole with the terminal already
//!   in it, and the text types as the light lifts away.
//! - **Logo**: the program draws a large block-art logo, and the ship docks on its chevron as it docks
//!   on the intro window's logo; the rest comes in below.
//!
//! All three are the Voyage run (the same funnel, jump, portal and arrival); they differ in whether
//! the ship flies, where it lands, and when the program is told to start (`text_starts`).
//!
//! ```text
//! cursor, logo:  0 ── jump ── cruise ── exit ── swoop ── dock ─ LAND ──────── lift ─ end
//!                                                                 ╰ the text starts
//! portal:        0 ── jump ── cruise ── exit ─ arrive ───────────────── LIFT ─ end
//!                                                                          ╰ the text starts
//! ```

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{Style, voyage};

/// What the animation hands the terminal over to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Handover {
    /// The ship docks on the mark's `>` cell. The first launch uses it.
    #[default]
    Cursor,
    /// No ship: the text types as the light lifts away.
    Portal,
    /// The ship docks on the chevron of a block-art logo.
    Logo,
}

impl Handover {
    pub const ALL: [Handover; 3] = [Handover::Cursor, Handover::Portal, Handover::Logo];

    /// Its name in devctl (`welcome cursor`) and in the program's arguments.
    pub fn key(self) -> &'static str {
        match self {
            Handover::Cursor => "cursor",
            Handover::Portal => "portal",
            Handover::Logo => "logo",
        }
    }

    /// The name on the menu and in the palette ("Play Welcome: Cursor").
    pub fn label(self) -> &'static str {
        match self {
            Handover::Cursor => "Cursor",
            Handover::Portal => "Portal",
            Handover::Logo => "Logo",
        }
    }

    pub fn parse(key: &str) -> Option<Handover> {
        Self::ALL.into_iter().find(|handover| handover.key() == key.trim())
    }

    /// The style that plays around the window: always the Voyage run.
    pub fn style(self) -> Style {
        Style::Voyage
    }

    /// Whether the ship flies. Portal is the same run without it.
    pub fn ship(self) -> bool {
        self != Handover::Portal
    }

    /// When the program's text should start, in seconds into the run: as the ship has docked, or, with
    /// no ship, as the light behind the window begins to lift away.
    pub fn text_starts(self) -> f32 {
        match self {
            Handover::Cursor | Handover::Logo => voyage::LAND,
            Handover::Portal => voyage::LIFT,
        }
    }

    /// How long the program waits before it draws, `elapsed` into a run that is playing. Nothing to
    /// wait for when no run plays (a window that is not shown, a test instance) or under Reduce Motion,
    /// where the glow stands in for the run and nothing flies.
    pub fn wait(self, playing: Option<Duration>, reduce_motion: bool) -> Duration {
        match playing {
            Some(elapsed) if !reduce_motion => {
                Duration::from_secs_f32(self.text_starts().max(0.0)).saturating_sub(elapsed)
            }
            _ => Duration::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hand_overs_round_trip_through_their_keys() {
        for handover in Handover::ALL {
            assert_eq!(Handover::parse(handover.key()), Some(handover));
            assert_eq!(
                serde_json::to_string(&handover).expect("serializes"),
                format!("\"{}\"", handover.key())
            );
        }
        assert_eq!(Handover::parse(" logo "), Some(Handover::Logo));
        assert_eq!(Handover::parse("nope"), None);
        assert_eq!(Handover::default(), Handover::Cursor, "the first launch uses the cursor");
    }

    #[test]
    fn all_three_play_the_voyage_run_and_only_portal_has_no_ship() {
        for handover in Handover::ALL {
            assert_eq!(handover.style(), Style::Voyage);
            assert_eq!(handover.ship(), handover != Handover::Portal, "{handover:?}");
            // The text never starts after the run is over.
            assert!(handover.text_starts() < handover.style().seconds());
        }
    }

    /// The ship docks, and the text starts as it has; with none, as the light lifts away.
    #[test]
    fn the_text_starts_when_the_ship_has_landed_or_the_light_lifts() {
        assert_eq!(Handover::Cursor.text_starts(), voyage::LAND);
        assert_eq!(Handover::Logo.text_starts(), voyage::LAND);
        assert_eq!(Handover::Portal.text_starts(), voyage::LIFT);
        assert!(Handover::Portal.text_starts() > Handover::Cursor.text_starts());
    }

    #[test]
    fn the_wait_counts_down_with_the_run_and_is_nothing_without_one() {
        let landed = Duration::from_secs_f32(voyage::LAND);
        assert_eq!(Handover::Cursor.wait(Some(Duration::ZERO), false), landed);
        let half = Handover::Cursor.wait(Some(landed / 2), false);
        assert!(half > Duration::ZERO && half < landed);
        assert_eq!(Handover::Cursor.wait(Some(landed), false), Duration::ZERO);
        assert_eq!(Handover::Cursor.wait(Some(landed * 3), false), Duration::ZERO, "late: no wait");
        assert_eq!(Handover::Logo.wait(None, false), Duration::ZERO, "no run plays");
        assert_eq!(Handover::Portal.wait(Some(Duration::ZERO), true), Duration::ZERO, "Reduce Motion");
    }
}
