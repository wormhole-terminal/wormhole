//! JetBrains Mono, compiled into the app so the terminal has a good coding
//! font on every Mac (see `assets/fonts/JetBrainsMono/README.md`).

/// The bundled family: the default terminal font (and so the code font).
pub const FAMILY: &str = "JetBrains Mono";
pub const VERSION: &str = "2.304";

macro_rules! face {
    ($file:literal) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/JetBrainsMono/", $file))
    };
}

/// The faces the terminal draws with: regular, bold, italic, bold italic.
/// Other weights aren't used by terminal or code text, so they'd only add size.
pub const FACES: [&[u8]; 4] = [
    face!("JetBrainsMono-Regular.ttf"),
    face!("JetBrainsMono-Bold.ttf"),
    face!("JetBrainsMono-Italic.ttf"),
    face!("JetBrainsMono-BoldItalic.ttf"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::fonts::custom::parse_font;

    #[test]
    fn every_face_is_monospace_jetbrains_mono() {
        for bytes in FACES {
            let families = parse_font(bytes).expect("bundled face parses");
            assert_eq!(families.len(), 1);
            assert_eq!(families[0].name, FAMILY);
            assert!(families[0].monospace);
        }
    }
}
