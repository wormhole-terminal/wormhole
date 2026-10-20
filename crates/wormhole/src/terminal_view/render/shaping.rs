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

/// Shape one word of a row, each glyph pinned to its cells.
pub(super) fn shape_word(
    segment: &Segment,
    text_system: &WindowTextSystem,
    base_font: &Font,
    font_size: Pixels,
    cell_width: Pixels,
    outlines: &mut Outlines,
) -> Vec<Shaped> {
    let runs: Vec<TextRun> =
        segment.runs.iter().map(|(len, style)| text_run(*len, style, base_font)).collect();
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
