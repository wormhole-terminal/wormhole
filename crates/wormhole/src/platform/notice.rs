//! The notice chip's panel: a small capsule of the system's own glass that drops from the top edge of a
//! terminal to say "Copied 3 lines". GPUI draws only inside its window and has no system material, so the
//! chip is a window of its own, a child of the app window's (it moves, minimises and enters full screen with
//! it) that ignores the mouse, except over its capsule while a sticky message waits for a click, and never
//! takes the keys. Core Animation plays the drop, so nothing is drawn per frame.
//!
//! What it says, when, and for how long is the app's (`crate::notice`, with core's `view::notice` rules);
//! this only shows what it is handed, and says when it is clicked. A [`Chip`] is one window's panel: making
//! one has it appear, [`Chip::hide`] sends it away, and dropping it closes the panel. Where there is no panel
//! to make (a system without one, a window without a native view, a test) [`Chip::new`] answers `None`.

use wormhole_core::theme::tokens::Appearance;
use wormhole_core::view::notice::{Buttons, Glyph, Press};

pub use super::os::notice::Chip;

/// Where a click on a clickable chip goes: the app reads it (`crate::notice`) and decides what it means.
/// A channel and not a callback, because AppKit calls from its event loop, outside any GPUI update.
pub type Clicks = smol::channel::Sender<Click>;

/// A click on the chip: where on it, and when it began (a long press is told by its start).
#[derive(Clone, Copy, Debug)]
pub struct Click {
    pub press: Press,
    pub down: std::time::Instant,
}

/// What the chip shows.
#[derive(Clone, Copy, Debug)]
pub struct Content<'a> {
    pub glyph: Glyph,
    pub text: &'a str,
    /// The theme's appearance, which the chip is drawn in whatever the system's is.
    pub appearance: Appearance,
    /// The buttons beside the words, and what the platform calls them (VoiceOver, the tooltip).
    pub buttons: Buttons,
    pub close_label: &'a str,
    pub info_label: &'a str,
}

/// What the chip is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    /// Liquid Glass (macOS 26 and later).
    Glass,
    /// The frosted HUD material of the systems before it.
    Frosted,
    /// A plain fill: Reduce Transparency or Increase Contrast.
    Solid,
}

#[cfg(feature = "devctl")]
impl Material {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Glass => "glass",
            Self::Frosted => "frosted",
            Self::Solid => "solid",
        }
    }
}

/// How the chip stands, for devctl's `notices`.
#[cfg(feature = "devctl")]
#[derive(Clone, Copy, Debug)]
pub struct Info {
    pub material: Material,
    /// The chip at rest, in the window's own space (points from its top left): x, y, width, height.
    pub frame: [f32; 4],
    /// The panel takes the mouse (only while the chip is clickable); otherwise every click passes through it.
    pub takes_mouse: bool,
    /// The panel's window number, for a capture of that one window (`screencapture -l`).
    pub window_number: i64,
    /// The app window's, for a capture of that one.
    pub parent_number: i64,
}

/// devctl's `notice-material`: make the chip of this material from now on (`None`: as the system
/// allows), to see all three on one Mac.
#[cfg(feature = "devctl")]
pub fn force_material(material: Option<Material>) {
    super::os::notice::force_material(material);
}
