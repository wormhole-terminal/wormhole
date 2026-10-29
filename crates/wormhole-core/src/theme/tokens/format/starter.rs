//! The two palettes a new theme starts from when it is copied from nothing: a neutral light and a
//! neutral dark, as data. A theme made from one has no `base` and nothing under it (a version 1
//! file never falls back to Tokyo Night), so every terminal colour is set here.

use crate::theme::tokens::Appearance;
use crate::theme::tokens::file::{TerminalFile, ThemeFile};

use super::CURRENT_VERSION;

struct Palette {
    foreground: &'static str,
    background: &'static str,
    cursor: &'static str,
    selection: &'static str,
    ansi: [&'static str; 16],
}

const DARK: Palette = Palette {
    foreground: "#d4d4d4",
    background: "#1e1e1e",
    cursor: "#d4d4d4",
    selection: "#3a3d41",
    ansi: [
        "#3b3b3b", "#e5534b", "#57ab5a", "#c69026", "#539bf5", "#b083f0", "#39c5cf", "#cdd9e5", "#636e7b",
        "#ff938a", "#6bc46d", "#daaa3f", "#6cb6ff", "#dcbdfb", "#56d4dd", "#f0f6fc",
    ],
};

const LIGHT: Palette = Palette {
    foreground: "#24292f",
    background: "#ffffff",
    cursor: "#24292f",
    selection: "#b6d6fd",
    ansi: [
        "#24292f", "#cf222e", "#116329", "#4d2d00", "#0969da", "#8250df", "#1b7c83", "#6e7781", "#57606a",
        "#a40e26", "#1a7f37", "#633c01", "#218bff", "#a475f9", "#3192aa", "#8c959f",
    ],
};

/// A new version 1 theme with no base: the neutral palette of `appearance`, no name or id yet.
pub fn starter(appearance: Appearance) -> ThemeFile {
    let p = if appearance == Appearance::Light { &LIGHT } else { &DARK };
    let mut terminal = TerminalFile {
        foreground: Some(p.foreground.into()),
        background: Some(p.background.into()),
        cursor: Some(p.cursor.into()),
        selection_background: Some(p.selection.into()),
        ..TerminalFile::default()
    };
    for (i, color) in p.ansi.iter().enumerate() {
        terminal.ansi.insert(i.to_string(), (*color).into());
    }
    ThemeFile {
        version: Some(CURRENT_VERSION),
        appearance: Some(appearance),
        terminal,
        ..ThemeFile::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::tokens::{builtin_tokens, resolve};

    #[test]
    fn a_starter_sets_every_terminal_colour_and_resolves_with_nothing_under_it() {
        let tokyo = builtin_tokens().into_iter().find(|t| t.name == "Tokyo Night").expect("Tokyo Night");
        for appearance in [Appearance::Light, Appearance::Dark] {
            let file = starter(appearance);
            assert!(file.base.is_none() && file.extends.is_none());
            assert!(
                super::super::missing_terminal(&|key| match key.strip_prefix("ansi.") {
                    Some(i) => file.terminal.ansi.contains_key(i),
                    None => file
                        .terminal
                        .entries()
                        .is_ok_and(|e| e.iter().any(|(n, _)| n == &format!("terminal.{key}"))),
                })
                .is_empty()
            );
            let tokens = resolve(&file, &|_| None).expect("resolves on its own");
            assert_eq!(tokens.appearance, appearance);
            assert_ne!(tokens.terminal.background, tokyo.terminal.background, "not Tokyo Night");
            assert_ne!(tokens.terminal.ansi[4], tokyo.terminal.ansi[4]);
        }
    }

    #[test]
    fn a_starter_written_and_read_again_is_the_same_theme() {
        let file =
            ThemeFile { name: Some("Plain".into()), id: Some("plain".into()), ..starter(Appearance::Dark) };
        let again = crate::theme::tokens::read_theme_file(&file.to_json_pretty()).expect("reads");
        assert_eq!(resolve(&again, &|_| None), resolve(&file, &|_| None));
    }

    /// A palette a new theme starts from reads on its own background: the text 7:1, every colour
    /// of the palette but the first 3:1, and the interface's text colours meet their minimums.
    #[test]
    fn a_starter_is_readable() {
        use crate::theme::tokens::Color;
        for appearance in [Appearance::Light, Appearance::Dark] {
            let tokens = resolve(&starter(appearance), &|_| None).expect("resolves");
            let bg = tokens.terminal.background;
            let ratio = |c: Color| Color::contrast_ratio(c, bg);
            assert!(ratio(tokens.terminal.foreground) >= 7.0, "{appearance:?} text");
            assert!(ratio(tokens.terminal.cursor) >= 3.0, "{appearance:?} cursor");
            for (i, c) in tokens.terminal.ansi.iter().enumerate().skip(1) {
                assert!(ratio(*c) >= 3.0, "{appearance:?} ansi.{i} {}: {}", c.to_hex(), ratio(*c));
            }
            assert!(ratio(tokens.terminal.selection_background) >= 1.1, "{appearance:?} selection");
            let low: Vec<_> =
                tokens.contrast_warnings().into_iter().filter(|(fg, _, _)| fg.starts_with("text.")).collect();
            assert!(low.is_empty(), "{appearance:?}: {low:?}");
        }
    }
}
