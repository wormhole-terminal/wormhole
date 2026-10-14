//! How many cells the terminal draws a piece of text in. The only way to ask is the cursor: go to
//! column 1, write the text, ask where the cursor is. The distance it moved is the drawn width.
//! Unicode's grapheme clusters decide what the width should be; a terminal with mode 2027 counts
//! them that way, one without counts code points, and the two disagree on emoji sequences.

use std::io::{self, Write};

use super::model::{Decision, Trace, WidthPass, WidthResult};
use super::query::Prober;
use crate::term::{Cursor, escape};

pub struct Case {
    pub name: &'static str,
    pub text: &'static str,
    /// Cells, per Unicode 15.1/16 grapheme clusters.
    pub expected: u8,
    /// Not counted: Unicode, fonts and terminals don't agree on it.
    pub disputed: bool,
}

const fn case(name: &'static str, text: &'static str, expected: u8) -> Case {
    Case { name, text, expected, disputed: false }
}

pub const CASES: [Case; 20] = [
    case("ASCII letter", "a", 1),
    case("e + combining acute", "e\u{301}", 1),
    case("CJK ideograph", "\u{4e2d}", 2),
    case("halfwidth katakana", "\u{ff71}", 1),
    case("Hangul syllable", "\u{d55c}", 2),
    case("Hangul jamo sequence", "\u{1112}\u{1161}\u{11ab}", 2),
    case("emoji face", "\u{1f600}", 2),
    case("red heart + VS16", "\u{2764}\u{fe0f}", 2),
    case("warning sign", "\u{26a0}", 1),
    case("warning sign + VS16", "\u{26a0}\u{fe0f}", 2),
    case("thumbs up + skin tone", "\u{1f44d}\u{1f3fd}", 2),
    case("family (ZWJ sequence)", "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}", 2),
    case("flag of Japan", "\u{1f1ef}\u{1f1f5}", 2),
    case("keycap 1", "1\u{fe0f}\u{20e3}", 2),
    case("skin tone modifier alone", "\u{1f3fd}", 2),
    case("CJK extension B", "\u{2000b}", 2),
    case("right arrow", "\u{2192}", 1),
    case("box drawing line", "\u{2500}", 1),
    Case { name: "Devanagari conjunct", text: "\u{915}\u{94d}\u{937}\u{93f}", expected: 2, disputed: true },
    case("zero width joiner alone", "\u{200d}", 0),
];

/// `1F468 200D 1F469`
pub fn codepoints(text: &str) -> String {
    text.chars().map(|c| format!("{:X}", c as u32)).collect::<Vec<_>>().join(" ")
}

/// The row of the scratch line: the second, away from anything at the top edge.
const ROW: u16 = 2;

/// The width from the two cursor reports (before and after the text), and what looked wrong.
pub fn width_from(reports: &[Cursor]) -> (Option<u8>, Vec<String>) {
    let mut notes = Vec::new();
    let [before, after] = reports else {
        notes.push(format!("{} of 2 cursor reports came back", reports.len()));
        return (None, notes);
    };
    if before.col != 1 || before.row != ROW {
        notes.push(format!(
            "the cursor was at row {} column {} before the text, not {ROW};1",
            before.row, before.col
        ));
    }
    if after.row != before.row {
        notes
            .push(format!("the cursor moved from row {} to row {}: the text wrapped", before.row, after.row));
        return (None, notes);
    }
    let w = after.col.saturating_sub(before.col);
    (Some(w.min(255) as u8), notes)
}

fn measure_one<W: Write>(p: &mut Prober<'_, W>, case: &Case) -> io::Result<WidthPass> {
    let mut request = format!("\x1b[{ROW};1H\x1b[2K\x1b[6n").into_bytes();
    request.extend_from_slice(case.text.as_bytes());
    request.extend_from_slice(b"\x1b[6n");
    let sent = escape(&request);
    let Some(got) = p.cursor(&request, 2)? else {
        return Ok(WidthPass {
            width: None,
            dsr_before: String::new(),
            dsr_after: String::new(),
            trace: Trace { sent, ..Trace::without_query(Decision::Skipped) },
        });
    };
    p.send(format!("\x1b[{ROW};1H\x1b[2K").as_bytes())?;
    let (width, mut notes) = width_from(&got.reports);
    if !got.stale.is_empty() {
        notes.push(format!("{} byte(s) were already waiting: {}", got.stale.len(), escape(&got.stale)));
    }
    let decided = match (&width, got.reports.len()) {
        (Some(_), _) => Decision::Reply,
        (None, 2) => Decision::Unparsed,
        (None, _) if got.raw.is_empty() => Decision::Timeout,
        (None, _) => Decision::Unparsed,
    };
    Ok(WidthPass {
        width,
        dsr_before: got.reports.first().map(|c| escape(&c.raw)).unwrap_or_default(),
        dsr_after: got.reports.get(1).map(|c| escape(&c.raw)).unwrap_or_default(),
        trace: Trace {
            sent,
            received: escape(&got.raw),
            micros: got.reports.last().map(|c| c.micros),
            decided,
            notes,
        },
    })
}

/// Measures every case as the terminal is, then again with mode 2027 set when the terminal knows
/// it (`mode_2027` is DECRPM's value: 1 set, 2 reset, 3 always set), and puts the mode back.
pub fn measure<W: Write>(p: &mut Prober<'_, W>, mode_2027: Option<u8>) -> io::Result<Vec<WidthResult>> {
    let mut results = Vec::new();
    for c in &CASES {
        results.push(WidthResult {
            name: c.name.to_string(),
            codepoints: codepoints(c.text),
            expected: c.expected,
            disputed: c.disputed,
            as_is: measure_one(p, c)?,
            with_2027: None,
        });
    }
    if matches!(mode_2027, Some(1..=3)) {
        p.send(b"\x1b[?2027h")?;
        for (c, r) in CASES.iter().zip(&mut results) {
            r.with_2027 = Some(measure_one(p, c)?);
        }
        if mode_2027 == Some(2) {
            p.send(b"\x1b[?2027l")?;
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cur(row: u16, col: u16) -> Cursor {
        Cursor { row, col, raw: format!("\x1b[{row};{col}R").into_bytes(), micros: 100 }
    }

    #[test]
    fn the_width_is_how_far_the_cursor_moved() {
        assert_eq!(width_from(&[cur(2, 1), cur(2, 3)]), (Some(2), vec![]));
        assert_eq!(width_from(&[cur(2, 1), cur(2, 1)]), (Some(0), vec![]));
    }

    #[test]
    fn odd_reports_are_noted() {
        let (w, notes) = width_from(&[cur(2, 5), cur(2, 7)]);
        assert_eq!(w, Some(2));
        assert!(notes[0].contains("column 5"), "{notes:?}");
        let (w, notes) = width_from(&[cur(2, 1), cur(3, 1)]);
        assert_eq!(w, None);
        assert!(notes[0].contains("wrapped"), "{notes:?}");
        let (w, notes) = width_from(&[cur(2, 1)]);
        assert_eq!(w, None);
        assert!(notes[0].contains("1 of 2"), "{notes:?}");
    }

    #[test]
    fn the_cases_are_what_they_say() {
        assert_eq!(CASES.iter().filter(|c| c.disputed).count(), 1);
        assert_eq!(codepoints("\u{1f468}\u{200d}\u{1f469}"), "1F468 200D 1F469");
        for c in &CASES {
            assert!(c.expected <= 2 && !c.text.is_empty(), "{}", c.name);
        }
        let mut names: Vec<&str> = CASES.iter().map(|c| c.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), CASES.len());
    }
}
