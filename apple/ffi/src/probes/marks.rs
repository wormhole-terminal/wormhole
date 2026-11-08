//! What a terminal's frame says about the Probes' marks and about folds, as two sections that follow the rows
//! (`FFI-CONTRACT.md`, "Probe decorations and fold markers"). Additive to layout version 1, like the placements:
//! a reader that does not know them reads the rows and stops, and sees the text as it is.
//!
//! After the cell extras, each when its header flag is set. Like the extras, they are as complete as the rows
//! the frame carries: a row the frame names and no record names has no mark.
//!
//! ```text
//! decorations (flag 8): count, then per row with a mark
//!     row, the block's tone, number of spans m, then per span
//!         first column, last column, tone, number of characters k (0: the text stays, else the span's width)
//!         then k scalar values: what each cell shows instead of its text
//! fold markers (flag 16): count, then per marker row
//!     row, fold id (low word, high word), lines hidden, flags (bit 0 open, bit 1 its block failed),
//!     then three strings (label, spoken, tooltip), each a byte length (`NONE`: no string) and its UTF-8 bytes
//!     padded to a whole word
//! tone: 0 none, else 1 + the tone's place in `Tone::ALL` (link, danger, warning, success, muted)
//! ```
//!
//! The spans are `probe::runtime::Decor`'s, the words `view::fold::marker_words`: Rust decides, Swift draws.

use wormhole_core::probe::runtime::{Decor, Inputs, RuleSpan};
use wormhole_core::probe::tone::Tone;
use wormhole_core::view::fold::{MarkerWords, marker_words};
use wormhole_term::{Pty, ScreenSnapshot};

use super::state;
use crate::frame::FLAGS_WORD;

pub(crate) const FLAG_DECOR: u32 = 8;
pub(crate) const FLAG_FOLDS: u32 = 16;
const NONE: u32 = u32::MAX;

/// What one row carries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RowMarks {
    spans: Vec<RuleSpan>,
    block: Option<Tone>,
}

impl RowMarks {
    fn is_empty(&self) -> bool {
        self.spans.is_empty() && self.block.is_none()
    }
}

/// A terminal's spans between frames, and what the last frame told Swift of each row.
#[derive(Default)]
pub(crate) struct Marks {
    decor: Decor,
    sent: Vec<RowMarks>,
    /// The viewport cell where the span under the pointer starts, when it shows its text.
    reveal: Option<(usize, usize)>,
    cell: Option<(u32, u32, bool)>,
    /// Whether the terminal's view shows every line, as last applied to it.
    folds_off: bool,
}

fn tone_word(tone: Option<Tone>) -> u32 {
    tone.and_then(|tone| Tone::ALL.iter().position(|t| *t == tone)).map_or(0, |at| at as u32 + 1)
}

impl Marks {
    /// Before the snapshot is taken: the view shows what "never hide lines" says. Whether that changed it.
    pub(crate) fn sync_folds(&mut self, pty: &Pty) -> bool {
        let off = state::folds_off();
        if off == self.folds_off {
            return false;
        }
        self.folds_off = off;
        pty.set_folds_off(off);
        true
    }

    /// After the snapshot: work the spans out for it. Whether the frame must carry every row, because a row's marks
    /// changed that the engine did not damage (rules loaded, a hook tinted a block, the pointer revealed a span).
    pub(crate) fn refresh(&mut self, tab: i64, snapshot: &ScreenSnapshot) -> bool {
        let (rules, generation) = state::rules();
        let block_tone = |block: u64| state::block_tone(tab, block);
        self.decor.update(
            snapshot,
            &Inputs { rules: rules.as_deref(), generation, reveal: self.reveal, block_tone: &block_tone },
        );
        let rows = snapshot.rows().len();
        self.sent.resize_with(rows, RowMarks::default);
        (0..rows).any(|row| !snapshot.rows()[row].changed() && self.marks(row) != self.sent[row])
    }

    fn marks(&self, row: usize) -> RowMarks {
        RowMarks { spans: self.decor.rules(row).to_vec(), block: self.decor.block(row) }
    }

    /// Append the sections for the rows `frame` carries (all of them when `all`), when any has something.
    pub(crate) fn append(&mut self, frame: &mut Vec<u8>, snapshot: &ScreenSnapshot, all: bool) {
        let mut flags = 0;
        let mut decor = Vec::new();
        let mut marked = 0u32;
        for row in 0..snapshot.rows().len() {
            if !(all || snapshot.rows()[row].changed()) {
                continue;
            }
            let now = self.marks(row);
            if !now.is_empty() {
                marked += 1;
                put_row(&mut decor, row, &now);
            }
            self.sent[row] = now;
        }
        if marked > 0 {
            frame.extend_from_slice(&marked.to_le_bytes());
            frame.extend_from_slice(&decor);
            flags |= FLAG_DECOR;
        }
        if folds_section(frame, snapshot, all) {
            flags |= FLAG_FOLDS;
        }
        if flags != 0 {
            let at = FLAGS_WORD * 4;
            let word = u32::from_le_bytes([frame[at], frame[at + 1], frame[at + 2], frame[at + 3]]) | flags;
            frame[at..at + 4].copy_from_slice(&word.to_le_bytes());
        }
    }

    /// The pointer is over viewport cell (`row`, `col`), with ⌥ as `alt`: the span there, when its rule lets it
    /// show its text, is the revealed one. `at` is that span's start cell, worked out by the caller. Whether the
    /// revealed span changed (the next frame is then a full one).
    pub(crate) fn set_reveal(&mut self, cell: Option<(u32, u32, bool)>, at: Option<(usize, usize)>) -> bool {
        self.cell = cell;
        std::mem::replace(&mut self.reveal, at) != at
    }

    /// The cell the pointer was last looked up for: while it stays in it nothing is asked again.
    pub(crate) fn last_cell(&self) -> Option<(u32, u32, bool)> {
        self.cell
    }
}

fn put_row(out: &mut Vec<u8>, row: usize, marks: &RowMarks) {
    for word in [row as u32, tone_word(marks.block), marks.spans.len() as u32] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    for span in &marks.spans {
        let mask = span.mask.as_deref().unwrap_or(&[]);
        for word in [span.first as u32, span.last as u32, tone_word(span.tone), mask.len() as u32] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        for c in mask {
            out.extend_from_slice(&u32::from(*c).to_le_bytes());
        }
    }
}

fn put_text(out: &mut Vec<u8>, text: Option<&str>) {
    let Some(text) = text else {
        out.extend_from_slice(&NONE.to_le_bytes());
        return;
    };
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
    out.resize(out.len().div_ceil(4) * 4, 0);
}

/// The marker rows of the frame's rows. Whether there is any.
fn folds_section(frame: &mut Vec<u8>, snapshot: &ScreenSnapshot, all: bool) -> bool {
    let start = frame.len();
    frame.extend_from_slice(&0u32.to_le_bytes());
    let mut count = 0u32;
    for (row, shown) in snapshot.rows().iter().enumerate() {
        let Some(fold) = shown.fold().filter(|_| all || shown.changed()) else { continue };
        let owner = fold.owner.as_deref().map(|owner| (owner.name.as_str(), owner.label.as_str()));
        let MarkerWords { label, spoken, tooltip } = marker_words(fold.hidden(), fold.expanded, owner, 0);
        let flags = u32::from(fold.expanded) | u32::from(fold.failed) << 1;
        for word in [row as u32, fold.id as u32, (fold.id >> 32) as u32, fold.hidden() as u32, flags] {
            frame.extend_from_slice(&word.to_le_bytes());
        }
        put_text(frame, Some(&label));
        put_text(frame, Some(&spoken));
        put_text(frame, tooltip.as_deref());
        count += 1;
    }
    if count == 0 {
        frame.truncate(start);
        return false;
    }
    frame[start..start + 4].copy_from_slice(&count.to_le_bytes());
    true
}
