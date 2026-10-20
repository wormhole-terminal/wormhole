//! Each word of a row shaped once and its glyphs reused wherever it shows up again.
//!
//! Shaping is what a busy screen costs: CoreText sets up the font's lookups for
//! every run of every line it shapes, so a screen that changes all over every
//! frame (`cmatrix`, a full-screen redraw, output streaming in) spent nearly all
//! of its frame there. A row's words come back again and again, though, in other
//! colors and at other columns, so they are shaped by their text and fonts alone
//! and painted glyph by glyph in the colors of the moment. Ligatures and
//! contextual forms stay right: they never reach across a blank, and a word is
//! still shaped whole with its glyphs pinned to its cells.
//!
//! Some screens make new words faster than CoreText shapes them: `donut` prints new runs of
//! `.,-~:;=!*#$@` in every row of every frame, which a font with ligatures has to shape whole, and
//! that was 30 ms a frame on an M5 Pro. So a frame shapes for [`SHAPE_BUDGET`] at most; a new word
//! after that is drawn character by character, from glyphs shaped one each and kept, and its row
//! is built again next frame, when the word is shaped in full (or, in an animation, gone).

use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::{FontId, GlyphId, Hsla, Pixels, Point};

use super::rows::{Segment, Style};

/// A glyph as shaping placed it, from its word's first cell.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Shaped {
    pub position: Point<Pixels>,
    /// The byte in the word it was shaped from, which gives its color.
    pub index: usize,
    pub font_id: FontId,
    pub id: GlyphId,
    pub emoji: bool,
}

/// A glyph ready to paint.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Glyph {
    pub position: Point<Pixels>,
    pub font_id: FontId,
    pub id: GlyphId,
    pub emoji: bool,
    /// From a word of wide characters (two cells each): a double-width row centres it in its
    /// doubled cells.
    pub wide: bool,
    pub color: Hsla,
    /// Blinking text: painted faded in the blink clock's off phase.
    pub blink: bool,
}

/// What a word's shape depends on: its text, and which runs are bold or italic.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
struct WordKey {
    wide: bool,
    text: String,
    /// Byte lengths with bold and italic, consecutive equal fonts merged.
    fonts: Vec<(usize, bool, bool)>,
}

impl WordKey {
    /// The key of the word in `bytes` of the segment, reusing this one's allocations.
    fn fill(&mut self, segment: &Segment, bytes: std::ops::Range<usize>) {
        self.wide = segment.wide;
        self.text.clear();
        self.text.push_str(&segment.text[bytes.clone()]);
        self.fonts.clear();
        let mut at = 0;
        for (len, style) in &segment.runs {
            let (start, end) = (at.max(bytes.start), (at + len).min(bytes.end));
            at += len;
            if start >= end {
                continue;
            }
            match self.fonts.last_mut() {
                Some((last, bold, italic)) if (*bold, *italic) == (style.bold, style.italic) => {
                    *last += end - start
                }
                _ => self.fonts.push((end - start, style.bold, style.italic)),
            }
        }
    }
}

/// Enough for every word a session shows for a while; past that it starts over
/// rather than keep a flood of one-off words (hashes, random data) forever.
const MAX_WORDS: usize = 16_384;

/// How long one frame of a terminal may spend shaping new words: half a frame at 120 Hz, which
/// leaves the rest of the window and the GPU the other half.
pub(super) const SHAPE_BUDGET: Duration = Duration::from_millis(4);

/// How much shaping the frame being built has left.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Budget {
    /// No limit: tests, and anything that isn't drawing a frame.
    Unlimited,
    Until(Instant),
    /// Past its time: new words are drawn character by character.
    Spent,
}

pub(super) struct WordCache<K> {
    /// What every word was shaped with (font, size, cell width, ligatures).
    key: Option<K>,
    words: HashMap<WordKey, Rc<[Shaped]>>,
    /// Characters shaped one by one (those that stand alone, and a word's characters while the
    /// frame's budget is spent), by character, wide, bold and italic.
    chars: HashMap<(char, bool, bool, bool), Rc<[Shaped]>>,
    /// The key being looked up, kept so a lookup doesn't allocate.
    scratch: WordKey,
    budget: Budget,
    /// A word was drawn character by character since [`Self::take_deferred`] last asked.
    deferred: bool,
}

impl<K> Default for WordCache<K> {
    fn default() -> Self {
        Self {
            key: None,
            words: HashMap::new(),
            chars: HashMap::new(),
            scratch: WordKey::default(),
            budget: Budget::Unlimited,
            deferred: false,
        }
    }
}

/// Characters whose glyph never depends on its neighbours: no font has ligatures or contextual
/// forms across them, and each has its own cell or two. A word of them is looked up character by
/// character, so a screen that keeps making new words of them (`cmatrix`'s katakana, CJK text,
/// braille graphs in `btop`) stops being shaped once each character has been seen. Everything
/// else, combining marks and joiners included, is still shaped as a whole word.
fn stands_alone(c: char) -> bool {
    matches!(
        u32::from(c),
        0x2800..=0x28FF // braille
            | 0x3041..=0x3096 // hiragana letters
            | 0x30A1..=0x30FA // katakana letters
            | 0x3400..=0x4DBF // CJK ideographs, extension A
            | 0x4E00..=0x9FFF // CJK ideographs
            | 0xAC00..=0xD7A3 // hangul syllables
            | 0xFF66..=0xFF9D // half-width katakana letters
    )
}

impl<K: PartialEq> WordCache<K> {
    /// Forget every word when what they were shaped with changed.
    pub fn sync(&mut self, key: K) {
        if self.key.as_ref() != Some(&key) {
            self.words.clear();
            self.chars.clear();
            self.key = Some(key);
        }
    }

    /// A frame starts: until `until` it may shape new words, after that they are drawn character
    /// by character. `None`: no limit.
    pub fn start_frame(&mut self, until: Option<Instant>) {
        self.budget = until.map_or(Budget::Unlimited, Budget::Until);
    }

    /// Whether a word was drawn character by character since the last call: what it was drawn in
    /// should be built again in the next frame.
    pub fn take_deferred(&mut self) -> bool {
        std::mem::take(&mut self.deferred)
    }

    /// Whether the frame may still shape a word it hasn't seen.
    fn may_shape(&mut self) -> bool {
        match self.budget {
            Budget::Unlimited => true,
            Budget::Spent => false,
            Budget::Until(until) if Instant::now() < until => true,
            Budget::Until(_) => {
                self.budget = Budget::Spent;
                false
            }
        }
    }

    /// The glyphs of `segment` in its colors, placed on a grid of `cell_width`; `shape` shapes a
    /// word (or a part of one) it hasn't seen before.
    pub fn glyphs(
        &mut self,
        segment: &Segment,
        cell_width: Pixels,
        mut shape: impl FnMut(&Segment) -> Vec<Shaped>,
    ) -> Vec<Glyph> {
        let mut out = Vec::new();
        let per_char = if segment.wide { 2 } else { 1 };
        // Every character in a cell (or two) of its own: no marks or joiners that need a neighbour.
        let one_per_cell = segment.text.chars().count() * per_char == segment.cells();
        if !(one_per_cell && segment.text.chars().any(stands_alone)) {
            self.word(segment, 0..segment.text.len(), 0, one_per_cell, cell_width, &mut shape, &mut out);
            return out;
        }
        // The characters that don't stand alone are shaped together, as the word they make.
        let mut pending: Option<(usize, usize)> = None;
        let mut runs = Runs::new(segment);
        for (k, (byte, c)) in segment.text.char_indices().enumerate() {
            if !stands_alone(c) {
                pending.get_or_insert((byte, k * per_char));
                continue;
            }
            if let Some((start, cell)) = pending.take() {
                self.word(segment, start..byte, cell, true, cell_width, &mut shape, &mut out);
            }
            self.character(segment, &mut runs, (byte, c), k * per_char, cell_width, &mut shape, &mut out);
        }
        if let Some((start, cell)) = pending {
            self.word(segment, start..segment.text.len(), cell, true, cell_width, &mut shape, &mut out);
        }
        out
    }

    /// The glyphs of character `c` at byte `byte` and `cell` cells into the segment, shaped once.
    #[allow(clippy::too_many_arguments)]
    fn character(
        &mut self,
        segment: &Segment,
        runs: &mut Runs,
        (byte, c): (usize, char),
        cell: usize,
        cell_width: Pixels,
        shape: &mut impl FnMut(&Segment) -> Vec<Shaped>,
        out: &mut Vec<Glyph>,
    ) {
        let (bold, italic) = runs.at(byte).map_or((false, false), |style| (style.bold, style.italic));
        let key = (c, segment.wide, bold, italic);
        let shaped = match self.chars.get(&key) {
            Some(shaped) => shaped.clone(),
            None => {
                if self.chars.len() >= MAX_WORDS {
                    self.chars.clear();
                }
                let shaped: Rc<[Shaped]> = shape(&segment.part(byte..byte + c.len_utf8(), cell)).into();
                self.chars.insert(key, shaped.clone());
                shaped
            }
        };
        place(segment, &shaped, byte, cell, cell_width, out);
    }

    /// The glyphs of the word in `bytes` of the segment, `cell` cells in, shaped once. A new word
    /// whose characters each have a cell (`one_per_cell`) waits for a frame with time to shape it
    /// once this one has spent its budget, and is drawn character by character until then.
    #[allow(clippy::too_many_arguments)]
    fn word(
        &mut self,
        segment: &Segment,
        bytes: std::ops::Range<usize>,
        cell: usize,
        one_per_cell: bool,
        cell_width: Pixels,
        shape: &mut impl FnMut(&Segment) -> Vec<Shaped>,
        out: &mut Vec<Glyph>,
    ) {
        let whole = bytes == (0..segment.text.len());
        self.scratch.fill(segment, bytes.clone());
        let shaped = match self.words.get(&self.scratch).cloned() {
            Some(shaped) => shaped,
            None if one_per_cell && !self.may_shape() => {
                self.deferred = true;
                let per_char = if segment.wide { 2 } else { 1 };
                let mut runs = Runs::new(segment);
                for (k, (byte, c)) in segment.text[bytes.clone()].char_indices().enumerate() {
                    let at = (bytes.start + byte, c);
                    self.character(segment, &mut runs, at, cell + k * per_char, cell_width, shape, out);
                }
                return;
            }
            None => {
                if self.words.len() >= MAX_WORDS {
                    self.words.clear();
                }
                let shaped: Rc<[Shaped]> =
                    if whole { shape(segment) } else { shape(&segment.part(bytes.clone(), cell)) }.into();
                self.words.insert(self.scratch.clone(), shaped.clone());
                shaped
            }
        };
        place(segment, &shaped, bytes.start, cell, cell_width, out);
    }
}

/// Shaped glyphs of a word that starts at byte `byte` and `cell` cells into the segment, in the
/// segment's colors.
fn place(
    segment: &Segment,
    shaped: &[Shaped],
    byte: usize,
    cell: usize,
    cell_width: Pixels,
    out: &mut Vec<Glyph>,
) {
    let dx = cell_width * cell as f32;
    let mut runs = Runs::new(segment);
    out.extend(shaped.iter().map(|glyph| {
        let style = runs.at(byte + glyph.index);
        Glyph {
            position: Point { x: glyph.position.x + dx, y: glyph.position.y },
            font_id: glyph.font_id,
            id: glyph.id,
            emoji: glyph.emoji,
            wide: segment.wide,
            color: style.map(|style| style.fg).unwrap_or_default(),
            blink: style.is_some_and(|style| style.blink),
        }
    }));
}

/// The style runs of a segment, looked up by byte. Lookups mostly go forward (a row's glyphs come
/// in order), so it walks on from the last one instead of starting over: a row where every cell has
/// its own color would otherwise cost a pass over all its runs per glyph.
struct Runs<'a> {
    runs: &'a [(usize, Style)],
    run: usize,
    start: usize,
}

impl<'a> Runs<'a> {
    fn new(segment: &'a Segment) -> Self {
        Self { runs: &segment.runs, run: 0, start: 0 }
    }

    /// The style of the run byte `index` is in (the last run past the end).
    fn at(&mut self, index: usize) -> Option<Style> {
        if index < self.start {
            // Backwards (right-to-left text): start over.
            (self.run, self.start) = (0, 0);
        }
        while self.run + 1 < self.runs.len() && index >= self.start + self.runs[self.run].0 {
            self.start += self.runs[self.run].0;
            self.run += 1;
        }
        self.runs.get(self.run).map(|(_, style)| *style)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{FontId, GlyphId, point, px};
    use wormhole_core::terminal::types::{Cell, Color, Colors, Flags, NamedColor, RowExtras};
    use wormhole_core::theme;

    use std::time::Instant;

    use super::{Shaped, WordCache};
    use crate::palette::Palette;
    use crate::terminal_view::render::rows::{Marks, Segment, build_row};

    fn segments(text: &str, red: &[usize], bold: &[usize]) -> Vec<Segment> {
        let cells: Vec<Cell> = text
            .chars()
            .enumerate()
            .map(|(col, c)| {
                let mut cell = Cell { c, ..Cell::default() };
                if red.contains(&col) {
                    cell.fg = Color::Named(NamedColor::Red);
                }
                if bold.contains(&col) {
                    cell.flags = Flags::BOLD;
                }
                cell
            })
            .collect();
        let palette = Palette::from_theme(&theme::builtin()[0]);
        build_row(&cells, &RowExtras::default(), &palette, &Colors::default(), &Marks::default()).segments
    }

    /// One glyph per character, one cell apart.
    fn fake_shape(segment: &Segment) -> Vec<Shaped> {
        segment
            .text
            .char_indices()
            .enumerate()
            .map(|(k, (index, c))| Shaped {
                position: point(px(8.0 * k as f32), px(0.0)),
                index,
                font_id: FontId(0),
                id: GlyphId(c as u32),
                emoji: false,
            })
            .collect()
    }

    #[test]
    fn a_word_is_shaped_once_whatever_its_color_and_column() {
        let mut cache = WordCache::default();
        cache.sync(0u8);
        let shaped = std::cell::Cell::new(0);
        let glyphs = |cache: &mut WordCache<u8>, segment: &Segment| {
            cache.glyphs(segment, px(8.0), |segment| {
                shaped.set(shaped.get() + 1);
                fake_shape(segment)
            })
        };
        let plain = segments("ab ab", &[], &[]);
        let red = segments("  ab", &[2, 3], &[]);
        let first = glyphs(&mut cache, &plain[0]);
        glyphs(&mut cache, &plain[1]);
        let colored = glyphs(&mut cache, &red[0]);
        assert_eq!(shaped.get(), 1, "the same word in another place and color isn't shaped again");
        let palette = Palette::from_theme(&theme::builtin()[0]);
        assert_eq!(first[1].color, palette.foreground);
        assert_eq!(colored[1].color, palette.ansi[1]);
        assert_eq!(colored[1].position, first[1].position);

        glyphs(&mut cache, &segments("ab", &[], &[1])[0]);
        assert_eq!(shaped.get(), 2, "a bold letter is another font");
        cache.sync(1);
        glyphs(&mut cache, &plain[0]);
        assert_eq!(shaped.get(), 3, "another font or size shapes again");
    }

    #[test]
    fn past_the_budget_a_new_word_is_drawn_character_by_character_until_a_frame_shapes_it() {
        let mut cache = WordCache::default();
        cache.sync(0u8);
        let asked = std::cell::RefCell::new(Vec::new());
        let segment = &segments("=>=", &[1], &[])[0];
        let glyphs = |cache: &mut WordCache<u8>| {
            cache.glyphs(segment, px(8.0), |segment| {
                asked.borrow_mut().push(segment.text.clone());
                fake_shape(segment)
            })
        };
        // Spent before it started.
        cache.start_frame(Some(Instant::now()));
        let by_char = glyphs(&mut cache);
        assert!(cache.take_deferred(), "the row has to be built again");
        assert!(!cache.take_deferred(), "asked once");
        // The next frame has time: the word is shaped whole, in the same cells and colors.
        cache.start_frame(None);
        let whole = glyphs(&mut cache);
        assert!(!cache.take_deferred());
        assert_eq!(*asked.borrow(), ["=", ">", "=>="], "its characters once each, then the word");
        assert_eq!(by_char, whole);
        // Seen words don't wait, whatever is left of the budget; characters already shaped aren't
        // shaped again.
        cache.start_frame(Some(Instant::now()));
        glyphs(&mut cache);
        assert!(!cache.take_deferred());
        assert_eq!(asked.borrow().len(), 3);
    }

    #[test]
    fn a_word_with_a_mark_is_shaped_whatever_the_budget() {
        let mut cache = WordCache::default();
        cache.sync(0u8);
        let mut extras = RowExtras::default();
        extras.push_zerowidth(0, '\u{0301}');
        cache.start_frame(Some(Instant::now()));
        // Drawn by character the mark would lose its letter.
        let (asked, _) = shaped_row(&mut cache, &row("ab"), &extras);
        assert_eq!(asked, ["a\u{0301}b"]);
        assert!(!cache.take_deferred());
    }

    #[test]
    fn each_glyph_takes_the_color_of_its_run() {
        let mut cache = WordCache::default();
        cache.sync(0u8);
        let segment = &segments("abc", &[1], &[])[0];
        let colors: Vec<_> =
            cache.glyphs(segment, px(8.0), fake_shape).into_iter().map(|g| g.color).collect();
        let palette = Palette::from_theme(&theme::builtin()[0]);
        assert_eq!(colors, [palette.foreground, palette.ansi[1], palette.foreground]);
    }

    /// The words `shape` was asked for, and the glyphs, for a row of cells.
    fn shaped_row(
        cache: &mut WordCache<u8>,
        cells: &[Cell],
        extras: &RowExtras,
    ) -> (Vec<String>, Vec<(f32, u32, gpui_kit::Hsla)>) {
        let palette = Palette::from_theme(&theme::builtin()[0]);
        let mut asked = Vec::new();
        let mut glyphs = Vec::new();
        for segment in build_row(cells, extras, &palette, &Colors::default(), &Marks::default()).segments {
            let col = segment.col as f32 * 8.0;
            let found = cache.glyphs(&segment, px(8.0), |segment| {
                asked.push(segment.text.clone());
                fake_shape(segment)
            });
            glyphs.extend(found.into_iter().map(|g| (col + f32::from(g.position.x), g.id.0, g.color)));
        }
        (asked, glyphs)
    }

    fn row(text: &str) -> Vec<Cell> {
        let mut cells = Vec::new();
        for c in text.chars() {
            if matches!(c, '日' | '本') {
                cells.push(Cell { c, flags: Flags::WIDE_CHAR, ..Cell::default() });
                cells.push(Cell { c: ' ', flags: Flags::WIDE_CHAR_SPACER, ..Cell::default() });
            } else {
                cells.push(Cell { c, ..Cell::default() });
            }
        }
        cells
    }

    #[test]
    fn characters_that_stand_alone_are_shaped_once_each_whatever_word_they_are_in() {
        let mut cache = WordCache::default();
        cache.sync(0u8);
        let (asked, glyphs) = shaped_row(&mut cache, &row("aﾊﾞxyｱｲ"), &RowExtras::default());
        // ﾞ (a voiced mark) isn't a letter that stands alone: it is shaped with what follows it.
        assert_eq!(asked, ["a", "ﾊ", "ﾞxy", "ｱ", "ｲ"]);
        let at: Vec<(f32, u32)> = glyphs.iter().map(|g| (g.0, g.1)).collect();
        let ids: Vec<u32> = "aﾊﾞxyｱｲ".chars().map(u32::from).collect();
        assert_eq!(at, ids.iter().enumerate().map(|(k, id)| (8.0 * k as f32, *id)).collect::<Vec<_>>());

        // The same characters in new words: nothing new to shape.
        let (asked, _) = shaped_row(&mut cache, &row("ｲｱﾊ ｱa"), &RowExtras::default());
        assert!(asked.is_empty(), "{asked:?}");
    }

    #[test]
    fn wide_characters_stand_alone_two_cells_apart() {
        let mut cache = WordCache::default();
        cache.sync(0u8);
        let (asked, glyphs) = shaped_row(&mut cache, &row("日本日"), &RowExtras::default());
        assert_eq!(asked, ["日", "本"]);
        let at: Vec<f32> = glyphs.iter().map(|g| g.0).collect();
        assert_eq!(at, [0.0, 16.0, 32.0]);
    }

    #[test]
    fn a_character_with_a_combining_mark_is_shaped_with_its_word() {
        let mut cache = WordCache::default();
        cache.sync(0u8);
        let cells = row("ｱｲ");
        let mut extras = RowExtras::default();
        extras.push_zerowidth(1, '\u{0301}');
        let (asked, _) = shaped_row(&mut cache, &cells, &extras);
        assert_eq!(asked, ["ｱｲ\u{0301}"]);
    }

    #[test]
    fn every_character_keeps_its_own_color_and_font() {
        let mut cache = WordCache::default();
        cache.sync(0u8);
        let mut cells = row("ｱｱxｱ");
        cells[1].fg = Color::Named(NamedColor::Red);
        cells[3].flags = Flags::BOLD;
        let (asked, glyphs) = shaped_row(&mut cache, &cells, &RowExtras::default());
        assert_eq!(asked, ["ｱ", "x", "ｱ"], "a bold ｱ is another font");
        let palette = Palette::from_theme(&theme::builtin()[0]);
        let colors: Vec<_> = glyphs.iter().map(|g| g.2).collect();
        assert_eq!(colors[0], palette.foreground);
        assert_eq!(colors[1], palette.ansi[1]);
        assert_eq!(colors[2], palette.foreground);
    }
}
