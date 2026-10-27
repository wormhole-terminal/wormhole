//! A commit message as the History tab handles it: kept as git gave it for copying (capped), and for the
//! screen cleaned by one rule of what it must not receive, split into the subject and the blocks of its
//! body, with its `Co-authored-by:` trailers read out.
//! Pure text, so it is tested without a repository.

/// How much of a commit's message is kept; the rest is cut and
/// [`CommitDetails::message_cut`](super::CommitDetails::message_cut) says so. The message of an
/// ordinary commit is a few hundred bytes.
pub const MESSAGE_CAP: usize = 64 * 1024;

/// Trailers and co-authors listed at most.
const LIST_CAP: usize = 32;

/// The message as git gave it, for copying: capped, with Windows line ends made plain and the end
/// trimmed, and nothing else touched. Whether it was cut.
pub(crate) fn normalize(message: &str) -> (String, bool) {
    let cut = message.len() > MESSAGE_CAP;
    let kept = if cut { &message[..message.floor_char_boundary(MESSAGE_CAP)] } else { message };
    (kept.replace("\r\n", "\n").trim_end().to_string(), cut)
}

/// Characters that draw nothing yet are part of how text reads, so a screen keeps them: the joiners
/// that make an emoji sequence (a person at a computer, a flag with a tag, a skin tone) one picture, the
/// variation selectors, and the left-to-right and right-to-left marks.
fn is_joiner(c: char) -> bool {
    matches!(c, '\u{200C}'..='\u{200F}' | '\u{FE00}'..='\u{FE0F}' | '\u{E0100}'..='\u{E01EF}')
}

/// What the one rule for text from a repository does with `c` on a screen: nothing for what draws nothing
/// or turns the text round (zero-width spaces, bidirectional overrides and isolates, tags), a visible
/// replacement for a control character (an escape sequence, say), a line break and a tab as they are in
/// `multiline` text and a space in one line. The rule of the list's rows, the heading and the body alike.
fn shown(c: char, multiline: bool) -> Option<char> {
    if wormhole_term::is_invisible(c) && !is_joiner(c) {
        None
    } else if c == '\n' || c == '\t' {
        Some(if multiline { c } else { ' ' })
    } else if c.is_control() {
        Some('\u{fffd}')
    } else {
        Some(c)
    }
}

/// One line of a name, an address, a ref or a subject as a screen shows it: at most `max` characters.
pub(crate) fn display_line(text: &str, max: usize) -> String {
    text.chars().filter_map(|c| shown(c, false)).take(max).collect()
}

/// A message or a body as a screen shows it.
pub(crate) fn display_text(text: &str) -> String {
    text.chars().filter_map(|c| shown(c, true)).collect()
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
            if ix == last && is_trailer_block(&lines, ix > 0) {
                Block::Trailers(text)
            } else if lines.iter().any(|line| keeps_its_break(line)) {
                Block::Lines(text)
            } else {
                Block::Paragraph(lines.iter().map(|line| line.trim()).collect::<Vec<_>>().join(" "))
            }
        })
        .collect()
}

/// The keys that are trailers wherever they stand, even when they are all a body has.
const KNOWN_TRAILERS: [&str; 14] = [
    "signed-off-by",
    "co-authored-by",
    "reviewed-by",
    "acked-by",
    "tested-by",
    "reported-by",
    "suggested-by",
    "helped-by",
    "cc",
    "fixes",
    "closes",
    "resolves",
    "change-id",
    "bug",
];

/// The last paragraph is trailers when every line reads `Key: value` and either something comes before it
/// (`after_prose`) or the keys are ones that only trailers use. A body that is one `Note: this is why`
/// is a sentence, not a trailer.
fn is_trailer_block(lines: &[&str], after_prose: bool) -> bool {
    lines.iter().all(|line| is_trailer(line))
        && (after_prose
            || lines.iter().all(|line| {
                let key = line.split_once(':').map_or("", |(key, _)| key);
                KNOWN_TRAILERS.iter().any(|known| key.eq_ignore_ascii_case(known))
            }))
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

/// How many cells wide `text` is on a screen: wide characters (Chinese, Korean, most emoji) are two,
/// combining and joining ones none, the rest one. What a line's room is counted in, so a line of
/// Chinese is not taken for half as long as it is.
fn cells(text: &str) -> usize {
    text.chars().map(|c| usize::from(wormhole_term::cell_width(c))).sum()
}

/// The start of `line` that is at most `budget` cells wide.
fn take_cells(line: &str, budget: usize) -> &str {
    let mut used = 0;
    for (at, c) in line.char_indices() {
        used += usize::from(wormhole_term::cell_width(c));
        if used > budget {
            return &line[..at];
        }
    }
    line
}

/// How many lines of its first paragraph a commit's body shows while it is compact.
pub const COMPACT_LINES: usize = 3;

/// How much of a body is drawn whole (its text, and its trailers apart): a long message is for
/// copying, the screen shows the start of it and says so.
pub const DISPLAY_CAP: usize = 6_000;

/// How many lines of a body are drawn whole: a message of thousands of one-line paragraphs is cut here,
/// whatever its length in characters.
pub const DISPLAY_LINES: usize = 80;

/// A commit's body ready to draw, made once when the commit's details are read: the compact view's
/// text, the whole view's text (the blocks joined, so a body of a thousand paragraphs is not a thousand
/// elements), and what is in neither.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Body {
    /// The first block, for the compact view: empty when the body is empty or starts with its trailers.
    pub first: String,
    /// How many cells wide each of the first lines of that block is (at most [`COMPACT_LINES`] + 1 are
    /// kept): what the compact view's height is estimated from.
    first_lines: Vec<usize>,
    /// Every block but the trailers, apart by a blank line, cut at [`DISPLAY_CAP`] characters.
    pub text: String,
    /// The closing trailers, cut at the same.
    pub trailers: String,
    /// Some of `text` or `trailers` was cut.
    pub cut: bool,
    /// There is a second block or there are trailers.
    others: bool,
}

impl Body {
    /// The body `body` (see [`body`]) as it is drawn.
    pub fn of(body: &str) -> Self {
        let blocks = blocks(body);
        let first = blocks.first().filter(|block| !matches!(block, Block::Trailers(_)));
        let first_lines = first
            .map(|block| match block {
                Block::Paragraph(text) | Block::Lines(text) | Block::Trailers(text) => {
                    text.split('\n').take(COMPACT_LINES + 1).map(cells).collect()
                }
            })
            .unwrap_or_default();
        let (mut text, mut trailers) = (Vec::new(), Vec::new());
        for block in &blocks {
            match block {
                Block::Paragraph(t) | Block::Lines(t) => text.push(t.as_str()),
                Block::Trailers(t) => trailers.push(t.as_str()),
            }
        }
        let (text, cut_text) = cap(&text.join("\n\n"), DISPLAY_CAP, DISPLAY_LINES);
        let (trailers, cut_trailers) = cap(&trailers.join("\n"), DISPLAY_CAP / 4, DISPLAY_LINES / 4);
        Self {
            first: first
                .map(|block| match block {
                    Block::Paragraph(t) | Block::Lines(t) | Block::Trailers(t) => {
                        cap(t, DISPLAY_CAP, DISPLAY_LINES).0
                    }
                })
                .unwrap_or_default(),
            first_lines,
            text,
            trailers,
            cut: cut_text || cut_trailers,
            others: blocks.len() > 1 || (first.is_none() && !blocks.is_empty()),
        }
    }

    /// Whether the body has nothing to show.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty() && self.trailers.is_empty()
    }

    /// Whether the compact view leaves anything out, for lines of about `line_chars` characters: a second
    /// block, trailers, or a first block that wraps to more than [`COMPACT_LINES`] lines (`line_chars` is a
    /// line's room in cells). The wrapping is
    /// an estimate (the view wraps by pixels), made to err towards "more": an offer that hides nothing
    /// costs a click, a missing one hides text.
    pub fn more(&self, line_chars: usize) -> bool {
        let line_chars = line_chars.max(1);
        let lines: usize = self.first_lines.iter().map(|len| len.div_ceil(line_chars).max(1)).sum();
        self.others || lines > COMPACT_LINES
    }
}

impl Body {
    /// The compact view's text for lines of about `line_chars` characters: the first block up to
    /// [`COMPACT_LINES`] lines, an ellipsis where it was cut (at a word when there is one near). Made
    /// here, by the same estimate as [`Body::more`], rather than by the view clamping its lines: a clamp
    /// makes the text system lay the whole block out again every frame.
    pub fn compact(&self, line_chars: usize) -> String {
        let line_chars = line_chars.max(2);
        let mut left = COMPACT_LINES;
        let mut out = String::new();
        let mut lines = self.first.split('\n').peekable();
        while let Some(line) = lines.next() {
            let needs = cells(line).div_ceil(line_chars).max(1);
            if !out.is_empty() {
                out.push('\n');
            }
            if needs <= left {
                out.push_str(line);
                left -= needs;
                if left == 0 && lines.peek().is_some() {
                    out.push('\u{2026}');
                    break;
                }
                continue;
            }
            // This line does not fit what is left: cut it, at a word when one is in the last fifth.
            let keep = take_cells(line, left * line_chars - 1);
            let at = keep.rfind(' ').filter(|&at| at * 5 >= keep.len() * 4).unwrap_or(keep.len());
            out.push_str(keep[..at].trim_end());
            out.push('\u{2026}');
            break;
        }
        out
    }
}

/// `text` cut to `max` characters and `lines` lines (on a character), and whether it was.
fn cap(text: &str, max: usize, lines: usize) -> (String, bool) {
    let by_chars = text.char_indices().nth(max).map(|(at, _)| at);
    let by_lines = text.match_indices('\n').nth(lines.saturating_sub(1)).map(|(at, _)| at);
    match by_chars.into_iter().chain(by_lines).min() {
        Some(at) => (text[..at].trim_end().to_string(), true),
        None => (text.to_string(), false),
    }
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
            let name = display_line(name.trim(), 256);
            (!name.is_empty()).then(|| Contact { name, email: display_line(email.trim(), 256) })
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
    fn what_is_copied_is_what_git_gave_and_what_is_shown_is_cleaned_by_one_rule() {
        let raw = "\x1b[31mred\x1b[0m\r\nline\ttwo\u{202e}x\x07\n\n";
        let (copy, cut) = normalize(raw);
        assert_eq!(copy, "\x1b[31mred\x1b[0m\nline\ttwo\u{202e}x\x07");
        assert!(!cut);
        assert_eq!(display_text(&copy), "\u{fffd}[31mred\u{fffd}[0m\nline\ttwox\u{fffd}");
        // One line: the controls are marked, a break or a tab is a space, a name is cut to its length.
        assert_eq!(display_line("A\x1b]0;x\x07da\tb", 256), "A\u{fffd}]0;x\u{fffd}da b");
        assert_eq!(display_line("abcdef", 3), "abc");
    }

    #[test]
    fn emoji_sequences_survive_and_what_turns_text_round_does_not() {
        let person_at_a_computer = "\u{1F9D1}\u{200D}\u{1F4BB}";
        let flag = "\u{1F1EF}\u{1F1F5}";
        let thumbs_up_medium_skin = "\u{1F44D}\u{1F3FD}";
        let heart = "\u{2764}\u{FE0F}";
        for sequence in [person_at_a_computer, flag, thumbs_up_medium_skin, heart] {
            assert_eq!(display_line(sequence, 40), sequence, "{sequence:?}");
            assert_eq!(display_text(&format!("a {sequence} b")), format!("a {sequence} b"));
        }
        // The row and the heading are the same text, so are the copies of before and after.
        assert_eq!(
            display_line(&format!("{person_at_a_computer} fix"), 40),
            display_text(&format!("{person_at_a_computer} fix"))
        );
        // Bidirectional overrides and isolates, zero-width spaces and tags are gone.
        assert_eq!(display_line("a\u{202e}b\u{2066}c\u{200b}d\u{e0041}e", 40), "abcde");
        // The marks that only direct a language's text stay.
        assert_eq!(display_line("a\u{200f}b", 40), "a\u{200f}b");
        // Copying never touches them.
        assert_eq!(normalize(person_at_a_computer).0, person_at_a_computer);
    }

    #[test]
    fn a_huge_message_is_cut_on_a_character() {
        let (text, cut) = normalize(&format!("subject\n\n{}", "é".repeat(MESSAGE_CAP)));
        assert!(cut);
        assert!(text.len() <= MESSAGE_CAP);
        assert!(text.starts_with("subject\n\néé"));
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
        // A key with a space in it is a sentence, and a paragraph of trailers that is not the last is prose.
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
    fn a_body_is_drawn_from_its_first_block_and_all_of_it_in_one_piece() {
        let body = Body::of(
            "First paragraph,\nwrapped.\n\nSecond.\n\n- a list\n- of two\n\nFixes: #1\nCc: Bob <b@x.y>",
        );
        assert_eq!(body.first, "First paragraph, wrapped.");
        assert_eq!(body.text, "First paragraph, wrapped.\n\nSecond.\n\n- a list\n- of two");
        assert_eq!(body.trailers, "Fixes: #1\nCc: Bob <b@x.y>");
        assert!(!body.cut);
        assert!(body.more(80), "more blocks follow");
        assert!(Body::of("").is_empty() && Body::default().is_empty());
        assert!(!Body::of("").more(40));
    }

    #[test]
    fn a_short_single_paragraph_needs_no_toggle_and_a_long_one_does() {
        assert!(!Body::of(&"x".repeat(100)).more(40), "three lines of forty");
        assert!(Body::of(&"x".repeat(121)).more(40), "a fourth line");
        // A list counts its own line breaks, each wrapped on its own.
        assert!(Body::of("- a\n- b\n- c\n- d").more(40));
        assert!(!Body::of("- a\n- b\n- c").more(40));
        let _ = Body::of("x").more(0); // a zero width never panics
    }

    #[test]
    fn the_compact_text_is_three_lines_and_an_ellipsis() {
        let line = |n: usize| "word ".repeat(n).trim_end().to_string();
        // Fits: as it is.
        assert_eq!(Body::of("short text").compact(40), "short text");
        // 200 characters in lines of 40 take five: cut inside the third, at a word.
        let long = Body::of(&line(40));
        let compact = long.compact(40);
        assert!(compact.ends_with('\u{2026}'));
        assert!(compact.chars().count() <= 3 * 40, "{}", compact.chars().count());
        assert!(!compact.trim_end_matches('\u{2026}').ends_with(' '));
        // A list keeps its breaks and is cut after its third line (or where a long line runs out).
        let list = Body::of("- one\n- two\n- three\n- four\n- five");
        assert_eq!(list.compact(40), "- one\n- two\n- three\u{2026}");
        let wrapped = Body::of(&format!("- {}\n- b\n- c", "x".repeat(70)));
        assert_eq!(wrapped.compact(40).lines().count(), 2, "the first item takes two of the three lines");
        // Nothing to show compact for a body of trailers, and no panic for odd widths.
        assert_eq!(Body::of("Fixes: #1").compact(40), "");
        let _ = Body::of("x y z").compact(0);
        let _ = Body::of("x y z").compact(1);
    }

    #[test]
    fn wide_characters_take_two_cells_of_a_lines_room() {
        // Forty Chinese characters are two lines of forty cells, sixty-one are four.
        assert!(!Body::of(&"字".repeat(40)).more(40));
        assert!(!Body::of(&"字".repeat(60)).more(40), "three lines exactly");
        assert!(Body::of(&"字".repeat(61)).more(40));
        let cut = Body::of(&"字".repeat(100)).compact(40);
        assert!(cut.ends_with('\u{2026}'));
        assert!(cells(&cut) <= 3 * 40, "{} cells", cells(&cut));
        // Wide emoji and Hangul count the same; a combining mark and a joiner take no room.
        assert_eq!(cells("\u{1F4BB}\u{AC00}"), 4);
        assert_eq!(cells("e\u{0301}\u{200D}"), 1);
    }

    #[test]
    fn trailers_alone_are_left_out_of_the_compact_view_and_a_note_is_not_trailers() {
        let trailers = Body::of("Signed-off-by: Ada <a@b.c>\nFixes: #4");
        assert_eq!(trailers.first, "");
        assert!(trailers.more(80), "something to show, but not compact");
        assert_eq!(
            (trailers.text.as_str(), trailers.trailers.as_str()),
            ("", "Signed-off-by: Ada <a@b.c>\nFixes: #4")
        );
        // One paragraph that happens to read `Note: …` is the body, shown compact.
        let note = Body::of("Note: this is why.");
        assert_eq!(note.first, "Note: this is why.");
        assert!(!note.more(80));
        assert_eq!(blocks("Note: this is why."), [Block::Paragraph("Note: this is why.".into())]);
        // After prose any `Key: value` closing paragraph is trailers, as git reads them.
        assert_eq!(
            blocks("Why.\n\nNote: this is why."),
            [Block::Paragraph("Why.".into()), Block::Trailers("Note: this is why.".into())]
        );
    }

    #[test]
    fn what_is_drawn_is_bounded_however_long_the_message() {
        let many: String = (0..5_000).map(|n| format!("paragraph {n}\n\n")).collect();
        let body = Body::of(&many);
        assert!(body.cut);
        assert!(body.text.chars().count() <= DISPLAY_CAP);
        assert!(body.text.lines().count() <= DISPLAY_LINES, "{}", body.text.lines().count());
        assert!(body.more(80));
        // Invisible characters are gone from what a repository sends.
        assert_eq!(display_text("a\u{202e}b\u{200b}c"), "abc");
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
