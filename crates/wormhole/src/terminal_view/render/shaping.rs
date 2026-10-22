//! Handing text to GPUI's text system: what cached rows and words were shaped with, and the
//! shaping calls themselves (a word of a row, a faint row above the viewport).

use std::rc::Rc;

use gpui_kit::{
    Font, FontId, FontStyle, FontWeight, Hsla, Pixels, ShapedLine, StrikethroughStyle, TextRun,
    WindowTextSystem, px,
};
use wormhole_core::terminal::types::{COLOR_COUNT, Colors};

use super::outline::Outlines;
use super::rows::{Segment, Style};
use super::words::Shaped;
use crate::palette::Palette;

/// What every cached row was shaped with; any change rebuilds them all.
#[derive(Clone, PartialEq)]
pub(super) struct ShapeKey {
    /// Compared by value: a settings change that leaves the colors alone (or
    /// rebuilds an equal palette) keeps the rows.
    pub palette: Rc<Palette>,
    pub ligatures: bool,
    pub font: FontId,
    pub font_size: Pixels,
    pub cell_width: Pixels,
    /// The colors programs set (OSC 4/10/11): a cell keeps its content when its
    /// palette entry changes, so a row found again by content would be stale.
    pub colors: u64,
}

/// What a word's glyphs were shaped with: the font (its features carry the
/// ligatures), its size and the cell they are pinned to.
pub(super) type WordsKey = (FontId, Pixels, Pixels);

/// A fingerprint of the runtime color overrides.
pub(super) fn colors_key(colors: &Colors) -> u64 {
    (0..COLOR_COUNT).fold(0u64, |hash, index| {
        let value =
            colors[index].map_or(u64::MAX, |c| u64::from(c.r) << 16 | u64::from(c.g) << 8 | u64::from(c.b));
        (hash.rotate_left(5) ^ value).wrapping_mul(0x517c_c1b7_2722_0a95)
    })
}

/// GPUI's text run for a style.
pub(super) fn text_run(len: usize, style: &Style, base_font: &Font) -> TextRun {
    let mut font = base_font.clone();
    if style.bold {
        font.weight = FontWeight::BOLD;
    }
    if style.italic {
        font.style = FontStyle::Italic;
    }
    TextRun {
        len,
        font,
        color: style.fg,
        background_color: None,
        underline: None,
        strikethrough: style
            .strike
            .then_some(StrikethroughStyle { thickness: px(1.0), color: Some(style.fg) }),
    }
}

/// GPUI's text runs to shape a word with: one per stretch of the same font (bold, italic), whatever
/// the colors. GPUI starts a new CoreText run at every color change, in a font copy of its own, so a
/// word of a color per cell (`ttystorm flood --kind colorflood`, a gradient) was shaped one cell at a
/// time: 3 to 4 ms for a screen-wide one. The glyphs are painted in their cells' colors afterwards
/// anyway, and [`super::words`] caches words by text and fonts alone, so shaping them by text and
/// fonts alone is also what keeps a cached word the same in any colors: a ligature now forms across a
/// color change, painted in the color of its first cell, as it already did when the word was first
/// seen in one color.
pub(super) fn word_runs(segment: &Segment, base_font: &Font) -> Vec<TextRun> {
    let mut runs: Vec<TextRun> = Vec::new();
    let mut font: Option<(bool, bool)> = None;
    for (len, style) in &segment.runs {
        match runs.last_mut() {
            Some(run) if font == Some((style.bold, style.italic)) => run.len += len,
            _ => {
                font = Some((style.bold, style.italic));
                // Strikethroughs are painted by the terminal; one here would only split the run.
                let plain = Style { strike: false, ..*style };
                runs.push(text_run(*len, &plain, base_font));
            }
        }
    }
    runs
}

/// Shape one word of a row, each glyph pinned to its cells.
pub(super) fn shape_word(
    segment: &Segment,
    text_system: &WindowTextSystem,
    base_font: &Font,
    font_size: Pixels,
    cell_width: Pixels,
    outlines: &mut Outlines,
) -> Vec<Shaped> {
    let runs = word_runs(segment, base_font);
    let advance = if segment.wide { cell_width * 2.0 } else { cell_width };
    let line = text_system.shape_line(segment.text.clone().into(), font_size, &runs, Some(advance));
    // Which faces the glyphs are from is only known here, with the text and the runs at hand.
    outlines.learn(text_system, &segment.text, &runs, &line);
    line.runs
        .iter()
        .flat_map(|run| {
            run.glyphs.iter().map(|glyph| Shaped {
                position: glyph.position,
                index: glyph.index,
                font_id: run.font_id,
                id: glyph.id,
                emoji: glyph.is_emoji,
            })
        })
        .collect()
}

/// Shape a faint row above the viewport.
pub(super) fn shape_blurred(
    text: String,
    runs: Vec<(usize, Hsla)>,
    text_system: &WindowTextSystem,
    base_font: &Font,
    font_size: Pixels,
    cell_width: Pixels,
) -> ShapedLine {
    let runs: Vec<TextRun> = runs
        .into_iter()
        .map(|(len, color)| TextRun {
            len,
            font: base_font.clone(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        })
        .collect();
    text_system.shape_line(text.into(), font_size, &runs, Some(cell_width))
}

#[cfg(test)]
mod tests {
    use gpui_kit::{FontStyle, FontWeight, font};
    use wormhole_core::terminal::types::{Cell, Color, Colors, Flags, NamedColor, RowExtras};
    use wormhole_core::theme;

    use super::word_runs;
    use crate::palette::Palette;
    use crate::terminal_view::render::rows::{Marks, build_row};

    #[test]
    fn a_word_is_shaped_in_one_run_per_font_whatever_its_colors() {
        let colors = [NamedColor::Red, NamedColor::Green, NamedColor::Blue];
        let cells: Vec<Cell> = "abcdefg"
            .chars()
            .enumerate()
            .map(|(col, c)| Cell {
                c,
                fg: Color::Named(colors[col % 3]),
                flags: match col {
                    3 | 4 => Flags::BOLD,
                    5 => Flags::STRIKEOUT,
                    _ => Flags::empty(),
                },
                ..Cell::default()
            })
            .collect();
        let palette = Palette::from_theme(&theme::builtin()[0]);
        let row = build_row(&cells, &RowExtras::default(), &palette, &Colors::default(), &Marks::default());
        let [segment] = row.segments.as_slice() else { panic!("one word: {:?}", row.segments) };
        assert_eq!(segment.runs.len(), 7, "a run per cell: each has its own color");
        let runs = word_runs(segment, &font("JetBrains Mono"));
        let shape: Vec<(usize, FontWeight, FontStyle, bool)> = runs
            .iter()
            .map(|run| (run.len, run.font.weight, run.font.style, run.strikethrough.is_some()))
            .collect();
        let (normal, bold, upright) = (FontWeight::NORMAL, FontWeight::BOLD, FontStyle::Normal);
        assert_eq!(
            shape,
            [(3, normal, upright, false), (2, bold, upright, false), (2, normal, upright, false)]
        );
    }
}
