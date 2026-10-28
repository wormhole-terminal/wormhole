//! The grey completion after the caret, as fish draws it.
//!
//! It completes the selected row, not the longest prefix the matches share. The row is what ↩ would pick and
//! ↑ ↓ change it, so the ghost is always the answer to "what happens if I accept?", where a shared prefix is
//! usually just what is typed already, or something no row is. And it is shown only when the row's text begins
//! with what is typed (ignoring case), because a completion has to continue the text: a row that matched
//! `tmnl` by its letters has nothing to continue, and ⇥ still goes into it.

/// What follows `typed` in `completion`, when `completion` begins with `typed` (ignoring case) and goes on
/// past it. Nothing for an empty `typed`: the field's placeholder is there.
pub fn ghost(typed: &str, completion: &str) -> Option<String> {
    if typed.is_empty() {
        return None;
    }
    let mut rest = completion.chars();
    for wanted in typed.chars() {
        let c = rest.next()?;
        if c != wanted && !c.to_lowercase().eq(wanted.to_lowercase()) {
            return None;
        }
    }
    let rest: String = rest.collect();
    (!rest.is_empty()).then_some(rest)
}

#[cfg(test)]
mod tests {
    use super::ghost;

    #[test]
    fn it_is_what_the_completion_adds() {
        assert_eq!(ghost("pro", "Projects/").as_deref(), Some("jects/"));
        assert_eq!(ghost("~/Pr", "~/Projects/").as_deref(), Some("ojects/"));
        assert_eq!(ghost("src/", "src/ui/").as_deref(), Some("ui/"));
    }

    #[test]
    fn nothing_when_it_does_not_continue_the_text() {
        assert_eq!(ghost("tmnl", "terminal/"), None);
        assert_eq!(ghost("pr/te", "Projects/terminal/"), None);
        assert_eq!(ghost("longer-than", "long/"), None);
    }

    #[test]
    fn nothing_when_there_is_nothing_to_add_or_nothing_typed() {
        assert_eq!(ghost("src/", "src/"), None);
        assert_eq!(ghost("", "src/"), None);
    }

    #[test]
    fn wide_characters_count_as_characters() {
        assert_eq!(ghost("é", "Éclair/").as_deref(), Some("clair/"));
    }
}
