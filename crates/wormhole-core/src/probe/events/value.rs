//! The values of a hook's `match` fields: a number test, a list, a glob. Each is parsed once (by
//! `validate`, to refuse a bad one with its reason, and by the compiled hook, to run it) and tested here,
//! so the two cannot disagree on what a value means.

use std::fmt;

/// Entries in a list value, at most.
pub const MAX_LIST: usize = 16;
/// The longest text a matcher looks at, in bytes. A field longer than this is not matched at all: a
/// hook does not get to make the app scan a megabyte, and a prefix would match what the whole is not.
pub const MAX_TEXT_BYTES: usize = 4096;
/// The longest glob, in bytes.
const MAX_GLOB_BYTES: usize = 128;
/// The longest number test, in bytes.
const MAX_TEST_BYTES: usize = 24;

/// `value` split at `,`, each entry trimmed and non-empty, at most [`MAX_LIST`].
pub fn list(value: &str) -> Result<Vec<&str>, String> {
    let entries: Vec<&str> = value.split(',').map(str::trim).collect();
    if entries.iter().any(|e| e.is_empty()) {
        return Err("has an empty entry: a list is `a,b,c` with something between the commas".into());
    }
    if entries.len() > MAX_LIST {
        return Err(format!("has {} entries; at most {MAX_LIST}", entries.len()));
    }
    Ok(entries)
}

/// A comparison of a number: `>=60000`, `!=0`, a bare `3` (equal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comparison {
    op: Op,
    n: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Comparison {
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if text.len() > MAX_TEST_BYTES {
            return Err(format!("is longer than {MAX_TEST_BYTES} bytes"));
        }
        // Two-character operators first, so `<=` is not `<` and `=5`.
        let (op, rest) =
            [("!=", Op::Ne), ("<=", Op::Le), (">=", Op::Ge), ("<", Op::Lt), (">", Op::Gt), ("=", Op::Eq)]
                .into_iter()
                .find_map(|(sign, op)| text.strip_prefix(sign).map(|rest| (op, rest)))
                .unwrap_or((Op::Eq, text));
        let n = rest.trim().parse::<i64>().map_err(|_| {
            format!("`{text}` is not a number test: an optional `=`, `!=`, `<`, `<=`, `>` or `>=` and a whole number, as in `>=60000`")
        })?;
        Ok(Self { op, n })
    }

    pub fn test(self, value: i64) -> bool {
        match self.op {
            Op::Eq => value == self.n,
            Op::Ne => value != self.n,
            Op::Lt => value < self.n,
            Op::Le => value <= self.n,
            Op::Gt => value > self.n,
            Op::Ge => value >= self.n,
        }
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = match self.op {
            Op::Eq => "=",
            Op::Ne => "!=",
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Gt => ">",
            Op::Ge => ">=",
        };
        write!(f, "{sign} {}", self.n)
    }
}

/// An anchored glob: the whole text must match, `*` is any run of characters (none too) and nothing
/// else is special, so a branch `release/*` is what it looks like.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glob {
    /// The literal pieces between the stars.
    parts: Vec<String>,
    anchored_start: bool,
    anchored_end: bool,
}

impl Glob {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > MAX_GLOB_BYTES {
            return Err(format!("`{text}` is longer than {MAX_GLOB_BYTES} bytes"));
        }
        if text.chars().any(crate::probe::hidden::is_hidden) {
            return Err("must not hold control, bidi or invisible characters".into());
        }
        Ok(Self {
            parts: text.split('*').filter(|p| !p.is_empty()).map(String::from).collect(),
            anchored_start: !text.starts_with('*'),
            anchored_end: !text.ends_with('*'),
        })
    }

    /// Linear in the text: each literal piece is found once, leftmost, which leaves the most room for the
    /// pieces after it; the last of an anchored end is checked against what is left, so no character is
    /// used twice.
    pub fn is_match(&self, text: &str) -> bool {
        if text.len() > MAX_TEXT_BYTES {
            return false;
        }
        // `*` (or `**`) matches anything; an empty glob only the empty text.
        if self.parts.is_empty() {
            return text.is_empty() || !self.anchored_start;
        }
        let mut rest = text;
        for (ix, part) in self.parts.iter().enumerate() {
            let (first, last) = (ix == 0, ix + 1 == self.parts.len());
            if first && self.anchored_start {
                let Some(after) = rest.strip_prefix(part.as_str()) else { return false };
                rest = after;
                if last && self.anchored_end {
                    return rest.is_empty();
                }
            } else if last && self.anchored_end {
                return rest.ends_with(part.as_str());
            } else {
                let Some(found) = rest.find(part.as_str()) else { return false };
                rest = &rest[found + part.len()..];
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_test_has_an_operator_and_a_whole_number() {
        let at = |text: &str| Comparison::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"));
        assert!(at("!=0").test(1) && !at("!=0").test(0));
        assert!(at(">=60000").test(60000) && !at(">=60000").test(59999));
        assert!(at(">3").test(4) && !at(">3").test(3));
        assert!(at("<=3").test(3) && !at("<3").test(3));
        assert!(at("3").test(3) && at("=3").test(3) && at(" = 3 ").test(3));
        assert!(at("-1").test(-1), "a signal's exit code can be negative");
        for bad in ["", ">=", "x", ">=x", "1.5", "==1", "=>1", "1 2", &"1".repeat(30)] {
            assert!(Comparison::parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_number_test_reads_back_as_words() {
        assert_eq!(Comparison::parse(">=60000").map(|c| c.to_string()), Ok(">= 60000".into()));
        assert_eq!(Comparison::parse("0").map(|c| c.to_string()), Ok("= 0".into()));
    }

    #[test]
    fn a_list_has_no_empty_entry_and_a_cap() {
        assert_eq!(list("a, b ,c"), Ok(vec!["a", "b", "c"]));
        for bad in ["", "a,", ",a", "a,,b"] {
            assert!(list(bad).is_err(), "{bad:?}");
        }
        assert!(list(&["x"; MAX_LIST + 1].join(",")).is_err());
        assert!(list(&["x"; MAX_LIST].join(",")).is_ok());
    }

    fn glob(text: &str) -> Glob {
        Glob::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
    }

    #[test]
    fn a_glob_is_anchored_and_star_is_the_only_wildcard() {
        assert!(glob("main").is_match("main") && !glob("main").is_match("mainline"));
        assert!(!glob("main").is_match("xmain"));
        assert!(glob("release/*").is_match("release/1.2") && glob("release/*").is_match("release/"));
        assert!(!glob("release/*").is_match("hotfix/release/1"));
        assert!(glob("*-wip").is_match("a-wip") && !glob("*-wip").is_match("a-wip2"));
        assert!(glob("a*b*c").is_match("aXbYc") && glob("a*b*c").is_match("abc"));
        assert!(!glob("a*b*c").is_match("acb"));
        assert!(glob("*").is_match("anything") && glob("*").is_match(""));
        assert!(glob("*a*").is_match("bab") && !glob("*a*").is_match("bbb"));
        assert!(!glob("a?").is_match("ab"), "`?` is a literal question mark");
        assert!(glob("a?").is_match("a?"));
    }

    #[test]
    fn a_glob_piece_cannot_be_used_twice() {
        // `ab*b` needs two b's: one from the first piece, one that ends the text.
        assert!(!glob("ab*b").is_match("ab"));
        assert!(glob("ab*b").is_match("abb"));
        assert!(glob("a*a").is_match("aa") && !glob("a*a").is_match("a"));
    }

    #[test]
    fn a_glob_refuses_hidden_characters_and_long_text_is_not_matched() {
        assert!(Glob::parse("a\u{202e}b").is_err());
        assert!(Glob::parse(&"a".repeat(129)).is_err());
        assert!(!glob("*").is_match(&"a".repeat(MAX_TEXT_BYTES + 1)));
    }
}
