//! Each word of a row shaped once and its glyphs reused wherever it shows up again.
//!
//! Shaping is what a busy screen costs: CoreText sets up the font's lookups for
//! every run of every line it shapes, so a screen that changes all over every
//! frame (`cmatrix`, a full-screen redraw, output streaming in) spent nearly all
//! of its frame there. A row's words come back again and again, though, in other
//! colors and at other columns, so they are shaped by their text and fonts alone
//! and painted glyph by glyph in the colors of the moment. Ligatures and
//! contextual forms stay right: they never reach across a blank, and a word is
//! still shaped whole with its glyphs pinned to its cells (one longer than
//! [`MAX_SHAPED_CELLS`] in parts of that many).
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
            if at >= bytes.end {
                break;
            }
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

/// The longest word shaped in one piece; a longer one is shaped in parts of this many cells, each
/// cached like a word. The frame's budget is checked before each word, so what one word costs is
/// what a frame can overshoot it by: a screen-wide word of new characters every row (a flood of
/// random text, `ttystorm flood --kind colorflood`) took a whole budget for one or two rows. A
/// ligature or contextual form across the edge of two parts isn't made; that takes a word longer
/// than this, with one exactly there (see [`cut`]), and only changes how two characters look.
pub(super) const MAX_SHAPED_CELLS: usize = 64;

/// How far back from [`MAX_SHAPED_CELLS`] a cut may move to land next to a letter or digit.
const CUT_SNAP: usize = 16;

/// Where the first part of `text` ends when it has more than `chars` characters: the byte and how
/// many characters come before it. Ligatures and contextual forms are mostly between punctuation
/// (`=>`, `!=`, `-->`), so the cut moves back, by up to [`CUT_SNAP`] characters, to the first place
/// with an ASCII letter or digit on one side; with none there it stays at `chars`. It depends on the
/// text alone, so the same word is cut the same way and its parts are found in the cache again.
fn cut(text: &str, chars: usize) -> Option<(usize, usize)> {
    let (hard, mut after) = text.char_indices().nth(chars)?;
    let mut at = (hard, chars);
    for (byte, before) in text[..hard].char_indices().rev().take(CUT_SNAP.min(chars - 1)) {
        if before.is_ascii_alphanumeric() || after.is_ascii_alphanumeric() {
            return Some(at);
        }
        (after, at) = (before, (byte, at.1 - 1));
    }
    Some((hard, chars))
}

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

    /// The glyphs of the word in `bytes` of the segment, `cell` cells in. A word whose characters
    /// each have a cell (`one_per_cell`) and that is longer than [`MAX_SHAPED_CELLS`] is shaped in
    /// parts of at most that many cells, cut where [`cut`] says, from its start. A word with
    /// combining marks or joiners is still shaped whole, however long: a cut could split a
    /// character from its mark.
    #[allow(clippy::too_many_arguments)]
    fn word(
        &mut self,
        segment: &Segment,
        mut bytes: std::ops::Range<usize>,
        mut cell: usize,
        one_per_cell: bool,
        cell_width: Pixels,
        shape: &mut impl FnMut(&Segment) -> Vec<Shaped>,
        out: &mut Vec<Glyph>,
    ) {
        if one_per_cell {
            let per_char = if segment.wide { 2 } else { 1 };
            let chars = (MAX_SHAPED_CELLS / per_char).max(1);
            while let Some((split, taken)) = cut(&segment.text[bytes.clone()], chars) {
                let end = bytes.start + split;
                self.part(segment, bytes.start..end, cell, true, cell_width, shape, out);
                (bytes.start, cell) = (end, cell + taken * per_char);
            }
        }
        self.part(segment, bytes, cell, one_per_cell, cell_width, shape, out);
    }

    /// The glyphs of the word (or part of one) in `bytes` of the segment, `cell` cells in, shaped
    /// once. A new one whose characters each have a cell (`one_per_cell`) waits for a frame with
    /// time to shape it once this one has spent its budget, and is drawn character by character
    /// until then.
    #[allow(clippy::too_many_arguments)]
    fn part(
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
