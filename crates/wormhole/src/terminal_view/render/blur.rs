//! The lines just above the viewport, drawn blurred under the floating title bar.
//!
//! The blur is the text stamped several times at small offsets, faintly. Every
//! stamp is a full glyph paint, so the taps are few: the centre and the four
//! diagonals, each a little stronger than the thirteen taps this replaced, so
//! the darkest point (all five overlapping) is about as dark as before:
//! 1 - (1 - 0.26)^5 ≈ 1 - (1 - 0.11)^13 ≈ 0.78.

use gpui_kit::Hsla;
use wormhole_core::alacritty_terminal::term::cell::{Cell, Flags};
use wormhole_core::alacritty_terminal::term::color::Colors;

use crate::palette::Palette;

/// Offsets (px) at which blurred text is stamped.
pub(super) const BLUR_TAPS: [(f32, f32); 5] =
    [(0.0, 0.0), (1.7, 1.7), (-1.7, 1.7), (1.7, -1.7), (-1.7, -1.7)];

/// Opacity of each stamp.
const BLUR_ALPHA: f32 = 0.26;

/// A row above the viewport as faint text and its color runs (byte lengths),
/// or `None` when there's nothing to draw. Trailing blanks are dropped: every
/// glyph is painted once per tap.
pub(super) fn blurred_row(
    cells: &[Cell],
    palette: &Palette,
    overrides: &Colors,
) -> Option<(String, Vec<(usize, Hsla)>)> {
    let mut text = String::new();
    let mut runs: Vec<(usize, Hsla)> = Vec::new();
    for cell in cells {
        if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
            continue;
        }
        let mut color = if cell.flags.contains(Flags::INVERSE) {
            palette.resolve(cell.bg, overrides)
        } else {
            palette.resolve(cell.fg, overrides)
        };
        color.a *= BLUR_ALPHA;
        let c = if cell.c == '\0' || cell.c == '\t' { ' ' } else { cell.c };
        text.push(c);
        match runs.last_mut() {
            Some((len, last)) if *last == color => *len += c.len_utf8(),
            _ => runs.push((c.len_utf8(), color)),
        }
    }
    let kept = text.trim_end_matches(' ').len();
    if kept == 0 {
        return None;
    }
    let mut dropped = text.len() - kept;
    text.truncate(kept);
    while dropped > 0 {
        let Some((len, _)) = runs.last_mut() else { break };
        let cut = dropped.min(*len);
        *len -= cut;
        dropped -= cut;
        if *len == 0 {
            runs.pop();
        }
    }
    Some((text, runs))
}

#[cfg(test)]
mod tests {
    use wormhole_core::alacritty_terminal::term::cell::Cell;
    use wormhole_core::alacritty_terminal::term::color::Colors;
    use wormhole_core::alacritty_terminal::vte::ansi::{Color, NamedColor};
    use wormhole_core::theme;

    use super::{BLUR_TAPS, blurred_row};
    use crate::palette::Palette;

    fn cells(text: &str) -> Vec<Cell> {
        text.chars().map(|c| Cell { c, ..Cell::default() }).collect()
    }

    #[test]
    fn few_taps() {
        assert!(BLUR_TAPS.len() <= 5);
        assert_eq!(BLUR_TAPS[0], (0.0, 0.0), "the centre stamp comes first");
    }

    #[test]
    fn trailing_blanks_are_not_stamped() {
        let palette = Palette::from_theme(&theme::builtin()[0]);
        let mut row = cells("  ab   ");
        row[3].fg = Color::Named(NamedColor::Red);
        for cell in &mut row[4..] {
            cell.fg = Color::Named(NamedColor::Blue);
        }
        let (text, runs) = blurred_row(&row, &palette, &Colors::default()).expect("text");
        assert_eq!(text, "  ab");
        let lens: Vec<usize> = runs.iter().map(|(len, _)| *len).collect();
        assert_eq!(lens, [3, 1]);
        assert!(runs.iter().all(|(_, color)| color.a < 0.3));
        assert_eq!(blurred_row(&cells("     "), &palette, &Colors::default()), None);
    }
}
