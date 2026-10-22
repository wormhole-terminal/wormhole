//! How the intro window's page plays into the animation: when each part of it enters once the
//! window is on screen, and how the traffic lights go. The page is what the window carries through
//! the wormhole, so it begins to enter just as the window arrives. The lights, which are the window's
//! chrome, are away for as long as the intro window is up: they fade out as a replay begins, and the
//! window brings them back when the intro is closed.
//!
//! ```text
//! 0 ──────────────── window arrives ─ logo ─ title ─ tagline ─ body ─ actions ─ corner ─── end
//!     nothing of the page shows       each rises and fades in, one after the other
//! ```

use super::{FTL_EXIT, Start, Style, ease_out_cubic, phase, smoothstep, voyage, wormhole};

/// How far each part of the page has entered, 0 to 1: they rise and fade in one after another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entrance {
    pub logo: f32,
    pub title: f32,
    pub tagline: f32,
    /// The highlights, or a step's card.
    pub body: f32,
    /// The buttons along the bottom.
    pub actions: f32,
    /// The replay button in the corner.
    pub corner: f32,
}

impl Entrance {
    /// Everything in: the page at rest.
    pub const DONE: Entrance =
        Entrance { logo: 1.0, title: 1.0, tagline: 1.0, body: 1.0, actions: 1.0, corner: 1.0 };

    /// Nothing in yet.
    pub const NONE: Entrance =
        Entrance { logo: 0.0, title: 0.0, tagline: 0.0, body: 0.0, actions: 0.0, corner: 0.0 };

    /// The least of the parts: 1 only when all of them are in.
    pub fn least(&self) -> f32 {
        [self.logo, self.title, self.tagline, self.body, self.actions, self.corner]
            .into_iter()
            .fold(1.0, f32::min)
    }
}

/// When the window is on screen, in seconds into `style`'s run.
pub fn window_arrives(style: Style) -> f32 {
    match style {
        Style::Emerge => 0.5 * style.seconds(),
        Style::Collapse => 0.6 * style.seconds(),
        Style::Ftl => FTL_EXIT * style.seconds(),
        Style::Aurora | Style::Tentacles | Style::Voyage => wormhole::ARRIVE,
        // Never gone.
        Style::Vortex | Style::Glow => 0.0,
    }
}

/// A beat after the window is there before the first part moves.
const LEAD: f32 = 0.1;
/// Each part's delay after the first, in the order of [`Entrance`]'s fields.
const DELAYS: [f32; 6] = [0.0, 0.12, 0.24, 0.42, 0.62, 0.85];
/// How long a part takes to come in.
const RISE: f32 = 0.6;

/// When the page's first part begins to come in, in seconds into the run: a beat after the window
/// is there, or, in the voyage, as the ship docks on the logo.
fn enters_from(style: Style) -> f32 {
    match style {
        Style::Voyage => voyage::LAND - 0.15,
        _ => window_arrives(style) + LEAD,
    }
}

/// By when, in seconds into the run, every part of the page is in. Under Reduce Motion (the glow)
/// nothing moves, so the page is simply there.
pub fn entered_by(style: Style) -> f32 {
    if style == Style::Glow {
        return 0.0;
    }
    enters_from(style) + DELAYS[DELAYS.len() - 1] + RISE
}

/// The page's parts at `seconds` into `style`'s run.
pub fn entrance(style: Style, seconds: f32) -> Entrance {
    if style == Style::Glow {
        return Entrance::DONE;
    }
    let from = enters_from(style);
    let part = |delay: f32| ease_out_cubic(phase(seconds, from + delay, from + delay + RISE));
    Entrance {
        logo: part(DELAYS[0]),
        title: part(DELAYS[1]),
        tagline: part(DELAYS[2]),
        body: part(DELAYS[3]),
        actions: part(DELAYS[4]),
        corner: part(DELAYS[5]),
    }
}

/// How long the traffic lights take to fade away as a replay begins.
const LIGHTS_AWAY: f32 = 0.25;

/// The traffic lights' opacity at `seconds` into a run. They fade away as a replay begins (or were
/// never there on a first launch) and stay away: they come back with the terminal, when the intro
/// window is closed, which the run doesn't know.
pub fn lights(seconds: f32, start: Start) -> f32 {
    match start {
        Start::Shown => 1.0 - smoothstep(0.0, LIGHTS_AWAY, seconds),
        Start::Hidden => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLES: [Style; 8] = [
        Style::Emerge,
        Style::Vortex,
        Style::Collapse,
        Style::Ftl,
        Style::Aurora,
        Style::Tentacles,
        Style::Voyage,
        Style::Glow,
    ];

    #[test]
    fn the_page_is_in_before_the_run_is_over() {
        for style in STYLES {
            assert!(entered_by(style) <= style.seconds(), "{style:?}: in by {}", entered_by(style));
            assert_eq!(entrance(style, entered_by(style)), Entrance::DONE, "{style:?}");
            assert_eq!(entrance(style, style.seconds()), Entrance::DONE, "{style:?}");
        }
    }

    #[test]
    fn nothing_enters_before_the_window_is_there() {
        for style in STYLES.into_iter().filter(|style| *style != Style::Glow) {
            assert_eq!(entrance(style, window_arrives(style)), Entrance::NONE, "{style:?}");
        }
        assert_eq!(entrance(Style::Glow, 0.0), Entrance::DONE, "Reduce Motion: the page is just there");
        // The window arrives at the drop out, in the middle of the run.
        assert!(window_arrives(Style::Tentacles) > 3.0);
        assert_eq!(window_arrives(Style::Tentacles), window_arrives(Style::Aurora));
    }

    #[test]
    fn the_parts_come_in_one_after_the_other() {
        let style = Style::Tentacles;
        let mid = window_arrives(style) + 0.5;
        let e = entrance(style, mid);
        assert!(e.logo > e.title && e.title > e.tagline && e.tagline > e.body, "{e:?}");
        assert!(e.body >= e.actions && e.actions >= e.corner, "{e:?}");
        assert!(e.least() < 1.0);
        assert_eq!(Entrance::DONE.least(), 1.0);
    }

    #[test]
    fn the_lights_go_and_stay_away() {
        // A replay: showing, then gone as it begins.
        assert_eq!(lights(0.0, Start::Shown), 1.0);
        let fading = lights(LIGHTS_AWAY / 2.0, Start::Shown);
        assert!(fading > 0.0 && fading < 1.0, "fading out: {fading}");
        // A first launch: never there. Either way they stay away for the rest of the run.
        assert_eq!(lights(0.0, Start::Hidden), 0.0);
        for start in [Start::Shown, Start::Hidden] {
            for seconds in [LIGHTS_AWAY, 1.0, wormhole::ARRIVE + 1.0, Style::Voyage.seconds(), 60.0] {
                assert_eq!(lights(seconds, start), 0.0, "{start:?} at {seconds}s");
            }
        }
    }
}
