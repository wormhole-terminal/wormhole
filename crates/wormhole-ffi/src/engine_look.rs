//! The theme's colours as the engine answers programs with them (`terminal_view/mod.rs` `share_palette` in the GPUI
//! app): every palette entry a program may ask for (OSC 4 for 0–255, OSC 10/11/12 for the foreground, background
//! and cursor) and whether the theme is light or dark (`CSI ? 996 n`, and mode 2031's reports when it changes).
//! Swift tells it the colours it draws with whenever the theme changes; a terminal started later gets them in its
//! spawn options (`launch`), every live one at once. The engine keeps what a program set itself on top.
//!
//! A terminal's view may tell it the colours it is drawn in itself (`Terminal::set_colors`, as each GPUI terminal
//! view shares its own window's palette): a window of another profile than the one Wormhole is in draws that
//! profile's theme, and a program is answered with what it is drawn in. Such a terminal no longer hears the look
//! of all.

use std::sync::{Arc, Mutex, PoisonError, RwLock, Weak};

use wormhole_term::types::{COLOR_COUNT, ColorScheme, Rgb};

use crate::terminal::Terminal;

/// The colours and the scheme the terminals are told.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EngineLook {
    pub(crate) colors: Box<[Rgb; COLOR_COUNT]>,
    pub(crate) scheme: ColorScheme,
}

static LOOK: RwLock<Option<EngineLook>> = RwLock::new(None);
/// Every terminal made, to tell a change to the live ones.
static TERMINALS: Mutex<Vec<Weak<Terminal>>> = Mutex::new(Vec::new());

/// What the theme in force says, once Swift has told it.
pub(crate) fn current() -> Option<EngineLook> {
    LOOK.read().unwrap_or_else(PoisonError::into_inner).clone()
}

/// Terminal `terminal` hears of the theme's changes from now on (it started with the look of the moment).
pub(crate) fn follow(terminal: &Arc<Terminal>) {
    let mut terminals = TERMINALS.lock().unwrap_or_else(PoisonError::into_inner);
    terminals.retain(|known| known.strong_count() > 0);
    terminals.push(Arc::downgrade(terminal));
}

/// The colours the terminals are drawn with: `slots` is the palette as `0xRRGGBB`, 256 indexed entries then the
/// foreground, background and cursor (`COLOR_COUNT` of them; a shorter list leaves the rest black). The scheme is
/// light when the background is (by its luma, what a program choosing its colours cares about). Telling the same
/// colours again does nothing.
#[uniffi::export]
pub fn set_terminal_colors(slots: Vec<u32>) {
    let look = look_of(&slots);
    {
        let mut current = LOOK.write().unwrap_or_else(PoisonError::into_inner);
        if current.as_ref() == Some(&look) {
            return;
        }
        *current = Some(look.clone());
    }
    let live: Vec<Arc<Terminal>> =
        TERMINALS.lock().unwrap_or_else(PoisonError::into_inner).iter().filter_map(Weak::upgrade).collect();
    for terminal in live {
        terminal.tell(&look);
    }
}

#[uniffi::export]
impl Terminal {
    /// The colours this terminal is drawn with, `slots` as `set_terminal_colors` takes them: its programs are
    /// answered with these, and told (mode 2031) when they turn light or dark. From now on the look of all no
    /// longer reaches it: whoever draws it tells it every change.
    pub fn set_colors(&self, slots: Vec<u32>) {
        TERMINALS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|known| !std::ptr::eq(known.as_ptr(), self));
        self.tell(&look_of(&slots));
    }
}

impl Terminal {
    fn tell(&self, look: &EngineLook) {
        self.with_pty(|pty| {
            pty.set_default_colors(&look.colors);
            pty.set_color_scheme(look.scheme);
        });
    }
}

/// `slots` as the engine's table, with the scheme its background gives.
fn look_of(slots: &[u32]) -> EngineLook {
    let mut colors = Box::new([Rgb::default(); COLOR_COUNT]);
    for (slot, value) in colors.iter_mut().zip(slots) {
        *slot = Rgb { r: (value >> 16) as u8, g: (value >> 8) as u8, b: *value as u8 };
    }
    EngineLook { scheme: scheme_of(colors[COLOR_COUNT - 2]), colors }
}

/// Light or dark by the background's luma (Rec. 601 weights in thousandths; half of 255,000 is the middle), as the
/// GPUI app's `Palette::color_scheme`.
fn scheme_of(background: Rgb) -> ColorScheme {
    let luma = 299 * u32::from(background.r) + 587 * u32::from(background.g) + 114 * u32::from(background.b);
    if luma >= 127_500 { ColorScheme::Light } else { ColorScheme::Dark }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scheme_follows_the_backgrounds_luma() {
        assert_eq!(scheme_of(Rgb { r: 0x1d, g: 0x1f, b: 0x21 }), ColorScheme::Dark);
        assert_eq!(scheme_of(Rgb { r: 0xfa, g: 0xfa, b: 0xfa }), ColorScheme::Light);
        assert_eq!(
            scheme_of(Rgb { r: 128, g: 128, b: 128 }),
            ColorScheme::Light,
            "the middle counts as light"
        );
        assert_eq!(scheme_of(Rgb { r: 127, g: 127, b: 127 }), ColorScheme::Dark);
    }
}
