//! A release's notes as lines a window can draw: headings, list items and paragraphs.
//!
//! The feed carries Markdown, written by hand or taken from the changelog. There is no Markdown
//! engine here, and none is wanted for what release notes use: this reads the block level (`#`
//! headings, `-`/`*`/`+` and `1.` items, blank lines between paragraphs) and strips the inline marks
//! (`**bold**`, `*em*`, `` `code` ``, `[text](url)`) down to their text, so nothing shows its syntax.
//! A link's address is dropped on purpose: the feed is unsigned, so a window must not offer to open
//! what it names. Anything else is text. Input is capped, as the feed is hostile input.

/// The most of the notes read; the feed's own cap is larger, a window shows no more than this.
const MAX_LINES: usize = 200;
/// The most of a line kept.
const MAX_LINE_CHARS: usize = 600;

/// One line of notes, its marks stripped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// A `#` heading of any level.
    Heading(String),
    /// A list item.
    Item(String),
    /// A paragraph's line.
    Text(String),
}

/// `notes` as drawable lines, blank lines and empty items dropped.
pub fn read_notes(notes: &str) -> Vec<Note> {
    notes
        .lines()
        .filter_map(|raw| {
            let line = raw.trim();
            let note = if let Some(heading) = heading(line) {
                Note::Heading(plain(heading))
            } else if let Some(item) = item(line) {
                Note::Item(plain(item))
            } else {
                Note::Text(plain(line))
            };
            let (Note::Heading(text) | Note::Item(text) | Note::Text(text)) = &note;
            (!text.is_empty()).then_some(note)
        })
        .take(MAX_LINES)
        .collect()
}

fn heading(line: &str) -> Option<&str> {
    let rest = line.trim_start_matches('#');
    let hashes = line.len() - rest.len();
    ((1..=6).contains(&hashes) && (rest.is_empty() || rest.starts_with(' '))).then(|| rest.trim())
}

fn item(line: &str) -> Option<&str> {
    for mark in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(mark) {
            return Some(rest.trim());
        }
        // A bare marker (the line was trimmed): an empty item, dropped by the caller.
        if line == mark.trim() {
            return Some("");
        }
    }
    // "1. " and "12) ": digits, a dot or a bracket, a space.
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    let rest = &line[digits..];
    (digits > 0 && digits <= 3)
        .then(|| rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")))
        .flatten()
        .map(str::trim)
}

/// `text` without its inline marks: emphasis and code marks removed, `[text](url)` as `text`.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(ch) = rest.chars().next() {
        if ch == '['
            && let Some((label, after)) = link(rest)
        {
            out.push_str(&plain(label));
            rest = after;
            continue;
        }
        if matches!(ch, '*' | '`') || (ch == '_' && emphasis_underscore(&out, rest)) {
            rest = &rest[ch.len_utf8()..];
            continue;
        }
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out.chars().take(MAX_LINE_CHARS).collect()
}

/// `[label](url)` at the start of `rest`: the label and what follows the link.
fn link(rest: &str) -> Option<(&str, &str)> {
    let close = rest.find("](")?;
    let label = &rest[1..close];
    let after_url = rest[close + 2..].find(')')?;
    (!label.contains('[') && !label.contains(']')).then(|| (label, &rest[close + 2 + after_url + 1..]))
}

/// An `_` marks emphasis only at a word's edge: `snake_case` keeps its underscores.
fn emphasis_underscore(before: &str, rest: &str) -> bool {
    let prev = before.chars().next_back();
    let next = rest[1..].chars().next();
    let word = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    !(word(prev) && word(next))
}

#[cfg(test)]
mod tests {
    use super::{Note, read_notes};

    fn heading(text: &str) -> Note {
        Note::Heading(text.into())
    }
    fn item(text: &str) -> Note {
        Note::Item(text.into())
    }
    fn text(text: &str) -> Note {
        Note::Text(text.into())
    }

    #[test]
    fn headings_items_and_paragraphs_are_told_apart() {
        let notes = "## Added\n\n- Faster **redraws**\n* A `config.json` key\n+ Third\n1. First step\n12) Twelfth\nPlain words.\n";
        assert_eq!(
            read_notes(notes),
            vec![
                heading("Added"),
                item("Faster redraws"),
                item("A config.json key"),
                item("Third"),
                item("First step"),
                item("Twelfth"),
                text("Plain words."),
            ]
        );
    }

    #[test]
    fn a_link_reads_as_its_label_and_never_its_address() {
        assert_eq!(
            read_notes("- See [the guide](https://evil.example/x) now"),
            vec![item("See the guide now")]
        );
        assert_eq!(read_notes("- broken [link]( no end"), vec![item("broken [link]( no end")]);
    }

    #[test]
    fn what_is_not_a_mark_is_kept() {
        assert_eq!(
            read_notes("#nospace\n-nospace\n2024. year"),
            vec![text("#nospace"), text("-nospace"), text("2024. year")]
        );
        assert_eq!(read_notes("- use snake_case and _this_"), vec![item("use snake_case and this")]);
        assert_eq!(read_notes("####### seven"), vec![text("####### seven")]);
    }

    #[test]
    fn nothing_is_left_of_blank_and_empty_lines() {
        assert!(read_notes("").is_empty());
        assert!(read_notes("  \n\n## \n- \n").is_empty());
    }

    #[test]
    fn a_hostile_feed_cannot_make_an_endless_page() {
        let many = "- x\n".repeat(10_000);
        assert_eq!(read_notes(&many).len(), 200);
        let long = format!("- {}", "y".repeat(5_000));
        assert_eq!(read_notes(&long), vec![item(&"y".repeat(600))]);
    }
}
