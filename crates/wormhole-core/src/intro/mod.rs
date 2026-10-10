//! The intro as data: the wormhole effect that plays over the whole screen around a window the
//! first time Wormhole opens, and again from About. Seven styles are on trial, plus the still
//! glow that stands in for all of them under Reduce Motion. This holds what doesn't need AppKit or
//! Metal: how long each style runs, how the window itself moves, and how fast space streams past
//! in the FTL jump. The app's overlay window and shader draw it.

pub mod page;
pub mod voyage;
pub mod wormhole;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// A swirl opens, the window grows out of it, and the ring settles onto its edges.
    Emerge,
    /// The window opens as usual and a band of light swirls once around its frame.
    Vortex,
    /// Space around the window spirals inward and its last light becomes the window's edge.
    Collapse,
    /// A faster-than-light jump in the manner of Stargate Universe's Destiny, heading into the
    /// screen: a charge builds, the stars stretch into streaks through a plasma tunnel, and the
    /// window arrives at the drop out in a cloud that thins away.
    Ftl,
    /// A particle wormhole: glittering particles gather at the screen's centre into a funnel, the
    /// jump flies into it among streaking stars, it stops dead with a spring back, and the portal
    /// opens onto the window, in front of aurora curtains that flow out to the screen's edges.
    Aurora,
    /// The same wormhole run as Aurora, but what blooms behind the window is tentacles of light:
    /// they reach out from behind it to the screen's edges, writhing, and draw back in at the end.
    Tentacles,
    /// The same wormhole run with a ship flown through it: the app icon's `>` turned to a `^`, big in
    /// front of the camera, which takes the jump, swoops across the window when the portal opens and
    /// docks on the logo of the intro window's page, the page coming in from there.
    Voyage,
    /// Reduce Motion: the edges glow and fade; nothing moves.
    Glow,
}

const ALL: [Style; 8] = [
    Style::Emerge,
    Style::Vortex,
    Style::Collapse,
    Style::Ftl,
    Style::Aurora,
    Style::Tentacles,
    Style::Voyage,
    Style::Glow,
];

impl Style {
    /// The styles that move, in the order About offers them.
    pub const ON_TRIAL: [Style; 7] = [
        Style::Emerge,
        Style::Vortex,
        Style::Collapse,
        Style::Ftl,
        Style::Aurora,
        Style::Tentacles,
        Style::Voyage,
    ];

    /// Its name in devctl.
    pub fn key(self) -> &'static str {
        match self {
            Style::Emerge => "emerge",
            Style::Vortex => "vortex",
            Style::Collapse => "collapse",
            Style::Ftl => "ftl",
            Style::Aurora => "aurora",
            Style::Tentacles => "tentacles",
            Style::Voyage => "voyage",
            Style::Glow => "glow",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Style::Emerge => "Emerge",
            Style::Vortex => "Edge Vortex",
            Style::Collapse => "Collapse",
            Style::Ftl => "FTL Jump",
            Style::Aurora => "Aurora",
            Style::Tentacles => "Tentacles",
            Style::Voyage => "Voyage",
            Style::Glow => "Glow",
        }
    }

    pub fn parse(key: &str) -> Option<Style> {
        ALL.into_iter().find(|style| style.key() == key.trim())
    }

    /// What plays: under Reduce Motion, the glow whatever was asked.
    pub fn with_motion(self, reduce_motion: bool) -> Style {
        if reduce_motion { Style::Glow } else { self }
    }

    pub fn seconds(self) -> f32 {
        match self {
            Style::Emerge => 4.2,
            Style::Vortex => 3.4,
            Style::Collapse => 4.6,
            Style::Ftl => 7.0,
            Style::Aurora | Style::Tentacles => wormhole::SECONDS,
            Style::Voyage => voyage::SECONDS,
            Style::Glow => 1.2,
        }
    }

    /// How the wormhole styles lay out their run for the shader.
    pub fn run(self) -> Run {
        match self {
            // The ship docks, and the arms bloom from there, a beat after.
            Style::Voyage => Run {
                seconds: voyage::SECONDS,
                lift: voyage::LIFT,
                bloom: voyage::LAND - wormhole::EXIT - 0.1,
                arms: 2.0,
                mist: 1.0,
            },
            _ => Run { seconds: wormhole::SECONDS, lift: wormhole::LIFT, bloom: 0.1, arms: 1.0, mist: 0.0 },
        }
    }

    /// The shader's number for it (`intro.metal`'s `switch`).
    pub fn shader_id(self) -> u32 {
        match self {
            Style::Emerge => 0,
            Style::Vortex => 1,
            Style::Collapse => 2,
            Style::Glow => 3,
            Style::Ftl => 4,
            Style::Aurora => 5,
            Style::Tentacles => 6,
            Style::Voyage => 7,
        }
    }
}

/// The window's own scale (about its centre) and opacity at `t`, from 0 to 1 across the intro.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowMotion {
    pub scale: f32,
    pub alpha: f32,
}

impl WindowMotion {
    pub const AT_REST: WindowMotion = WindowMotion { scale: 1.0, alpha: 1.0 };
}

/// How a wormhole style's run is laid out, in seconds: how long it is, when the light behind the window
/// begins to rise away and fade, when it begins to bloom after the drop out, and how big the arms of
/// light that bloom are (1 is as Tentacles has them), and how misty it all is: 1 swaps the funnel's fine
/// glitter for the smooth plasma tunnel of the FTL jump and blurs the arms into mist, 0 leaves both as
/// Aurora and Tentacles have them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Run {
    pub seconds: f32,
    pub lift: f32,
    pub bloom: f32,
    pub arms: f32,
    pub mist: f32,
}

/// Whether the window is on screen when the intro begins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Start {
    /// A replay: the window is there, and leaves as the intro starts.
    #[default]
    Shown,
    /// A first launch: the window was never shown, so it stays out of sight until it arrives.
    Hidden,
}

/// How the window moves while the intro plays, from a window that is showing. Vortex and Glow leave it
/// be.
pub fn window_motion(style: Style, t: f32) -> WindowMotion {
    window_motion_from(style, t, Start::Shown)
}

/// [`window_motion`] from a window that is `start`ing shown or hidden.
pub fn window_motion_from(style: Style, t: f32, start: Start) -> WindowMotion {
    let t = t.clamp(0.0, 1.0);
    match style {
        // Nothing while the screen sinks into space and the throat opens, then the window comes
        // through it.
        Style::Emerge => {
            let grow = phase(t, 0.36, 0.74);
            WindowMotion { scale: 0.08 + 0.92 * ease_out_back(grow), alpha: smoothstep(0.0, 0.45, grow) }
        }
        // Appears as the swirl closes in, settling from a touch larger.
        Style::Collapse => {
            let settle = phase(t, 0.55, 0.85);
            WindowMotion { scale: 1.05 - 0.05 * ease_out_cubic(settle), alpha: smoothstep(0.0, 1.0, settle) }
        }
        // Gone before the jump, and there when it arrives: settling from a little smaller as the
        // drop out's flash fades.
        Style::Ftl => {
            let arrive = phase(t, FTL_EXIT - 0.01, FTL_EXIT + 0.09);
            let leave = if start == Start::Shown { 1.0 - smoothstep(0.0, 0.06, t) } else { 0.0 };
            WindowMotion {
                scale: 0.93 + 0.07 * ease_out_cubic(arrive),
                alpha: leave.max(smoothstep(0.0, 1.0, arrive)),
            }
        }
        Style::Aurora | Style::Tentacles | Style::Voyage => {
            wormhole::window_motion(t * style.seconds(), start)
        }
        Style::Vortex | Style::Glow => WindowMotion::AT_REST,
    }
}

/// When the FTL jump goes to light speed, and when it drops out, as fractions of its run.
pub const FTL_JUMP: f32 = 0.22;
pub const FTL_EXIT: f32 = 0.76;

/// How fast the style's flow goes at `t` (0 to 1 across the run): the funnel's filaments in
/// Aurora and Tentacles ([`wormhole::speed`]), space past the window in the FTL jump ([`ftl_speed`]);
/// 0 for the styles without one.
pub fn speed(style: Style, t: f32) -> f32 {
    match style {
        Style::Ftl => ftl_speed(t),
        Style::Aurora | Style::Tentacles | Style::Voyage => wormhole::speed(t * style.seconds()),
        _ => 0.0,
    }
}

/// How far the style's flow has gone by `t`: the integral of [`speed`] over the run's seconds.
pub fn travel(style: Style, t: f32) -> f32 {
    const STEPS: usize = 240;
    let t = t.clamp(0.0, 1.0);
    let dt = t / STEPS as f32;
    (0..STEPS).map(|i| speed(style, (i as f32 + 0.5) * dt)).sum::<f32>() * dt * style.seconds()
}

/// How fast space streams past the window in the FTL jump at `t`, in points a second: a slow
/// drift, a hard push to light speed at the jump, a cruise, and the sudden drop out.
pub fn ftl_speed(t: f32) -> f32 {
    const DRIFT: f32 = 14.0;
    const CRUISE: f32 = 5200.0;
    let up = smoothstep(FTL_JUMP - 0.04, FTL_JUMP + 0.02, t);
    let down = smoothstep(FTL_EXIT - 0.015, FTL_EXIT + 0.03, t);
    DRIFT + (CRUISE - DRIFT) * up * (1.0 - down)
}

/// The moments the shader needs, as the `constant float` lines it is built on: core owns them, so
/// the shader has no copy that could drift. Fractions of the run for the FTL jump, seconds for the
/// wormhole run (a style's length and lift come with each frame, `Style::run`).
pub fn shader_constants() -> String {
    let constants = [
        ("FTL_JUMP", FTL_JUMP),
        ("FTL_EXIT", FTL_EXIT),
        ("WORMHOLE_GATHER", wormhole::GATHER),
        ("WORMHOLE_JUMP", wormhole::JUMP),
        ("WORMHOLE_EXIT", wormhole::EXIT),
        ("WORMHOLE_ARRIVE", wormhole::ARRIVE),
        // The ship and where it docks; its drawing is in the icon's canvas units.
        ("VOYAGE_LAND", voyage::LAND),
        ("VOYAGE_DISSOLVE", voyage::DISSOLVE),
        ("VOYAGE_CANVAS", voyage::ICON_CANVAS),
        ("VOYAGE_STROKE", voyage::ICON_STROKE),
        ("VOYAGE_APEX_Y", voyage::APEX_Y),
        ("VOYAGE_ARM_X", voyage::ARM_X),
        ("VOYAGE_ARM_Y", voyage::ARM_Y),
        ("VOYAGE_AURORA", if voyage::AURORA { 1.0 } else { 0.0 }),
    ];
    constants.iter().map(|(name, value)| format!("constant float {name} = {value:?};\n")).collect()
}

/// `t` mapped so `from` is 0 and `to` is 1, clamped.
fn phase(t: f32, from: f32, to: f32) -> f32 {
    ((t - from) / (to - from)).clamp(0.0, 1.0)
}

fn smoothstep(from: f32, to: f32, t: f32) -> f32 {
    let x = phase(t, from, to);
    x * x * (3.0 - 2.0 * x)
}

fn ease_out_cubic(x: f32) -> f32 {
    1.0 - (1.0 - x).powi(3)
}

/// Overshoots a little before settling: the window lands rather than stops.
fn ease_out_back(x: f32) -> f32 {
    const C1: f32 = 1.2;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (x - 1.0).powi(3) + C1 * (x - 1.0).powi(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_style_ends_with_the_window_at_rest() {
        for style in ALL {
            let end = window_motion(style, 1.0);
            assert!((end.scale - 1.0).abs() < 1e-4, "{style:?} ends at scale {}", end.scale);
            assert_eq!(end.alpha, 1.0, "{style:?}");
        }
    }

    #[test]
    fn emerge_starts_hidden_and_grows() {
        let start = window_motion(Style::Emerge, 0.0);
        assert_eq!(start.alpha, 0.0);
        assert!(start.scale < 0.1);
        let mid = window_motion(Style::Emerge, 0.4);
        assert!(mid.scale > start.scale && mid.alpha > 0.0);
    }

    #[test]
    fn the_ftl_jump_cruises_fast_and_drifts_either_side() {
        assert!(ftl_speed(0.0) < 50.0);
        assert!(ftl_speed(0.5) > 5000.0);
        assert!(ftl_speed(1.0) < 50.0);
        let (early, late) = (travel(Style::Ftl, 0.4), travel(Style::Ftl, 0.6));
        assert!(late > early && early > 0.0);
        assert_eq!(travel(Style::Ftl, 0.0), 0.0);
    }

    #[test]
    fn the_shader_gets_every_moment_by_name() {
        let constants = shader_constants();
        for name in [
            "FTL_JUMP",
            "FTL_EXIT",
            "WORMHOLE_JUMP",
            "WORMHOLE_EXIT",
            "WORMHOLE_ARRIVE",
            "VOYAGE_LAND",
            "VOYAGE_AURORA",
        ] {
            assert!(constants.contains(&format!("constant float {name} = ")), "{name}");
        }
        assert!(constants.contains("WORMHOLE_EXIT = 4.6;"), "{constants}");
    }

    #[test]
    fn tentacles_share_auroras_run() {
        let (aurora, tentacles) = (Style::Aurora, Style::Tentacles);
        assert_eq!(aurora.seconds(), tentacles.seconds());
        for t in [0.0, 0.2, 0.55, 0.8, 1.0] {
            assert_eq!(window_motion(aurora, t), window_motion(tentacles, t), "{t}");
            assert_eq!(speed(aurora, t), speed(tentacles, t), "{t}");
        }
        assert_ne!(aurora.shader_id(), tentacles.shader_id());
    }

    #[test]
    fn reduce_motion_always_glows() {
        for style in Style::ON_TRIAL {
            assert_eq!(style.with_motion(true), Style::Glow);
            assert_eq!(style.with_motion(false), style);
        }
    }

    #[test]
    fn styles_round_trip_through_their_keys() {
        for style in ALL {
            assert_eq!(Style::parse(style.key()), Some(style));
        }
        assert_eq!(Style::parse("nope"), None);
    }
}
