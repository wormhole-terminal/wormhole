//! What the app needs from the system's fonts beyond GPUI's text system: whether a family is
//! monospace or symbolic (for the font pickers), and the user's font files made available to
//! this process. GPUI's `add_fonts` reads only a collection's first face and can't let go of a
//! font again, so it is only the way a port would start (`fallback`).

use std::path::Path;

use super::os::fonts as os;

/// What a family's default face is like.
pub struct Traits {
    pub monospace: bool,
    pub symbolic: bool,
}

/// Classify a family by its default face; `None` for a family the system doesn't know, or where
/// it can't be asked. Thread-safe: the pickers classify on the background executor.
pub fn classify(family: &str) -> Option<Traits> {
    os::classify(family)
}

/// The family of the face whose PostScript name is `postscript` (`JetBrainsMono-Regular` is in
/// `JetBrains Mono`), for a settings import: iTerm2 and Terminal.app store the face, Wormhole
/// sets the family. `None` for a name the system doesn't know, or where it can't be asked.
pub fn family_of_postscript(postscript: &str) -> Option<String> {
    os::family_of_postscript(postscript)
}

/// Make a font file's faces available to this process (or stop that), like an installed font but
/// without touching the system.
pub fn register(path: &Path, register: bool) -> anyhow::Result<()> {
    os::register(path, register)
}
