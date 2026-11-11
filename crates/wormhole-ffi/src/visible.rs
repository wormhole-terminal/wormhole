//! A line of code as the diff and the file viewer draw it (`wormhole_core::view::visible`): tabs spelled as spaces and what a
//! person cannot see, or what would change how the line reads (a control, a bidirectional override: "Trojan Source"), shown as a
//! visible mark. One rule for both apps. Offsets are in unicode scalars, which is what Swift can follow without trusting its own
//! grapheme clusters (a combining mark after a control joins the mark when drawn; scalars never overrun).

use wormhole_core::view::visible::visible;

/// Where a placeholder (or the elision) is in the drawn text, in scalars: drawn dimmed.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VisibleMark {
    pub start: u32,
    pub end: u32,
}

/// `text` as drawn. `offsets` has one entry per scalar of the original (and one for its end): where it starts in `text`, in
/// scalars; empty when nothing moved (a line without a tab or a placeholder).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VisibleLine {
    pub text: String,
    pub offsets: Vec<u32>,
    pub marks: Vec<VisibleMark>,
}

#[uniffi::export]
pub fn visible_line(text: String, tab_width: u32) -> VisibleLine {
    let shown = visible(&text, tab_width as usize);
    if shown.offsets.is_none() {
        return VisibleLine { text, offsets: Vec::new(), marks: Vec::new() };
    }
    // The drawn text's scalar index at every byte.
    let mut scalar_at = vec![0u32; shown.text.len() + 1];
    let mut count = 0;
    for (byte, ch) in shown.text.char_indices() {
        scalar_at[byte..=byte + ch.len_utf8() - 1].iter_mut().for_each(|slot| *slot = count);
        count += 1;
    }
    scalar_at[shown.text.len()] = count;
    let mut offsets: Vec<u32> = text.char_indices().map(|(byte, _)| scalar_at[shown.offset(byte)]).collect();
    offsets.push(count);
    let marks = shown.marks.iter().map(|m| VisibleMark { start: scalar_at[m.start], end: scalar_at[m.end] }).collect();
    VisibleLine { text: shown.text.into_owned(), offsets, marks }
}

#[cfg(test)]
mod tests {
    use super::visible_line;

    #[test]
    fn a_plain_line_is_unchanged_and_unmapped() {
        let line = visible_line("let x = 1;".into(), 4);
        assert_eq!((line.text.as_str(), line.offsets.len(), line.marks.len()), ("let x = 1;", 0, 0));
    }

    #[test]
    fn hostile_lines_map_in_scalars_without_overrunning() {
        // A control, a tab and a bidi override, each followed by a combining mark (the cases that merged graphemes in Swift).
        for original in ["# \u{1}\u{301}", "#\t\u{301}", "a\u{202E}\u{301}b"] {
            let line = visible_line(original.into(), 4);
            let total = line.text.chars().count() as u32;
            assert_eq!(line.offsets.len(), original.chars().count() + 1);
            assert!(line.offsets.iter().all(|&o| o <= total), "{original:?}");
            assert!(line.offsets.windows(2).all(|w| w[0] <= w[1]), "{original:?}");
            assert!(line.marks.iter().all(|m| m.start <= m.end && m.end <= total), "{original:?}");
        }
        let line = visible_line("if x\u{202E} \u{2066}{".into(), 4);
        assert_eq!(line.text, "if x<U+202E> <U+2066>{");
        assert_eq!((line.marks[0].start, line.marks[0].end), (4, 12));
        assert_eq!(line.offsets[5], 12);
    }

    #[test]
    fn tabs_go_to_the_next_stop() {
        let line = visible_line("\ta".into(), 4);
        assert_eq!(line.text, "    a");
        assert_eq!(line.offsets, [0, 4, 5]);
    }
}
