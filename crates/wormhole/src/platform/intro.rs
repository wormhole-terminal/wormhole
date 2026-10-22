//! The intro's overlay: a window over the whole screen around an app window, drawn by a shader
//! every frame, with the app window scaled and faded as the style moves it. GPUI draws only inside
//! its windows and has no custom shaders, so this is the platform's. When it plays, how long, the
//! skip and the hand-over are the intro's (`crate::intro`); the styles and their timing core's.
//!
//! An [`Overlay`] is the intro on screen: making one starts it, [`Overlay::frame`] draws each
//! frame, and dropping it closes it and puts the window back. Where there is no overlay to make
//! (a system without one, or a window without a native view), `Overlay::new` answers `None` and
//! the intro doesn't play.

use wormhole_core::intro::{Start, Style, WindowMotion};

pub use super::os::intro::Overlay;

/// What an overlay is asked to play.
#[derive(Clone, Copy, Debug)]
pub struct Setup {
    pub style: Style,
    /// The theme's primary and info colours and its terminal magenta, as RGBA.
    pub colors: [[f32; 4]; 3],
    /// Above everything, menu bar and Dock included; when not (test instances kept behind other
    /// apps), just above the window.
    pub immersive: bool,
    /// Whether the window was showing as the intro began.
    pub start: Start,
    /// Whether the traffic lights fade away as the run begins (a replay over a showing window).
    pub lights: bool,
    /// Whether Voyage's ship flies: not in the terminal welcome's Portal hand-over.
    pub ship: bool,
}

/// One frame of the intro: where the run has got to, and what is asked of the window around it.
pub struct Frame {
    /// Progress across the run, 0 to 1, and the seconds since it began.
    pub t: f32,
    pub time: f32,
    /// How opaque the traffic lights are while the overlay fades them away, or `None` to leave them be.
    pub lights: Option<f32>,
    /// After a skip: how the window was, and how far it has settled back to rest (0 to 1), the
    /// overlay fading away with it.
    pub settling: Option<(WindowMotion, f32)>,
}

/// devctl's pictures of the intro, drawn off screen.
#[cfg(feature = "devctl")]
pub mod pictures {
    use std::path::Path;

    use anyhow::Result;
    use gpui_kit::Window;
    use wormhole_core::intro::Style;
    use wormhole_core::intro::voyage::Landing;

    use super::super::os::intro as os;

    /// `count` frames of `style` over a stand-in screen and window (`stand_in`: the desk, the
    /// window and its edge, as RGB), to `<dir>/intro-<style>-<n>.bmp`.
    pub fn frames(
        style: Style,
        count: usize,
        colors: [[f32; 4]; 3],
        stand_in: [[f32; 3]; 3],
        dir: &Path,
    ) -> Result<String> {
        os::frames(style, count, colors, stand_in, dir)
    }

    /// Voyage's ship at `landing` over `window`'s own picture at four moments of the landing, to
    /// `<dir>/<name>-<n>.png`.
    pub fn dock_frames(
        window: &Window,
        landing: Landing,
        colors: [[f32; 4]; 3],
        dir: &Path,
        name: &str,
    ) -> Result<String> {
        os::dock_frames(window, landing, colors, dir, name)
    }
}
