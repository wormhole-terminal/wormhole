//! The explanation behind a warning's ⓘ, as far as it is data: the reason a thing failed, as the popover
//! shows it. The words around it are the catalog's. A reason is what an error said, and nothing a
//! terminal showed: it is cleaned to one short line, so it can be read, selected and reported.

/// The longest reason the popover shows, in characters.
pub const REASON_MAX: usize = 240;

/// `error` as one short line: control characters and runs of whitespace become one space, and a reason
/// longer than [`REASON_MAX`] is cut with "…". Empty when the error said nothing.
#[must_use]
pub fn reason(error: &str) -> String {
    let mut line = String::with_capacity(error.len().min(REASON_MAX + 4));
    for word in error.split(|c: char| c.is_whitespace() || c.is_control()).filter(|word| !word.is_empty()) {
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if line.chars().count() > REASON_MAX {
        line = line.chars().take(REASON_MAX).collect::<String>().trim_end().to_string();
        line.push('…');
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reason_is_one_line() {
        assert_eq!(
            reason("could not start: no such file\n\tat /x/y"),
            "could not start: no such file at /x/y"
        );
        assert_eq!(reason("a\u{7}b\r\nc"), "a b c", "control characters never reach the popover");
    }

    #[test]
    fn a_long_reason_is_cut() {
        let cut = reason(&"word ".repeat(200));
        assert!(cut.chars().count() <= REASON_MAX + 1, "{}", cut.chars().count());
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn a_short_one_is_kept_whole_and_an_empty_one_stays_empty() {
        assert_eq!(reason("spawn failed"), "spawn failed");
        assert_eq!(reason("  \n "), "");
    }
}
