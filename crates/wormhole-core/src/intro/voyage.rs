//! Voyage: the wormhole run with a ship flown through it. The ship is the `>` of the app icon's `>_`
//! turned to point up as a `^`, big in front of the camera: it rises out of the bottom of the screen
//! into the funnel, takes the jump among the streaking stars, and when the portal opens it swoops
//! across the window, shrinking into the distance, and turns to `>` to dock on the logo of the intro
//! window's page, where the icon's own chevron is. The page comes in from there.
//!
//! ```text
//! 0 ─ launch ─ jump ──────────── cruise ───────── exit ─ swoop ─ dock ─ land ─ page ─ lift ─ end
//!     rises     flash, dive      bank and sway      portal  across    settle  docks  enters   rises
//! ```
//!
//! The ship's pose is core's (it is data, and tested); the shader draws what [`ship`] says, and the
//! moments are the wormhole's plus the few below.

use std::f32::consts::FRAC_PI_2;

use super::{ease_out_cubic, phase, smoothstep, wormhole};

/// Whether the aurora arms bloom behind the window once the ship has docked. Off for now: with the
/// ship landing on the logo it was too much. The arms are all still there (Tentacles shows them); this
/// is the one switch to bring them back.
pub const AURORA: bool = false;

/// The whole run: the wormhole's, with the time the landing and the page's coming in need. Without
/// the aurora there is nothing to wait for once the page is in, so the run ends soon after.
pub const SECONDS: f32 = if AURORA { 10.0 } else { 8.2 };
/// When the ship leaves the cruise for its swoop: as the portal opens.
pub const SWOOP: f32 = wormhole::EXIT + 0.1;
/// When it has reached the logo and settles onto it.
pub const DOCK: f32 = LAND - 0.4;
/// When it has docked: the icon's chevron is under it, and the rest of the icon fades in.
pub const LAND: f32 = 6.2;
/// How long it takes to dissolve into the logo once docked.
pub const DISSOLVE: f32 = 0.35;
/// When the light behind the window begins to rise away and fade.
pub const LIFT: f32 = if AURORA { 8.7 } else { 7.2 };

const _: () = assert!(wormhole::ARRIVE < SWOOP + 0.2 && SWOOP + 0.9 < DOCK && DOCK < LAND);
const _: () = assert!(LAND + DISSOLVE + 0.5 < LIFT && SECONDS - LIFT >= 1.0);

/// The app icon's canvas, in its own units (`packaging/icon/gen-icon.py`).
pub const ICON_CANVAS: f32 = 1024.0;
/// The icon's `>` as gen-icon.py draws it: a round-capped polyline through these three points, with
/// the cursor's width, on the icon's canvas. A test reads the generator to keep these the same.
pub const ICON_CHEVRON: [[f32; 2]; 3] = [[214.0, 214.0], [289.0, 289.0], [214.0, 364.0]];
pub const ICON_STROKE: f32 = 46.0;

/// The middle of the chevron's own box on the icon's canvas: the ship's position is this point.
pub const CHEVRON_CENTRE: [f32; 2] = [251.5, 289.0];

/// The chevron's own extent on the icon's canvas, its stroke included: width and height. The ship's
/// size is a canvas, so a target that is measured in its chevron's height (a terminal cell, a block-art
/// logo) needs this to size the ship. A test derives it from [`ICON_CHEVRON`].
pub const CHEVRON_EXTENT: [f32; 2] = [121.0, 196.0];

/// How much of the image's box the icon's canvas fills. `AppIcon.svg`, which the page draws, puts the
/// artwork on the macOS grid, a body 824 wide in a box of 1024, so the logo's box is bigger than the
/// canvas the chevron is placed on. A test reads the SVG to keep this the same.
pub const ICON_BODY: f32 = 824.0 / ICON_CANVAS;

/// The ship's drawing, in canvas units about [`CHEVRON_CENTRE`], turned to point up: the apex, and the
/// tail's two ends at ±`ARM_X`. (The icon's chevron points right; a quarter turn anticlockwise makes
/// `(u, v)` into `(v, -u)`.)
pub const APEX_Y: f32 = -37.5;
pub const ARM_X: f32 = 75.0;
pub const ARM_Y: f32 = 37.5;

/// Where the ship lands: the logo's box on screen, in points from the screen's top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landing {
    /// The middle of the logo's box.
    pub centre: [f32; 2],
    /// Its side.
    pub size: f32,
}

impl Landing {
    /// The landing that puts the ship's chevron with its middle at `chevron` and its height at
    /// `height` (points from the screen's top left): for a target that is not an image of the icon,
    /// like the terminal cell the welcome program draws its `>` in. The inverse of
    /// [`chevron`](Self::chevron) and [`canvas`](Self::canvas).
    pub fn on_chevron(chevron: [f32; 2], height: f32) -> Landing {
        let canvas = height * ICON_CANVAS / CHEVRON_EXTENT[1];
        let k = canvas / ICON_CANVAS;
        Landing {
            centre: [
                chevron[0] - (CHEVRON_CENTRE[0] - ICON_CANVAS / 2.0) * k,
                chevron[1] - (CHEVRON_CENTRE[1] - ICON_CANVAS / 2.0) * k,
            ],
            size: canvas / ICON_BODY,
        }
    }

    /// The side of the icon's canvas on screen: the artwork's box, inside the logo's. It is what the
    /// ship's size means, so a ship this big draws its chevron exactly as the logo does.
    pub fn canvas(&self) -> f32 {
        self.size * ICON_BODY
    }

    /// Where the icon's chevron is on screen: its box's middle, which sits left of the icon's.
    pub fn chevron(&self) -> [f32; 2] {
        let k = self.canvas() / ICON_CANVAS;
        [
            self.centre[0] + (CHEVRON_CENTRE[0] - ICON_CANVAS / 2.0) * k,
            self.centre[1] + (CHEVRON_CENTRE[1] - ICON_CANVAS / 2.0) * k,
        ]
    }
}

/// The ship at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ship {
    /// Where its chevron's middle is, points from the screen's top left.
    pub pos: [f32; 2],
    /// The side of the icon box it would be drawn in: at the dock, the logo's.
    pub size: f32,
    /// Radians clockwise from pointing up; a quarter turn (pointing right) is the icon's `>`.
    pub heading: f32,
    /// Rolled over onto a wing, radians.
    pub bank: f32,
    /// How hard the engines burn, 0 to 1.
    pub thrust: f32,
    /// 0 to 1: it dissolves into the logo as this goes.
    pub opacity: f32,
    /// A flash of the afterburners at the jump and the drop out, 0 to 1.
    pub flare: f32,
}

/// How big the ship is in front of the camera: the side of the icon box it would be drawn in, as a
/// fraction of the screen's height. The chevron is a fifth of that box's width, so this is several
/// screens; it was a box of 0.9 screens when the icon's chevron was twice as big in it.
const BIG: f32 = 2.05;
/// Where in the screen the ship hovers, rises to and cruises, as fractions of its height below the
/// funnel's centre.
const HOVER: f32 = 0.30;
const CRUISE: f32 = 0.26;
/// How the ship is smaller in the cruise than at the launch: it is further into the tunnel.
const CRUISE_SCALE: f32 = 0.72;
/// How long the launch takes.
const LAUNCH: f32 = 1.1;

/// The ship's place, size and heading before it banks: from the launch to the swoop's end.
fn raw(seconds: f32, screen: [f32; 2], landing: Landing) -> ([f32; 2], f32, f32) {
    let [w, h] = screen;
    let centre = [w / 2.0, h / 2.0];
    let s = seconds;

    // The cruise, which the swoop leaves from: up out of the bottom of the screen, a hover as the
    // funnel gathers, a dive into the tunnel at the jump, then sway and bob.
    let cruise = |s: f32| -> ([f32; 2], f32, f32) {
        let rise = ease_out_cubic(phase(s, 0.0, LAUNCH));
        let dive = smoothstep(wormhole::JUMP - 0.05, wormhole::JUMP + 0.8, s);
        let settled = smoothstep(wormhole::JUMP, wormhole::JUMP + 1.0, s);
        let hover_y = centre[1] + HOVER * h;
        let y = (h * 1.28) + (hover_y - h * 1.28) * rise;
        let y = y + ((centre[1] + CRUISE * h) - hover_y) * dive;
        let y = y + 0.012 * h * (1.7 * s).sin() * settled;
        let x = centre[0] + 0.035 * w * (1.1 * s + 0.4).sin() * settled;
        let kick = if s > wormhole::EXIT {
            let d = s - wormhole::EXIT;
            1.0 + 0.10 * (-5.0 * d).exp() * (12.0 * d).sin()
        } else {
            1.0
        };
        let size =
            BIG * h * (1.0 - (1.0 - CRUISE_SCALE) * dive) * (1.0 + 0.025 * (2.3 * s).sin() * settled) * kick;
        let heading = 0.07 * (1.1 * s + 0.4).cos() * settled;
        ([x, y], size, heading)
    };
    if s < SWOOP {
        return cruise(s);
    }

    // The swoop: a curve from where the cruise left off, veering up and to the left and then arcing
    // over to come in on the chevron of the logo from its left, as the icon's `>` points; shrinking as it
    // recedes.
    let (p0, size0, heading0) = cruise(SWOOP);
    let target = landing.chevron();
    let p1 = [p0[0] - 0.10 * w, p0[1] - 0.50 * (p0[1] - target[1])];
    let p2 = [target[0] - 0.24 * w, target[1] + 0.08 * h];
    let tau = smoothstep(0.0, 1.0, phase(s, SWOOP, DOCK));
    let bezier = |t: f32| -> [f32; 2] {
        let u = 1.0 - t;
        [
            u * u * u * p0[0] + 3.0 * u * u * t * p1[0] + 3.0 * u * t * t * p2[0] + t * t * t * target[0],
            u * u * u * p0[1] + 3.0 * u * u * t * p1[1] + 3.0 * u * t * t * p2[1] + t * t * t * target[1],
        ]
    };
    let pos = bezier(tau);
    // The nose comes round to point right, as the icon's `>` does, leaning into the veer on the way:
    // a ship yawing round steadily, not a head that follows every bend of the curve.
    let heading = heading0 + (FRAC_PI_2 - heading0) * smoothstep(0.05, 1.0, tau)
        - 0.5 * (std::f32::consts::PI * tau).sin();
    let shrink = smoothstep(0.0, 1.0, tau).powf(0.8);
    let size = (size0.ln() + (landing.canvas().ln() - size0.ln()) * shrink).exp();
    let touchdown = 1.0 + 0.12 * (-((s - LAND) / 0.08).powi(2)).exp();
    if s >= DOCK { (target, landing.canvas() * touchdown, FRAC_PI_2) } else { (pos, size, heading) }
}

/// The ship at `seconds` into the run, on a screen of `screen` points with the logo where `landing`
/// says.
pub fn ship(seconds: f32, screen: [f32; 2], landing: Landing) -> Ship {
    let (pos, size, heading) = raw(seconds, screen, landing);
    let s = seconds;

    // Rolled over onto a wing in the sway, and into the turns of the swoop.
    let sway = 0.18
        * (1.1 * s + 0.4).sin()
        * smoothstep(wormhole::JUMP, wormhole::JUMP + 1.0, s)
        * (1.0 - smoothstep(SWOOP, SWOOP + 0.3, s));
    let dt = 0.04;
    let rate = (raw(s + dt, screen, landing).2 - raw(s - dt, screen, landing).2) / (2.0 * dt);
    let turning = if (SWOOP..DOCK).contains(&s) { -0.5 * rate } else { 0.0 };
    let bank = (sway + turning).clamp(-0.6, 0.6);

    // The engines: the launch's burn, easing off as the funnel gathers, full in the cruise, cut at
    // the drop out, lit again for the swoop and out as it docks.
    let launch = 1.0 - 0.45 * smoothstep(0.6, wormhole::JUMP, s);
    let cruise = smoothstep(wormhole::JUMP - 0.05, wormhole::JUMP + 0.3, s);
    let cut = smoothstep(wormhole::EXIT - 0.15, wormhole::EXIT + 0.1, s);
    let swoop = smoothstep(SWOOP, SWOOP + 0.4, s);
    let docking = 1.0 - smoothstep(DOCK - 0.3, LAND, s);
    let thrust = if s < wormhole::EXIT {
        launch.max(cruise)
    } else {
        (0.25 * (1.0 - cut) + 0.25 * cut + 0.45 * swoop) * docking
    };
    let flare = (-((s - wormhole::JUMP) / 0.12).powi(2)).exp()
        + 0.35 * (-((s - wormhole::EXIT) / 0.15).powi(2)).exp();
    let opacity = 1.0 - smoothstep(LAND + 0.05, LAND + 0.05 + DISSOLVE, s);
    Ship { pos, size, heading, bank, thrust: thrust.clamp(0.0, 1.0), opacity, flare: flare.min(1.0) }
}

/// How many past positions the comet trail is drawn through, and how far apart in time.
pub const TRAIL_POINTS: usize = 24;
pub const TRAIL_STEP: f32 = 0.03;

/// Where the ship was at the last [`TRAIL_POINTS`] steps of [`TRAIL_STEP`] seconds, newest first.
pub fn trail(seconds: f32, screen: [f32; 2], landing: Landing) -> [[f32; 2]; TRAIL_POINTS] {
    std::array::from_fn(|k| ship(seconds - k as f32 * TRAIL_STEP, screen, landing).pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: [f32; 2] = [1440.0, 900.0];
    const LOGO: Landing = Landing { centre: [720.0, 330.0], size: 96.0 };

    fn at(seconds: f32) -> Ship {
        ship(seconds, SCREEN, LOGO)
    }

    #[test]
    fn the_ship_rises_into_view_from_below_the_screen() {
        let start = at(0.0);
        assert!(start.pos[1] > SCREEN[1], "starts below the screen: {:?}", start.pos);
        assert!(at(0.4).pos[1] < start.pos[1], "and rises");
        // Big: a good part of the screen's width, at its hover.
        let hover = at(1.1);
        let width = (2.0 * (ARM_X + ICON_STROKE / 2.0)) / ICON_CANVAS * hover.size;
        assert!(width > 0.2 * SCREEN[0], "a ship, not an icon: {width} wide");
        assert_eq!(hover.opacity, 1.0);
    }

    #[test]
    fn it_cruises_in_view_the_whole_jump() {
        let mut seconds = wormhole::JUMP + 0.4;
        while seconds < wormhole::EXIT {
            let ship = at(seconds);
            assert!(
                (0.0..SCREEN[0]).contains(&ship.pos[0]) && (0.0..SCREEN[1]).contains(&ship.pos[1]),
                "{seconds}: {:?}",
                ship.pos
            );
            assert!(ship.thrust > 0.9, "engines burn in the cruise: {}", ship.thrust);
            seconds += 0.1;
        }
        assert!(at(wormhole::JUMP).flare > 0.9, "an afterburner flash at the jump");
        assert!(at(wormhole::EXIT - 2.0).flare < 0.01);
    }

    #[test]
    fn it_docks_on_the_logos_chevron_pointing_right() {
        let target = LOGO.chevron();
        for seconds in [DOCK, (DOCK + LAND) / 2.0, LAND] {
            let ship = at(seconds);
            assert!(
                (ship.pos[0] - target[0]).abs() < 0.5 && (ship.pos[1] - target[1]).abs() < 0.5,
                "{seconds}: {:?} vs {target:?}",
                ship.pos
            );
            assert!((ship.heading - FRAC_PI_2).abs() < 0.01, "{seconds}: heading {}", ship.heading);
        }
        let docked = at(LAND);
        assert!(
            (docked.size / LOGO.canvas() - 1.12).abs() < 0.01,
            "a touchdown pulse over the icon canvas's size"
        );
        assert_eq!(docked.opacity, 1.0);
        assert!(docked.thrust < 0.01, "engines out: {}", docked.thrust);
        // The pulse is over, and the ship has become the logo.
        let after = at(LAND + 0.6);
        assert_eq!(after.opacity, 0.0);
        assert!((after.size - LOGO.canvas()).abs() < 0.5);
    }

    /// The page draws `AppIcon.svg` in the logo's box: its artwork sits on the macOS grid, so the chevron
    /// is where the canvas says only after the canvas is scaled to the body and centred in the box.
    #[test]
    fn the_logo_box_holds_the_icon_canvas_on_the_macos_grid() {
        let svg = include_str!("../../../../packaging/icon/AppIcon.svg");
        let inset = (ICON_CANVAS - 824.0) / 2.0;
        let transform = format!("translate({inset} {inset}) scale({ICON_BODY:.6})");
        assert!(svg.contains(&transform), "AppIcon.svg no longer places its artwork with {transform}");
        // The canvas's middle stays the box's middle, and the chevron lands inside the box.
        let chevron = LOGO.chevron();
        let k = LOGO.size / ICON_CANVAS;
        let expect = [
            LOGO.centre[0] + (inset + ICON_BODY * CHEVRON_CENTRE[0] - ICON_CANVAS / 2.0) * k,
            LOGO.centre[1] + (inset + ICON_BODY * CHEVRON_CENTRE[1] - ICON_CANVAS / 2.0) * k,
        ];
        assert!((chevron[0] - expect[0]).abs() < 1e-3 && (chevron[1] - expect[1]).abs() < 1e-3);
    }

    /// A chevron measured in its own height lands where asked, at the size asked: what the terminal
    /// cell and the block-art logo need.
    #[test]
    fn a_landing_can_be_made_from_a_chevrons_middle_and_height() {
        let landing = Landing::on_chevron([300.0, 120.0], 14.0);
        let chevron = landing.chevron();
        assert!((chevron[0] - 300.0).abs() < 1e-3 && (chevron[1] - 120.0).abs() < 1e-3, "{chevron:?}");
        // The chevron is CHEVRON_EXTENT tall on a canvas, so its height on screen is that share.
        let tall = landing.canvas() * CHEVRON_EXTENT[1] / ICON_CANVAS;
        assert!((tall - 14.0).abs() < 1e-3, "{tall}");
        assert!(landing.size > landing.canvas(), "the logo's box is bigger than its canvas");
    }

    #[test]
    fn the_chevron_extent_is_the_icons_points_with_their_stroke() {
        let xs = ICON_CHEVRON.map(|p| p[0]);
        let ys = ICON_CHEVRON.map(|p| p[1]);
        let span = |v: [f32; 3]| {
            v.iter().cloned().fold(f32::MIN, f32::max) - v.iter().cloned().fold(f32::MAX, f32::min)
        };
        assert_eq!(CHEVRON_EXTENT, [span(xs) + ICON_STROKE, span(ys) + ICON_STROKE]);
    }

    #[test]
    fn the_chevron_is_where_the_icon_draws_it() {
        // The box's middle is the canvas's; the chevron's own box is left of it.
        assert!(LOGO.chevron()[1] < LOGO.centre[1], "a touch higher than the box's middle");
        assert!(LOGO.chevron()[0] < LOGO.centre[0], "and left of it");
        // The icon's three points, with the stroke, fill the box about `CHEVRON_CENTRE`.
        let xs = ICON_CHEVRON.map(|p| p[0]);
        let ys = ICON_CHEVRON.map(|p| p[1]);
        let half = ICON_STROKE / 2.0;
        let (min_x, max_x) = (
            xs.iter().cloned().fold(f32::MAX, f32::min) - half,
            xs.iter().cloned().fold(f32::MIN, f32::max) + half,
        );
        let (min_y, max_y) = (
            ys.iter().cloned().fold(f32::MAX, f32::min) - half,
            ys.iter().cloned().fold(f32::MIN, f32::max) + half,
        );
        assert_eq!([(min_x + max_x) / 2.0, (min_y + max_y) / 2.0], CHEVRON_CENTRE);
        // Turned to point up: the apex above, the tail's ends below to either side.
        let [a, b, c] = ICON_CHEVRON;
        let turned = |p: [f32; 2]| [p[1] - CHEVRON_CENTRE[1], -(p[0] - CHEVRON_CENTRE[0])];
        assert_eq!(turned(b), [0.0, APEX_Y]);
        assert_eq!(turned(a), [-ARM_X, ARM_Y]);
        assert_eq!(turned(c), [ARM_X, ARM_Y]);
    }

    /// The icon generator is the source of the chevron: if it changes, the ship has to follow.
    #[test]
    fn the_icon_generator_draws_the_same_chevron() {
        let source = include_str!("../../../../packaging/icon/gen-icon.py");
        let line = source.lines().find(|l| l.starts_with("CHEVRON = ")).expect("gen-icon.py has a CHEVRON");
        let numbers: Vec<f32> = line
            .split(|c: char| !c.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .map(|part| part.parse().expect("a number"))
            .collect();
        let ours: Vec<f32> = ICON_CHEVRON.iter().flatten().copied().collect();
        assert_eq!(
            numbers, ours,
            "gen-icon.py's CHEVRON changed: update voyage::ICON_CHEVRON and CHEVRON_CENTRE"
        );
        let stroke = source.lines().find(|l| l.starts_with("STROKE = ")).expect("a STROKE");
        assert_eq!(
            stroke.trim_start_matches("STROKE = ").trim().parse::<f32>().expect("a number"),
            ICON_STROKE
        );
    }

    #[test]
    fn it_flies_smoothly_from_start_to_finish() {
        let frame = 1.0 / 60.0;
        let mut seconds = 0.0;
        let mut last = at(0.0);
        while seconds < LAND + 0.5 {
            seconds += frame;
            let now = at(seconds);
            let step = ((now.pos[0] - last.pos[0]).powi(2) + (now.pos[1] - last.pos[1]).powi(2)).sqrt();
            assert!(step < 45.0, "{seconds:.2}s: it jumped {step} points in a frame");
            assert!(
                (now.heading - last.heading).abs() < 0.15,
                "{seconds:.2}s: it turned {} in a frame",
                (now.heading - last.heading).abs()
            );
            assert!(
                (now.size / last.size - 1.0).abs() < 0.12,
                "{seconds:.2}s: its size jumped {} to {}",
                last.size,
                now.size
            );
            last = now;
        }
    }

    #[test]
    fn the_trail_follows_where_it_was() {
        let trail = trail(5.2, SCREEN, LOGO);
        assert_eq!(trail[0], at(5.2).pos, "newest first");
        assert_eq!(trail[1], at(5.2 - TRAIL_STEP).pos);
        // Moving fast through the swoop: the points are spread out.
        assert!(trail[0] != trail[TRAIL_POINTS - 1]);
    }
}
