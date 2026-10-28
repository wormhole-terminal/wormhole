//! How a name from the disk is shown and ordered. A name is chosen by whoever made the file (a cloned
//! repository, a download), so it is cleaned before it is drawn, and ordered the way a person reads it.

use std::cmp::Ordering;

/// What stands in the place of a character that can't be shown.
const REPLACEMENT: char = '\u{FFFD}';

/// `name` as it is drawn: a control character (a newline in a name would make the row two lines) and
/// an invisible formatting one (a bidi override makes `exe.txt` read as `txt.exe`) become the
/// replacement character, so the name shows that something is there. Everything else is kept,
/// including the full extension.
pub fn clean(name: &str) -> String {
    name.chars().map(|c| if hidden(c) { REPLACEMENT } else { c }).collect()
}

fn hidden(c: char) -> bool {
    c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') || wormhole_term::is_invisible(c)
}

/// Case-insensitive natural order: `file2` before `file10`, `A` and `a` together. Names that are the
/// same up to case and leading zeros still order the same way every time (by their bytes), so a sort
/// never shuffles them.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut left, mut right) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (left.peek().copied(), right.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let (x, y) = (digits(&mut left), digits(&mut right));
                let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
                // More digits is a bigger number; the same number of digits compares as text.
                match x.len().cmp(&y.len()).then_with(|| x.cmp(y)) {
                    Ordering::Equal => {}
                    other => return other,
                }
            }
            (Some(x), Some(y)) => {
                left.next();
                right.next();
                match x.to_lowercase().cmp(y.to_lowercase()) {
                    Ordering::Equal => {}
                    other => return other,
                }
            }
        }
    }
}

/// Takes a run of ASCII digits off the front.
fn digits(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut run = String::new();
    while let Some(c) = chars.next_if(char::is_ascii_digit) {
        run.push(c);
    }
    run
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::{clean, natural_cmp};

    fn sorted(names: &[&str]) -> Vec<String> {
        let mut names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
        names.sort_by(|a, b| natural_cmp(a, b));
        names
    }

    #[test]
    fn numbers_in_a_name_are_read_as_numbers() {
        assert_eq!(sorted(&["file10", "file2", "file1"]), ["file1", "file2", "file10"]);
        assert_eq!(sorted(&["v1.10", "v1.9", "v1.2"]), ["v1.2", "v1.9", "v1.10"]);
        assert_eq!(
            sorted(&["a007", "a7", "a8"]),
            ["a007", "a7", "a8"],
            "equal numbers fall back to the bytes"
        );
    }

    #[test]
    fn case_does_not_split_the_alphabet() {
        assert_eq!(sorted(&["banana", "Apple", "cherry", "apple"]), ["Apple", "apple", "banana", "cherry"]);
        assert_eq!(
            sorted(&[".zshrc", "Makefile", ".git", "README"]),
            [".git", ".zshrc", "Makefile", "README"]
        );
    }

    #[test]
    fn a_prefix_comes_first_and_the_order_is_total() {
        assert_eq!(natural_cmp("a", "a.txt"), Ordering::Less);
        assert_eq!(natural_cmp("a", "a"), Ordering::Equal);
        assert_ne!(natural_cmp("A", "a"), Ordering::Equal, "never a tie between different names");
        assert_eq!(natural_cmp("a", "A"), natural_cmp("a", "A"));
    }

    #[test]
    fn a_huge_number_does_not_overflow() {
        let big = "x99999999999999999999999999999999999999";
        assert_eq!(natural_cmp(big, "x1"), Ordering::Greater);
    }

    #[test]
    fn controls_and_invisible_marks_are_shown_as_the_replacement_character() {
        assert_eq!(clean("a\nb\tc\u{7f}"), "a\u{FFFD}b\u{FFFD}c\u{FFFD}");
        // A right-to-left override would make this read as `gpj.exe`.
        assert_eq!(clean("photo\u{202e}gpj.exe"), "photo\u{FFFD}gpj.exe");
        assert_eq!(clean("zero\u{200b}width"), "zero\u{FFFD}width");
        assert_eq!(clean("line\u{2028}sep"), "line\u{FFFD}sep");
    }

    #[test]
    fn an_ordinary_name_is_kept_with_its_whole_extension() {
        for name in ["archive.tar.gz", "Ünïcode ファイル.txt", ".env.local", "no extension", "a b.md"] {
            assert_eq!(clean(name), name);
        }
    }
}
