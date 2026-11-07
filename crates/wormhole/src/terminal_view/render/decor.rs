//! What a Probe's rules and hooks add to the rows: tones on matched text, text shortened or redacted
//! on screen, and the tint of a block a hook marked. Display only: the grid, the selection, find, copy and
//! the saved text never see any of it.
//!
//! The rules match a *logical line*, the screen rows a soft wrap joined, so a link or a key that wraps is one
//! match and a mask covers every row of it. The work is bounded the way the row cache bounds the build:
//! - a line none of whose rows changed keeps the spans it had, so a frame with no changed row scans nothing;
//! - a line that did change is looked up by its text first (scrolled output is the same lines a row
//!   higher), so only text never seen before is run through the rules;
//! - with no rules in force none of this runs and the rows are built as they always were.

use std::collections::HashMap;

use gpui_kit::App;
use wormhole_core::probe::tone::Tone;
use wormhole_term::{RevealWhen, ScreenSnapshot};

use crate::probe_runtime::{Decoration, RuleSet};
use crate::terminal_view::TerminalView;

/// Lines whose spans are remembered by text: more than a few screens, so scrolling back over output that
/// was seen finds it, and fewer than a scrollback, so the memo stays small.
const MEMO_LINES: usize = 1024;

/// One rule's mark on one row: the columns `first..=last`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RuleSpan {
    pub first: usize,
    pub last: usize,
    pub tone: Option<Tone>,
    /// What each cell from `first` shows instead of its text; `None` leaves the text.
    pub mask: Option<Vec<char>>,
}

/// What the rows were worked out for: when any of it differs the spans are worked out again.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    generation: u64,
    reveal: Option<(usize, usize)>,
    rows: usize,
    cols: usize,
}

/// What one line of text came to, in characters of the line.
type Hits = Vec<(std::ops::Range<usize>, Decoration)>;

#[derive(Default)]
pub(super) struct Decor {
    /// The spans on each viewport row; empty past the end, which is every row while no rules are in force.
    rows: Vec<Vec<RuleSpan>>,
    /// The tone of the block each viewport row is in, when a hook gave it one.
    block: Vec<Option<Tone>>,
    stamp: Option<Stamp>,
    memo: HashMap<String, Hits>,
    // Scratch, kept for the next line.
    next: Vec<Vec<RuleSpan>>,
    text: String,
    cells: Vec<(usize, usize, bool)>,
}

/// What `Decor::update` reads besides the screen.
pub(super) struct Inputs<'a> {
    pub rules: Option<&'a RuleSet>,
    /// Which rules these are: spans are worked out again when it changes.
    pub generation: u64,
    /// The viewport cell where a shortened or redacted span starts that shows its text now (the pointer is
    /// over it).
    pub reveal: Option<(usize, usize)>,
    /// The tone a hook gave the block with this number.
    pub block_tone: &'a dyn Fn(u64) -> Option<Tone>,
}

impl Decor {
    pub fn rules(&self, row: usize) -> &[RuleSpan] {
        self.rows.get(row).map_or(&[], Vec::as_slice)
    }

    pub fn block(&self, row: usize) -> Option<Tone> {
        self.block.get(row).copied().flatten()
    }

    /// Bring the spans in line with `snapshot` (already updated).
    pub fn update(&mut self, snapshot: &ScreenSnapshot, inputs: &Inputs<'_>) {
        let rows = snapshot.rows();
        self.block.clear();
        self.block.resize(rows.len(), None);
        for span in snapshot.block_spans() {
            if let Some(tone) = (inputs.block_tone)(span.id) {
                self.block[span.rows.clone()].fill(Some(tone));
            }
        }
        let Some(rules) = inputs.rules else {
            self.rows.clear();
            self.memo.clear();
            self.stamp = None;
            return;
        };
        let stamp = Stamp {
            generation: inputs.generation,
            reveal: inputs.reveal,
            rows: rows.len(),
            cols: snapshot.columns(),
        };
        let fresh = self.stamp.as_ref() != Some(&stamp);
        if self.stamp.as_ref().is_none_or(|old| old.generation != stamp.generation) {
            self.memo.clear();
        }
        let reuse = !fresh && self.rows.len() == rows.len();
        let mut next = std::mem::take(&mut self.next);
        next.iter_mut().for_each(Vec::clear);
        next.resize_with(rows.len(), Vec::new);
        let mut first = 0;
        while first < rows.len() {
            let mut last = first;
            while last + 1 < rows.len() && rows[last].wraps() {
                last += 1;
            }
            if reuse && rows[first..=last].iter().all(|row| !row.changed()) {
                for (kept, old) in next[first..=last].iter_mut().zip(&mut self.rows[first..=last]) {
                    *kept = std::mem::take(old);
                }
            } else {
                self.line(rules, snapshot, first..=last, inputs.reveal, &mut next);
            }
            first = last + 1;
        }
        self.next = std::mem::replace(&mut self.rows, next);
        self.stamp = Some(stamp);
    }

    /// The spans of one logical line, over the viewport rows `rows`, into `out`.
    fn line(
        &mut self,
        rules: &RuleSet,
        snapshot: &ScreenSnapshot,
        rows: std::ops::RangeInclusive<usize>,
        reveal: Option<(usize, usize)>,
        out: &mut [Vec<RuleSpan>],
    ) {
        use wormhole_term::types::Flags;
        self.text.clear();
        self.cells.clear();
        for row in rows {
            let built = &snapshot.rows()[row];
            let cells = built.cells();
            for (col, cell) in cells.iter().enumerate().take(built.size().visible_columns(cells.len())) {
                if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                    continue;
                }
                self.text.push(if cell.c == '\0' { ' ' } else { cell.c });
                self.cells.push((row, col, cell.flags.contains(Flags::WIDE_CHAR)));
            }
        }
        // Blanks past the text have nothing for a rule to find.
        let text = self.text.trim_end();
        if text.is_empty() {
            return;
        }
        if !self.memo.contains_key(text) {
            if self.memo.len() >= MEMO_LINES {
                self.memo.clear();
            }
            let hits = rules
                .decorate(text)
                .into_iter()
                .map(|found| {
                    let chars =
                        text[..found.range.start].chars().count()..text[..found.range.end].chars().count();
                    (chars, found)
                })
                .collect();
            self.memo.insert(text.to_string(), hits);
        }
        let Some(hits) = self.memo.get(text) else { return };
        for (chars, found) in hits {
            let Some(cells) = self.cells.get(chars.clone()) else { continue };
            let Some(&(row0, col0, _)) = cells.first() else { continue };
            // A masked span the pointer is over shows its text, when its rule lets it.
            let revealed =
                found.mask.is_some() && found.reveal != RevealWhen::Never && reveal == Some((row0, col0));
            let mask = found.mask.as_ref().filter(|_| !revealed);
            if found.tone.is_none() && mask.is_none() {
                continue;
            }
            // One span per row the match covers.
            let mut at = 0;
            while at < cells.len() {
                let row = cells[at].0;
                let end = at + cells[at..].iter().take_while(|(r, _, _)| *r == row).count();
                let (first, &(_, last, wide)) = (cells[at].1, &cells[end - 1]);
                let last = last + usize::from(wide);
                let masked = mask.map(|mask| {
                    let mut shown = vec![' '; last - first + 1];
                    for (k, &(_, col, _)) in cells[at..end].iter().enumerate() {
                        shown[col - first] = mask.get(at + k).copied().unwrap_or(' ');
                    }
                    shown
                });
                out[row].push(RuleSpan { first, last, tone: found.tone, mask: masked });
                at = end;
            }
        }
    }
}

impl TerminalView {
    /// What Probes add to the rows of `snapshot`, worked out into `decor` before the rows are synced: the rules
    /// in force, the span the pointer reveals and the blocks hooks marked. With nothing in force it clears it.
    pub(super) fn probe_marks(&self, decor: &mut Decor, snapshot: &ScreenSnapshot, cx: &App) {
        let rules = crate::probe_runtime::rules(cx);
        let tab = self.tab;
        let block_tone = |block: u64| crate::probe_runtime::block_tone(tab, block, cx);
        decor.update(
            snapshot,
            &Inputs {
                rules: rules.as_deref(),
                generation: crate::probe_runtime::generation(cx),
                reveal: self.reveal.at,
                block_tone: &block_tone,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::probe::manifest::Manifest;
    use wormhole_core::probe::rules::CompiledRules;
    use wormhole_term::ScreenSnapshot;
    use wormhole_term::testing::TestTerm;

    use super::*;

    const RULES: &str = r#"
[[contributes.rules]]
id = "failed"
pattern = 'FAILED [a-z_]+'
tone = "danger"

[[contributes.rules]]
id = "key"
pattern = 'sk-[a-z0-9]{12,}'
look = "redact"
"#;

    fn rules() -> RuleSet {
        let text = format!(
            "[probe]\nid = \"tiny\"\nname = \"Tiny\"\ndescription = \"d\"\nauthor = \"a\"\n\
             version = \"0.1.0\"\nwormhole = \">=1.3\"\napi = 1\n{RULES}"
        );
        let parsed = Manifest::parse(&text).unwrap_or_else(|err| panic!("{err}"));
        RuleSet::new(CompiledRules::compile(&parsed.manifest).expect("compiles"))
    }

    fn none(_: u64) -> Option<Tone> {
        None
    }

    fn update(decor: &mut Decor, term: &mut TestTerm, snapshot: &mut ScreenSnapshot, rules: &RuleSet) {
        term.snapshot(snapshot);
        decor
            .update(snapshot, &Inputs { rules: Some(rules), generation: 1, reveal: None, block_tone: &none });
    }

    #[test]
    fn a_match_that_wraps_is_one_match_with_a_span_on_each_row() {
        let (rules, mut term) = (rules(), TestTerm::new(10, 4, 100));
        term.feed(b"ok FAILED test_one\r\nnext");
        let (mut decor, mut snapshot) = (Decor::default(), ScreenSnapshot::default());
        update(&mut decor, &mut term, &mut snapshot, &rules);
        // "ok FAILED " | "test_one" : the first row from column 3 to its end, the second to column 7.
        let spans = |row| decor.rules(row).iter().map(|s| (s.first, s.last, s.tone)).collect::<Vec<_>>();
        assert_eq!(spans(0), [(3, 9, Some(Tone::Danger))]);
        assert_eq!(spans(1), [(0, 7, Some(Tone::Danger))]);
        assert!(spans(2).is_empty());
    }

    #[test]
    fn a_redacted_span_shows_blocks_in_its_cells_and_its_text_when_revealed() {
        let (rules, mut term) = (rules(), TestTerm::new(40, 3, 100));
        term.feed(b"key sk-abcdef123456 ok");
        let (mut decor, mut snapshot) = (Decor::default(), ScreenSnapshot::default());
        update(&mut decor, &mut term, &mut snapshot, &rules);
        let span = &decor.rules(0)[0];
        assert_eq!((span.first, span.last), (4, 18));
        assert_eq!(span.mask.as_ref().map(|m| m.iter().collect::<String>()), Some("█".repeat(15)));
        term.snapshot(&mut snapshot);
        decor.update(
            &snapshot,
            &Inputs { rules: Some(&rules), generation: 1, reveal: Some((0, 4)), block_tone: &none },
        );
        assert!(
            decor.rules(0).is_empty(),
            "the pointer is over it: the text shows and the rule paints nothing else"
        );
        decor.update(
            &snapshot,
            &Inputs { rules: Some(&rules), generation: 1, reveal: Some((0, 5)), block_tone: &none },
        );
        assert!(decor.rules(0)[0].mask.is_some(), "another cell is not this span's start");
    }

    #[test]
    fn a_frame_with_no_changed_row_scans_nothing_and_scrolled_lines_are_not_scanned_again() {
        let (rules, mut term) = (rules(), TestTerm::new(30, 4, 100));
        term.feed(b"FAILED a\r\nplain\r\nFAILED b\r\n$ ");
        let (mut decor, mut snapshot) = (Decor::default(), ScreenSnapshot::default());
        update(&mut decor, &mut term, &mut snapshot, &rules);
        let first = rules.scans();
        assert_eq!(first, 4, "the four lines that have text");
        update(&mut decor, &mut term, &mut snapshot, &rules);
        assert!(rules.scans() <= first + 1, "only the cursor's row may be reported: {}", rules.scans());
        // One more line scrolls everything up: the lines are the ones the memo has, the new one is scanned.
        term.feed(b"\r\nFAILED c");
        let before = rules.scans();
        update(&mut decor, &mut term, &mut snapshot, &rules);
        assert!(
            rules.scans() - before <= 2,
            "the new line and the prompt that changed: {}",
            rules.scans() - before
        );
        assert_eq!(decor.rules(1).len(), 1, "`FAILED b` moved up a row with its span");
    }
}
