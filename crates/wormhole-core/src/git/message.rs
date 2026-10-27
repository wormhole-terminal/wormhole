//! A commit message as the History tab shows it: cleaned of what a screen must not receive, capped,
//! split into the subject and the blocks of its body, with its `Co-authored-by:` trailers read out.
//! Pure text, so it is tested without a repository.

/// How much of a commit's message is kept; the rest is cut and
/// [`CommitDetails::message_cut`](super::CommitDetails::message_cut) says so. The message of an
/// ordinary commit is a few hundred bytes.
pub const MESSAGE_CAP: usize = 64 * 1024;

/// Trailers and co-authors listed at most.
const LIST_CAP: usize = 32;

/// The message as it is shown and copied: capped, with Windows line ends made plain and control
/// characters (the escape sequences of a hostile message) replaced. Whether it was cut.
pub(crate) fn clean(message: &str) -> (String, bool) {
    let cut = message.len() > MESSAGE_CAP;
    let kept = if cut { &message[..floor_char_boundary(message, MESSAGE_CAP)] } else { message };
    let text: String = kept
        .replace("\r\n", "\n")
        .chars()
        .map(|c| if c.is_control() && c != '\n' && c != '\t' { '\u{fffd}' } else { c })
        .collect();
    (text.trim_end().to_string(), cut)
}

/// The largest index not over `at` that is the start of a character.
fn floor_char_boundary(text: &str, at: usize) -> usize {
    (0..=at.min(text.len())).rev().find(|&ix| text.is_char_boundary(ix)).unwrap_or(0)
}

/// One line of a name or an address: no control characters, at most `max` characters.
pub(crate) fn plain(text: &str, max: usize) -> String {
    text.chars().filter(|c| !c.is_control()).take(max).collect()
}

/// The first line.
pub(crate) fn subject(message: &str) -> &str {
    message.lines().next().unwrap_or("")
}

/// Everything after the subject line, without the blank lines around it.
pub(crate) fn body(message: &str) -> &str {
    message.split_once('\n').map_or("", |(_, rest)| rest).trim_matches('\n')
}

/// One paragraph of a message body, as it should be drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// Prose, hard-wrapped at 72 columns by habit: its lines are joined, so it wraps to the panel.
    Paragraph(String),
    /// A list, a quote or code, whose line breaks mean something and are kept.
    Lines(String),
    /// The closing `Key: value` lines (`Signed-off-by:`, `Co-authored-by:`, `Fixes:`), kept as lines.
    Trailers(String),
}

/// The blocks of `body` (see [`body`]), in order.
pub(crate) fn blocks(body: &str) -> Vec<Block> {
    let paragraphs: Vec<Vec<&str>> = body
        .split("\n\n")
        .map(|paragraph| paragraph.split('\n').collect::<Vec<_>>())
        .filter(|lines| lines.iter().any(|line| !line.trim().is_empty()))
        .collect();
    let last = paragraphs.len().saturating_sub(1);
    paragraphs
        .into_iter()
        .enumerate()
        .map(|(ix, lines)| {
            let text = lines.join("\n");
            if ix == last && lines.iter().all(|line| is_trailer(line)) {
                Block::Trailers(text)
            } else if lines.iter().any(|line| keeps_its_break(line)) {
                Block::Lines(text)
            } else {
                Block::Paragraph(lines.iter().map(|line| line.trim()).collect::<Vec<_>>().join(" "))
            }
        })
        .collect()
}

/// `Signed-off-by: Ada <a@b.c>`, `Fixes: #12`: a key of letters, digits and dashes, a colon and a value.
fn is_trailer(line: &str) -> bool {
    let Some((key, value)) = line.split_once(':') else { return false };
    !key.is_empty()
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && value.starts_with(' ')
        && !value.trim().is_empty()
}

/// A line whose break is not just where the author's editor wrapped: indented, a list item, a quote,
/// a fence or a heading.
fn keeps_its_break(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.len() != line.len() {
        return true;
    }
    let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
    let numbered =
        digits > 0 && trimmed[digits..].starts_with(['.', ')']) && trimmed[digits + 1..].starts_with(' ');
    numbered || ["- ", "* ", "+ ", "> ", "```", "# ", "|"].iter().any(|marker| trimmed.starts_with(marker))
}

/// A name, and the address it was given with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contact {
    pub name: String,
    pub email: String,
}

/// The `Co-authored-by: Name <email>` trailers: the message's last paragraph, where git reads
/// trailers, whatever the case the key is written in. A line that doesn't read as a name with an
/// address is skipped.
pub(crate) fn co_authors(message: &str) -> Vec<Contact> {
    let Some(last) = message.trim_end().rsplit("\n\n").next() else { return Vec::new() };
    last.lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            if !key.trim().eq_ignore_ascii_case("co-authored-by") {
                return None;
            }
            let (name, rest) = value.trim().rsplit_once('<')?;
            let email = rest.strip_suffix('>')?;
            let name = plain(name.trim(), 256);
            (!name.is_empty()).then(|| Contact { name, email: plain(email.trim(), 256) })
        })
        .take(LIST_CAP)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_splits_into_subject_and_body() {
        assert_eq!(subject("fix: a\n\nWhy.\n\nMore."), "fix: a");
        assert_eq!(body("fix: a\n\nWhy.\n\nMore."), "Why.\n\nMore.");
        assert_eq!((subject("only"), body("only")), ("only", ""));
        assert_eq!((subject(""), body("")), ("", ""));
        assert_eq!(body("a\nb"), "b", "no blank line: the second line is the body");
    }

    #[test]
    fn control_characters_never_reach_the_screen() {
        let (text, cut) = clean("\x1b[31mred\x1b[0m\r\nline\ttwo\x07\n\n");
        assert_eq!(text, "\u{fffd}[31mred\u{fffd}[0m\nline\ttwo\u{fffd}");
        assert!(!cut);
        assert_eq!(plain("A\x1b]0;x\x07da", 256), "A]0;xda");
        assert_eq!(plain("abcdef", 3), "abc");
    }

    #[test]
    fn a_huge_message_is_cut_on_a_character() {
        let (text, cut) = clean(&format!("subject\n\n{}", "é".repeat(MESSAGE_CAP)));
        assert!(cut);
        assert!(text.len() <= MESSAGE_CAP);
        assert!(text.starts_with("subject\n\néé"));
        assert_eq!(floor_char_boundary("aé", 2), 1, "inside the two-byte é: back to its start");
    }

    #[test]
    fn prose_is_unwrapped_and_lists_are_not() {
        let body = "First line of a hard-wrapped\nparagraph that goes on.\n\n\
                    - one item\n- another\n  continued\n\n\
                    1. numbered\n2) also\n\n\
                    > quoted\n\n\
                    Reviewed-by: Bob <bob@example.com>\nFixes: #12";
        assert_eq!(
            blocks(body),
            [
                Block::Paragraph("First line of a hard-wrapped paragraph that goes on.".into()),
                Block::Lines("- one item\n- another\n  continued".into()),
                Block::Lines("1. numbered\n2) also".into()),
                Block::Lines("> quoted".into()),
                Block::Trailers("Reviewed-by: Bob <bob@example.com>\nFixes: #12".into()),
            ]
        );
        assert!(blocks("").is_empty());
        assert!(blocks("\n\n \n\n").is_empty());
        // A sentence with a colon is not a trailer, and neither is a middle paragraph of them.
        assert_eq!(blocks("Note: this is why."), [Block::Trailers("Note: this is why.".into())]);
        assert_eq!(
            blocks("Why it matters: a lot, really.\n\nFixes: #1\n\nclosing words"),
            [
                Block::Paragraph("Why it matters: a lot, really.".into()),
                Block::Paragraph("Fixes: #1".into()),
                Block::Paragraph("closing words".into()),
            ]
        );
    }

    #[test]
    fn co_authors_come_from_the_trailers() {
        let message = "feat: pair\n\nBody with Co-authored-by: Not Me <no@example.com> inside.\n\n\
                       Signed-off-by: Ada <ada@example.com>\n\
                       co-authored-by: Bob Builder <bob@example.com>\n\
                       Co-Authored-By: Cy <cy@example.com>\n\
                       Co-authored-by: broken line\n";
        let found: Vec<_> = co_authors(message).into_iter().map(|c| (c.name, c.email)).collect();
        assert_eq!(
            found,
            [("Bob Builder".into(), "bob@example.com".into()), ("Cy".into(), "cy@example.com".into())]
        );
        // Not in the last paragraph: not a trailer.
        assert!(co_authors("a\n\nCo-authored-by: X <x@y.z>\n\nlast words").is_empty());
        assert!(co_authors("").is_empty());
    }
}
