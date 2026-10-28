//! The explanation behind a warning's ⓘ, as far as it is data: the reason a thing failed, as the popover
//! shows it. The words around it are the catalog's. A reason is what an error said, and nothing a
//! terminal showed: it is cleaned to one short line, so it can be read, selected and reported.

/// The longest reason the popover shows, in characters.
pub const REASON_MAX: usize = 240;

/// How much of the start of a cut reason is kept; the rest of the room is its end, where an error chain
/// (`context: cause: root cause`) says what really went wrong.
const HEAD: usize = REASON_MAX / 3;

/// `error` as one short line: control characters and runs of whitespace become one space, invisible
/// formatting characters (a bidi override, a zero-width joiner) are dropped, and a reason longer than
/// [`REASON_MAX`] keeps its start and its end with "…" between. Empty when the error said nothing.
#[must_use]
pub fn reason(error: &str) -> String {
    let mut line = String::with_capacity(error.len().min(REASON_MAX * 4));
    let mut gap = false;
    for c in error.chars() {
        if c.is_whitespace() || c.is_control() {
            gap = !line.is_empty();
        } else if !wormhole_term::is_invisible(c) {
            if std::mem::take(&mut gap) {
                line.push(' ');
            }
            line.push(c);
        }
    }
    let count = line.chars().count();
    if count <= REASON_MAX {
        return line;
    }
    let head: String = line.chars().take(HEAD).collect();
    let tail: String = line.chars().skip(count - (REASON_MAX - HEAD - 1)).collect();
    format!("{}…{}", head.trim_end(), tail.trim_start())
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
    fn invisible_formatting_characters_are_dropped() {
        // A right-to-left override would make the end of the line read backwards; a joiner hides nothing.
        assert_eq!(reason("exe\u{202E}txt\u{200B}.log"), "exetxt.log");
        assert_eq!(reason("a\u{2066} b"), "a b");
    }

    #[test]
    fn a_long_reason_is_cut_in_the_middle_and_keeps_its_root_cause() {
        let chain = format!("{}: the root cause is HERE", "context".repeat(60));
        let cut = reason(&chain);
        assert!(cut.chars().count() <= REASON_MAX, "{}", cut.chars().count());
        assert!(cut.starts_with("contextcontext"), "the start stays: {cut}");
        assert!(cut.ends_with("the root cause is HERE"), "the end stays: {cut}");
        assert!(cut.contains('…'));
    }

    #[test]
    fn a_short_one_is_kept_whole_and_an_empty_one_stays_empty() {
        assert_eq!(reason("spawn failed"), "spawn failed");
        assert_eq!(reason("  \n "), "");
        let exact = "x".repeat(REASON_MAX);
        assert_eq!(reason(&exact), exact, "right at the cap is not cut");
    }
}
