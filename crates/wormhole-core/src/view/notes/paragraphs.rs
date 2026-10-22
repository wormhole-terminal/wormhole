//! Text rules for notes: which paragraph ⌘↵ sends, a note's title and body, and when a note counts as
//! empty.

/// Whether text handed over in `chunks` is only whitespace. Editors keep their text in a rope,
/// whose chunks can be read where it is, instead of copying the whole text into a `String`
/// first. A character is never split between chunks, so this agrees with `trim().is_empty()` of the whole text.
pub fn chunks_blank<'a>(mut chunks: impl Iterator<Item = &'a str>) -> bool {
    chunks.all(|chunk| chunk.trim().is_empty())
}

/// The paragraph (the lines around the cursor, up to the nearest blank lines) containing byte
/// offset `cursor`. On a blank line itself there is none: sending then must send nothing, not
/// the paragraphs on either side.
pub fn paragraph_at(text: &str, cursor: usize) -> String {
    let mut cursor = cursor.min(text.len());
    while !text.is_char_boundary(cursor) {
        cursor -= 1;
    }
    let blank = |line: &str| line.trim().is_empty();
    let line_start = text[..cursor].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[cursor..].find('\n').map_or(text.len(), |i| cursor + i);
    if blank(&text[line_start..line_end]) {
        return String::new();
    }
    // Grow over the non-blank lines above and below; `start`/`end` sit on line boundaries.
    let mut start = line_start;
    while start > 0 {
        let above_end = start - 1;
        let above_start = text[..above_end].rfind('\n').map_or(0, |i| i + 1);
        if blank(&text[above_start..above_end]) {
            break;
        }
        start = above_start;
    }
    let mut end = line_end;
    while end < text.len() {
        let below_start = end + 1;
        let below_end = text[below_start..].find('\n').map_or(text.len(), |i| below_start + i);
        if blank(&text[below_start..below_end]) {
            break;
        }
        end = below_end;
    }
    text[start..end].to_string()
}

/// A title from the note's first non-empty line.
pub fn note_title(text: &str) -> Option<String> {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty())?;
    Some(line.trim_start_matches(['#', '-', '*', '>', ' ']).to_string())
}

/// What an open note shows under its title: the text after the line the title
/// comes from, without the blank lines around it. `None` for a note that is its title alone,
/// which then shows its one line once.
pub fn note_body(text: &str) -> Option<&str> {
    let mut rest = text;
    // Past the title line (the first with something on it) and the line break after it.
    loop {
        let (line, after) = rest.split_once('\n').unwrap_or((rest, ""));
        let titled = !line.trim().is_empty();
        rest = after;
        if titled || rest.is_empty() {
            break;
        }
    }
    while let Some((line, after)) = rest.split_once('\n') {
        if !line.trim().is_empty() {
            break;
        }
        rest = after;
    }
    let body = rest.trim_end();
    (!body.trim().is_empty()).then_some(body)
}

/// For each merged note, the existing note (index into `existing`) that shows it from now on: one
/// with exactly that text if there is one, else the next unused one in order (an edited note),
/// else none (a new note). Positions alone would hand a note's editor to its neighbour whenever a
/// note is added or removed above it.
pub fn note_slots(existing: &[String], merged: &[String]) -> Vec<Option<usize>> {
    let mut used = vec![false; existing.len()];
    let mut slots: Vec<Option<usize>> = merged
        .iter()
        .map(|text| {
            let found = (0..existing.len()).find(|&i| !used[i] && existing[i] == text.trim());
            if let Some(i) = found {
                used[i] = true;
            }
            found
        })
        .collect();
    let mut free = (0..existing.len()).filter(|&i| !used[i]).collect::<Vec<_>>().into_iter();
    for slot in slots.iter_mut().filter(|slot| slot.is_none()) {
        *slot = free.next();
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::{chunks_blank, note_body, note_slots, note_title, paragraph_at};

    #[test]
    fn a_single_paragraph_is_sent_whole_from_anywhere_in_it() {
        let text = "first line\nsecond line";
        for cursor in [0, 5, 10, 11, text.len()] {
            assert_eq!(paragraph_at(text, cursor), text, "cursor {cursor}");
        }
        assert_eq!(paragraph_at("", 0), "");
    }

    #[test]
    fn paragraph_expands_over_non_blank_lines() {
        let text = "p1\n\na\nb\nc\n\np3";
        // Anywhere in the middle paragraph (offsets 4..=8), first or last line included.
        for cursor in 4..=8 {
            assert_eq!(paragraph_at(text, cursor), "a\nb\nc", "cursor {cursor}");
        }
        // The ends of the outer paragraphs stop at the blank line next to them.
        assert_eq!(paragraph_at(text, 2), "p1");
        assert_eq!(paragraph_at(text, text.len()), "p3");
        assert_eq!(paragraph_at(text, 11), "p3");
    }

    #[test]
    fn paragraph_on_a_blank_line_is_empty() {
        // The cursor on the empty line between two paragraphs sends neither of them.
        let text = "p1\n\np2\n\np3";
        assert_eq!(paragraph_at(text, 3), "");
        assert_eq!(paragraph_at(text, 7), "");
        // Several blank lines in a row are all blank.
        let text = "p1\n\n\np2";
        assert_eq!(paragraph_at(text, 3), "");
        assert_eq!(paragraph_at(text, 4), "");
        assert_eq!(paragraph_at(text, 0), "p1");
        assert_eq!(paragraph_at(text, 5), "p2");
        // A line of spaces separates paragraphs like an empty one.
        assert_eq!(paragraph_at("a\n  \nb", 0), "a");
        assert_eq!(paragraph_at("a\n  \nb", 3), "");
        assert_eq!(paragraph_at("a\n  \nb", 5), "b");
    }

    #[test]
    fn paragraph_handles_multibyte_text_and_stray_cursors() {
        let text = "héllo\n\nwörld ✓";
        // Inside the two-byte `é` (offset 2): rounded down, still the first paragraph.
        assert_eq!(paragraph_at(text, 2), "héllo");
        // Inside the three-byte `✓`, and far past the end.
        assert_eq!(paragraph_at(text, text.len() - 1), "wörld ✓");
        assert_eq!(paragraph_at(text, 10_000), "wörld ✓");
    }

    #[test]
    fn blankness_agrees_with_the_whole_text_however_it_is_chunked() {
        let texts = ["", " ", "\n\t \n", "x", "  x  ", "\u{a0}\u{2003}", "\u{a0}y", "héllo", "a\n\n"];
        for text in texts {
            // Every way of cutting the text at character boundaries.
            let cuts: Vec<usize> = (0..=text.len()).filter(|&i| text.is_char_boundary(i)).collect();
            for &cut in &cuts {
                let chunks = [&text[..cut], &text[cut..]];
                assert_eq!(chunks_blank(chunks.into_iter()), text.trim().is_empty(), "{text:?} cut at {cut}");
            }
        }
        assert!(chunks_blank(std::iter::empty()));
    }

    #[test]
    fn a_note_is_titled_by_its_first_non_empty_line() {
        assert_eq!(note_title("# Plan\nbody").as_deref(), Some("Plan"));
        assert_eq!(note_title("\n  \n- item one\ntwo").as_deref(), Some("item one"));
        assert_eq!(note_title("> quoted").as_deref(), Some("quoted"));
        assert_eq!(note_title("   \n\n"), None);
        assert_eq!(note_title(""), None);
    }

    #[test]
    fn an_open_note_shows_the_lines_after_its_title() {
        // "Sidebar ideas" is the title; the text under it starts with the next line.
        let note = "Sidebar ideas\nDrop the idle line.\nBranch as a chip.\n\nOne lead column.\n";
        assert_eq!(note_body(note), Some("Drop the idle line.\nBranch as a chip.\n\nOne lead column."));
        // Blank lines before the title, or between it and the text, aren't shown.
        assert_eq!(note_body("\n  \nTitle\n\n\nbody\n"), Some("body"));
        // A note of one line shows it once, however it is padded.
        assert_eq!(note_body("buy milk"), None);
        assert_eq!(note_body("buy milk\n"), None);
        assert_eq!(note_body("\nbuy milk\n\n  \n"), None);
        assert_eq!(note_body(""), None);
        assert_eq!(note_body("\n\n"), None);
    }

    fn texts(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn notes_keep_their_editors_by_text() {
        let existing = texts(&["a", "b", "c"]);
        // An external insert above: every note keeps its own editor.
        assert_eq!(note_slots(&existing, &texts(&["new", "a", "b", "c"])), [None, Some(0), Some(1), Some(2)]);
        // A removal and a reorder.
        assert_eq!(note_slots(&existing, &texts(&["c", "a"])), [Some(2), Some(0)]);
        // An edited note takes the editor that is left over.
        assert_eq!(note_slots(&existing, &texts(&["a", "b, edited", "c"])), [Some(0), Some(1), Some(2)]);
        // Duplicates are matched one to one.
        assert_eq!(note_slots(&texts(&["x", "x"]), &texts(&["x", "x", "x"])), [Some(0), Some(1), None]);
    }
}
