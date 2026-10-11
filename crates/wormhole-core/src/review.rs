//! Review comments on a diff, and the message they make for the coding agent in the terminal.
//!
//! Reading what an agent changed is where parallel agents slow down: the person sees a line that
//! is wrong and has to describe where it is all over again in the agent's prompt. Here a comment
//! is pinned to a line of a file's diff as it is read in the diff view, comments gather per
//! terminal across files, and [`Review::message`] turns them into one message that names each
//! place (`src/orders.rs:42`), quotes the line and says what the person wrote. The app pastes it
//! into the terminal's prompt, and only into a program that takes a paste as text (bracketed
//! paste): anywhere else each line would run as a command. The quoted line comes from the
//! repository, which may be anyone's, so the message carries no control characters at all.

/// Which side of the diff a line is on: the new file's (an added or unchanged line) or the old
/// file's (a removed line, which has no number in the new file).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineSide {
    Old,
    New,
}

/// The longest stretch of a line quoted in the message; a longer one is cut with an ellipsis.
const QUOTE_MAX_CHARS: usize = 160;

/// One comment: where it is and what it says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewComment {
    /// The file's path in the repository, as the diff names it.
    pub path: String,
    pub side: LineSide,
    /// The line's number on its side.
    pub line: u32,
    /// The line as the diff showed it, quoted in the message.
    pub code: String,
    pub text: String,
}

impl ReviewComment {
    /// Where it is, as an agent reads a place: "src/orders.rs:42", and for a removed line, which
    /// the file no longer has, "src/orders.rs:40 (removed line)".
    pub fn location(&self) -> String {
        match self.side {
            LineSide::New => format!("{}:{}", self.path, self.line),
            LineSide::Old => format!("{}:{} (removed line)", self.path, self.line),
        }
    }

    fn quote(&self) -> String {
        let code = printable(&self.code, false);
        let code = code.trim();
        if code.chars().count() <= QUOTE_MAX_CHARS {
            return code.to_string();
        }
        let cut: String = code.chars().take(QUOTE_MAX_CHARS).collect();
        format!("{}…", cut.trim_end())
    }
}

/// A terminal's comments, in the order they were written. One per line: commenting on a line again
/// replaces what it said.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Review {
    comments: Vec<ReviewComment>,
}

impl Review {
    /// Add `comment`, or replace the one on the same line; an empty text removes that one.
    pub fn put(&mut self, comment: ReviewComment) {
        let at = self.position(&comment.path, comment.side, comment.line);
        match (at, comment.text.trim().is_empty()) {
            (Some(ix), true) => {
                self.comments.remove(ix);
            }
            (Some(ix), false) => self.comments[ix] = comment,
            (None, true) => {}
            (None, false) => self.comments.push(comment),
        }
    }

    /// The comment on that line, if there is one.
    pub fn on(&self, path: &str, side: LineSide, line: u32) -> Option<&ReviewComment> {
        self.position(path, side, line).map(|ix| &self.comments[ix])
    }

    fn position(&self, path: &str, side: LineSide, line: u32) -> Option<usize> {
        self.comments.iter().position(|c| c.path == path && c.side == side && c.line == line)
    }

    /// The lines of `path` that have a comment, for the diff view to mark.
    pub fn lines_of<'a>(&'a self, path: &'a str) -> impl Iterator<Item = (LineSide, u32)> + 'a {
        self.comments.iter().filter(move |c| c.path == path).map(|c| (c.side, c.line))
    }

    pub fn len(&self) -> usize {
        self.comments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.comments.is_empty()
    }

    pub fn clear(&mut self) {
        self.comments.clear();
    }

    /// "1 comment", "3 comments on 2 files".
    pub fn summary(&self) -> String {
        let mut files: Vec<&str> = self.comments.iter().map(|c| c.path.as_str()).collect();
        files.sort_unstable();
        files.dedup();
        let comments = if self.comments.len() == 1 {
            "1 comment".to_string()
        } else {
            format!("{} comments", self.comments.len())
        };
        if files.len() > 1 { format!("{comments} on {} files", files.len()) } else { comments }
    }

    /// The message for the agent: a line saying what follows, then each comment numbered, with its
    /// place, the line it is on quoted, and its text. `None` without comments.
    pub fn message(&self) -> Option<String> {
        if self.comments.is_empty() {
            return None;
        }
        let mut out = String::from("Review comments on your changes. Please address each one:\n");
        for (ix, comment) in self.comments.iter().enumerate() {
            out.push('\n');
            out.push_str(&format!("{}. {}\n", ix + 1, comment.location()));
            let quote = comment.quote();
            if !quote.is_empty() {
                out.push_str(&format!("   > {quote}\n"));
            }
            for line in printable(&comment.text, true).trim().lines() {
                out.push_str("   ");
                out.push_str(line.trim_end());
                out.push('\n');
            }
        }
        Some(out.trim_end().to_string())
    }
}

/// `text` without control characters (C0, DEL and C1: escape sequences, the bell, a carriage
/// return or ^C that a terminal would act on); a tab becomes a space, and line breaks stay only
/// in a comment's own text (`lines`).
fn printable(text: &str, lines: bool) -> String {
    text.chars()
        .filter_map(|c| match c {
            '\n' if lines => Some('\n'),
            '\t' => Some(' '),
            c if c.is_control() => None,
            c => Some(c),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{LineSide, QUOTE_MAX_CHARS, Review, ReviewComment};

    fn comment(path: &str, side: LineSide, line: u32, code: &str, text: &str) -> ReviewComment {
        ReviewComment { path: path.into(), side, line, code: code.into(), text: text.into() }
    }

    #[test]
    fn the_message_names_each_place_quotes_its_line_and_says_what_was_written() {
        let mut review = Review::default();
        review.put(comment("src/orders.rs", LineSide::New, 42, "    let total = 0;", "Use a checked add."));
        review.put(comment(
            "src/api.rs",
            LineSide::Old,
            7,
            "validate(req)?;",
            "Why did this go?\nWe need it.",
        ));
        assert_eq!(review.summary(), "2 comments on 2 files");
        assert_eq!(
            review.message().as_deref(),
            Some(
                "Review comments on your changes. Please address each one:\n\
                 \n\
                 1. src/orders.rs:42\n   > let total = 0;\n   Use a checked add.\n\
                 \n\
                 2. src/api.rs:7 (removed line)\n   > validate(req)?;\n   Why did this go?\n   We need it."
            )
        );
    }

    #[test]
    fn a_line_has_one_comment_and_an_empty_one_takes_it_away() {
        let mut review = Review::default();
        assert_eq!(review.message(), None);
        review.put(comment("a.rs", LineSide::New, 1, "x", "first"));
        review.put(comment("a.rs", LineSide::New, 1, "x", "second"));
        review.put(comment("a.rs", LineSide::Old, 1, "y", "the old side is another line"));
        assert_eq!(review.len(), 2);
        assert_eq!(review.summary(), "2 comments");
        assert_eq!(review.on("a.rs", LineSide::New, 1).map(|c| c.text.as_str()), Some("second"));
        let mut marked: Vec<_> = review.lines_of("a.rs").collect();
        marked.sort_by_key(|(side, line)| (*side == LineSide::New, *line));
        assert_eq!(marked, [(LineSide::Old, 1), (LineSide::New, 1)]);
        review.put(comment("a.rs", LineSide::New, 1, "x", "   "));
        assert_eq!(review.len(), 1);
        review.put(comment("b.rs", LineSide::New, 3, "z", ""));
        assert_eq!(review.len(), 1, "an empty comment on a new line adds nothing");
        assert_eq!(review.summary(), "1 comment");
        review.clear();
        assert!(review.is_empty() && review.message().is_none());
    }

    /// A repository's line or a comment can't reach the terminal as anything but text.
    #[test]
    fn the_message_carries_no_control_characters() {
        let mut review = Review::default();
        let hostile = "x;\x1b[201~ rm -rf ~\r\x03\x07\u{9b}31m\tend";
        review.put(comment("a.rs", LineSide::New, 1, hostile, "fix\r\x1b]0;title\x07 this\nplease"));
        let message = review.message().expect("message");
        assert!(!message.chars().any(|c| c.is_control() && c != '\n'), "{message:?}");
        assert!(message.contains("   > x;[201~ rm -rf ~31m end\n"), "{message}");
        assert!(message.ends_with("   fix]0;title this\n   please"), "{message}");
    }

    #[test]
    fn a_long_line_is_quoted_in_part_and_a_blank_one_not_at_all() {
        let long = "x".repeat(QUOTE_MAX_CHARS + 40);
        let mut review = Review::default();
        review.put(comment("a.rs", LineSide::New, 1, &long, "too long"));
        review.put(comment("a.rs", LineSide::New, 2, "   ", "on a blank line"));
        let message = review.message().expect("message");
        assert!(message.contains(&format!("   > {}…\n", "x".repeat(QUOTE_MAX_CHARS))), "{message}");
        assert!(message.contains("2. a.rs:2\n   on a blank line"), "{message}");
    }
}
