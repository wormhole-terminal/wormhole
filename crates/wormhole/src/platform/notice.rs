//! The notice chip's panel: a small capsule of the system's own glass that drops from under a window's
//! title row to say "Copied 3 lines". GPUI draws only inside its window and has no system material, so the
//! chip is a window of its own, a child of the app window's (it moves, minimises and enters full screen with
//! it), that ignores the mouse and never takes the keys. Core Animation plays the drop, so nothing is drawn
//! per frame.
//!
//! What it says, when, and for how long is the app's (`crate::notice`, with core's `view::notice` rules);
//! this only shows what it is handed. A [`Chip`] is one window's panel: making one has it appear,
//! [`Chip::hide`] sends it away, and dropping it closes the panel. Where there is no panel to make (a
//! system without one, a window without a native view, a test) [`Chip::new`] answers `None`.

use wormhole_core::theme::tokens::Appearance;
use wormhole_core::view::notice::Glyph;

pub use super::os::notice::Chip;

/// What the chip shows.
#[derive(Clone, Copy, Debug)]
pub struct Content<'a> {
    pub glyph: Glyph,
    pub text: &'a str,
    /// The theme's appearance, which the chip is drawn in whatever the system's is.
    pub appearance: Appearance,
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
