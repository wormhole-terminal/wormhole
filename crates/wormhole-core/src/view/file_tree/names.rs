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
    let (mut left, mut right) = (a, b);
    loop {
        let (Some(x), Some(y)) = (left.chars().next(), right.chars().next()) else {
            // One ended: it comes first; both: they are the same by this order, so by their bytes.
            return match (left.is_empty(), right.is_empty()) {
                (true, true) => a.cmp(b),
                (true, false) => Ordering::Less,
                _ => Ordering::Greater,
            };
        };
        if x.is_ascii_digit() && y.is_ascii_digit() {
            let (number_x, rest_x) = split_digits(left);
            let (number_y, rest_y) = split_digits(right);
            // More digits is a bigger number; the same number of digits compares as text.
            let (digits_x, digits_y) = (number_x.trim_start_matches('0'), number_y.trim_start_matches('0'));
            match digits_x.len().cmp(&digits_y.len()).then_with(|| digits_x.cmp(digits_y)) {
                Ordering::Equal => {}
                other => return other,
            }
            (left, right) = (rest_x, rest_y);
        } else {
            match x.to_lowercase().cmp(y.to_lowercase()) {
                Ordering::Equal => {}
                other => return other,
            }
            (left, right) = (&left[x.len_utf8()..], &right[y.len_utf8()..]);
        }
    }
}

/// The run of ASCII digits at the front of `text`, and what follows it: slices, nothing copied.
fn split_digits(text: &str) -> (&str, &str) {
    text.split_at(text.bytes().take_while(u8::is_ascii_digit).count())
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
