//! A rule's regular expression: whether it may be run at all, and what its captures can be.
//!
//! The `regex` crate is the engine (linear time, no look-around or back-references), so a pattern
//! cannot be a ReDoS by construction. What is left to check is cost and meaning, from the parsed
//! form (`regex-syntax`'s AST for the flags written, its HIR for what the pattern can match):
//!
//! - at most 1 KiB, and the compiled program at most 256 KiB;
//! - never able to match the empty string (a rule that matches everywhere marks everything);
//! - the flags `i` and `u` (so `(?-u:\b)` can ask for an ASCII word boundary); `m`, `s`, `x`, `U`, `R` are
//!   refused. `u` is not restricted further: a `str` regex refuses a pattern that can match invalid UTF-8;
//! - a Unicode `\b` is a warning, for it makes the engine's lazy DFA give up on non-ASCII text.
//!
//! [`Pattern::capture_hazard`] is the anchoring check for a capture that becomes a command's
//! argument (section 3.1 of the design): see there for exactly what it accepts.

use regex_syntax::ast::{self, Ast, Flag, FlagsItemKind, GroupKind};
use regex_syntax::hir::{Class, Hir, HirKind, Look};

use super::Findings;

/// A pattern's length, in bytes.
pub(crate) const MAX_PATTERN_BYTES: usize = 1024;
/// What the compiled program may take (`RegexBuilder::size_limit`).
pub(crate) const SIZE_LIMIT: usize = 256 * 1024;
/// How deeply groups may nest: a pattern is a line of text, not a program.
const NEST_LIMIT: u32 = 32;
/// The longest a capture that becomes a command's argument may be, in bytes. The rule only sees 2 KiB
/// of a line anyway; this keeps an argument to something a person would call an argument.
pub(crate) const MAX_ARGUMENT_BYTES: usize = 1024;

/// A pattern that passed the checks.
#[derive(Debug)]
pub(crate) struct Pattern {
    /// Capture groups, not counting the whole match.
    pub groups: usize,
    hir: Hir,
}

/// Why a capture is not safe to hand to a program as an argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptureHazard {
    /// It can be longer than the limit, or without one.
    Unbounded,
    /// Its first character can be `-`, so it can be read as an option.
    MayStartWithDash,
    /// It can be empty, or not take part in a match at all (it is in an alternative or an optional
    /// part), and an empty argument is not a value.
    MayBeEmpty,
}

/// Check `pattern` (the manifest field `path`) and report what is wrong; the parsed pattern when it
/// can be used.
pub(crate) fn analyze(pattern: &str, path: &str, out: &mut Findings) -> Option<Pattern> {
    if pattern.len() > MAX_PATTERN_BYTES {
        out.error(
            "pattern-too-long",
            path,
            format!("{} bytes; a pattern may have {MAX_PATTERN_BYTES}", pattern.len()),
        );
        return None;
    }
    let tree = match ast::parse::ParserBuilder::new().nest_limit(NEST_LIMIT).build().parse(pattern) {
        Ok(tree) => tree,
        Err(err) => {
            out.error("pattern-syntax", path, short(&err.to_string()));
            return None;
        }
    };
    let before = out.errors();
    let Ok(flags) = ast::visit(&tree, FlagVisitor(Vec::new()));
    for flag in flags {
        out.error(
            "pattern-flag",
            path,
            format!("the flag `{flag}` is not allowed: only `i`, and `u` (as in `(?-u:\\b)`)"),
        );
    }
    let hir = match regex_syntax::ParserBuilder::new().nest_limit(NEST_LIMIT).build().parse(pattern) {
        Ok(hir) => hir,
        Err(err) => {
            out.error("pattern-syntax", path, short(&err.to_string()));
            return None;
        }
    };
    match hir.properties().minimum_len() {
        Some(0) => out.error(
            "pattern-empty-match",
            path,
            "can match the empty string, so it would match everywhere; require at least one character",
        ),
        None => out.error("pattern-never-matches", path, "can never match anything"),
        Some(_) => {}
    }
    if has_unicode_word_boundary(&hir) {
        out.warning(
            "pattern-unicode-word-boundary",
            path,
            "a Unicode `\\b` makes matching slow on non-ASCII text; write `(?-u:\\b)` for an ASCII word boundary",
        );
    }
    let compiled = match regex::RegexBuilder::new(pattern).size_limit(SIZE_LIMIT).build() {
        Ok(compiled) => compiled,
        Err(regex::Error::CompiledTooBig(_)) => {
            out.error(
                "pattern-too-big",
                path,
                format!("compiles to more than {} KiB; use a simpler pattern", SIZE_LIMIT / 1024),
            );
            return None;
        }
        Err(err) => {
            out.error("pattern-syntax", path, short(&err.to_string()));
            return None;
        }
    };
    (out.errors() == before).then(|| Pattern { groups: compiled.captures_len() - 1, hir })
}

/// The reason line of regex-syntax's multi-line error text.
fn short(text: &str) -> String {
    text.lines().find_map(|line| line.strip_prefix("error: ")).unwrap_or(text).trim().to_string()
}

/// The flags a pattern writes, other than the ones allowed.
struct FlagVisitor(Vec<char>);

impl ast::Visitor for FlagVisitor {
    type Output = Vec<char>;
    type Err = std::convert::Infallible;

    fn finish(self) -> Result<Self::Output, Self::Err> {
        Ok(self.0)
    }

    fn visit_pre(&mut self, node: &Ast) -> Result<(), Self::Err> {
        let flags = match node {
            Ast::Flags(set) => Some(&set.flags),
            Ast::Group(group) => match &group.kind {
                GroupKind::NonCapturing(flags) => Some(flags),
                _ => None,
            },
            _ => None,
        };
        for item in flags.into_iter().flat_map(|flags| &flags.items) {
            let FlagsItemKind::Flag(flag) = &item.kind else { continue };
            let name = match flag {
                Flag::CaseInsensitive | Flag::Unicode => continue,
                Flag::MultiLine => 'm',
                Flag::DotMatchesNewLine => 's',
                Flag::SwapGreed => 'U',
                Flag::CRLF => 'R',
                Flag::IgnoreWhitespace => 'x',
            };
            self.0.push(name);
        }
        Ok(())
    }
}

fn visit<'a>(hir: &'a Hir, f: &mut impl FnMut(&'a Hir)) {
    f(hir);
    match hir.kind() {
        HirKind::Repetition(rep) => visit(&rep.sub, f),
        HirKind::Capture(cap) => visit(&cap.sub, f),
        HirKind::Concat(parts) | HirKind::Alternation(parts) => parts.iter().for_each(|part| visit(part, f)),
        HirKind::Empty | HirKind::Literal(_) | HirKind::Class(_) | HirKind::Look(_) => {}
    }
}

fn has_unicode_word_boundary(hir: &Hir) -> bool {
    let mut found = false;
    visit(hir, &mut |node| {
        found |= matches!(node.kind(), HirKind::Look(Look::WordUnicode | Look::WordUnicodeNegate));
    });
    found
}

impl Pattern {
    /// Whether capture `index` (0 is the whole match) is safe to become a program's argument, and
    /// what makes it not.
    ///
    /// A capture comes from a program's output, so it is hostile, and an argument that begins with
    /// `-` is read as an option (`--upload-pack=…`). Wormhole refuses such a capture when it runs and
    /// puts the end-of-options marker before it, but a pattern that *can* produce one is still
    /// wrong, so `validate` demands the pattern rule it out. A capture is accepted when all three hold:
    ///
    /// 0. **Present**: it always takes part in a match (it is in no alternative and no part that may
    ///    repeat zero times) and cannot match the empty string, so the argument is never empty.
    /// 1. **Bounded**: the longest string it can match is at most [`MAX_ARGUMENT_BYTES`].
    /// 2. **No dash first**: no string it matches starts with `-`. This is computed from the capture's
    ///    own HIR: the characters a literal, a class or a repetition of one can start with, through
    ///    groups and alternatives, and through the parts of a sequence that can match nothing
    ///    (an optional part may be skipped, so what follows it counts too). Anything else, such as
    ///    `.`, `\S`, `[^a]` or `\w`-with-`-`, can start with a dash and is refused.
    ///
    /// That is sound: it never accepts a capture that can start with `-`, and it refuses some
    /// patterns that cannot (it does not reason about what follows the capture). The design also lets
    /// a literal in front of the capture count as an anchor; that is not accepted, because
    /// `commit (.+)` still matches `commit --upload-pack=…`.
    pub(crate) fn capture_hazard(&self, index: usize) -> Option<CaptureHazard> {
        let (node, optional) =
            if index == 0 { (&self.hir, false) } else { find_capture(&self.hir, index, false)? };
        if optional || node.properties().minimum_len() == Some(0) {
            return Some(CaptureHazard::MayBeEmpty);
        }
        if node.properties().maximum_len().is_none_or(|max| max > MAX_ARGUMENT_BYTES) {
            return Some(CaptureHazard::Unbounded);
        }
        can_start_with_dash(node).then_some(CaptureHazard::MayStartWithDash)
    }
}

/// Capture `index` below `hir`, and whether a match can go without it (it is in an alternative, or in
/// a part that may repeat zero times).
fn find_capture(hir: &Hir, index: usize, optional: bool) -> Option<(&Hir, bool)> {
    match hir.kind() {
        HirKind::Capture(cap) if cap.index as usize == index => Some((&cap.sub, optional)),
        HirKind::Capture(cap) => find_capture(&cap.sub, index, optional),
        HirKind::Repetition(rep) => find_capture(&rep.sub, index, optional || rep.min == 0),
        HirKind::Concat(parts) => parts.iter().find_map(|part| find_capture(part, index, optional)),
        HirKind::Alternation(parts) => parts.iter().find_map(|part| find_capture(part, index, true)),
        HirKind::Empty | HirKind::Literal(_) | HirKind::Class(_) | HirKind::Look(_) => None,
    }
}

/// Whether some string `hir` matches begins with `-`.
fn can_start_with_dash(hir: &Hir) -> bool {
    match hir.kind() {
        HirKind::Empty | HirKind::Look(_) => false,
        HirKind::Literal(literal) => literal.0.first() == Some(&b'-'),
        HirKind::Class(Class::Unicode(class)) => {
            class.ranges().iter().any(|range| range.start() <= '-' && '-' <= range.end())
        }
        HirKind::Class(Class::Bytes(class)) => {
            class.ranges().iter().any(|range| range.start() <= b'-' && b'-' <= range.end())
        }
        HirKind::Repetition(rep) => can_start_with_dash(&rep.sub),
        HirKind::Capture(cap) => can_start_with_dash(&cap.sub),
        HirKind::Alternation(parts) => parts.iter().any(can_start_with_dash),
        HirKind::Concat(parts) => {
            for part in parts {
                if can_start_with_dash(part) {
                    return true;
                }
                // A part that cannot match nothing decides the first character; one that can is
                // skipped, so the next one may be first.
                if part.properties().minimum_len().is_some_and(|min| min > 0) {
                    return false;
                }
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The codes found in `pattern`, and the pattern when it passed.
    fn check(pattern: &str) -> (Vec<&'static str>, Option<Pattern>) {
        let mut out = Findings::default();
        let parsed = analyze(pattern, "p", &mut out);
        (out.0.iter().map(|f| f.code).collect(), parsed)
    }

    fn codes(pattern: &str) -> Vec<&'static str> {
        check(pattern).0
    }

    #[test]
    fn the_design_s_patterns_pass() {
        for ok in [
            r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
            r"(?-u:\b)(ACME-[0-9]{1,6})(?-u:\b)",
            r"sk-[A-Za-z0-9_-]{20,}",
            r"(?-u:\b)([0-9a-f][0-9a-f]{6,39})(?-u:\b)",
            r"(?i)error: (\w+)",
        ] {
            assert_eq!(codes(ok), Vec::<&str>::new(), "{ok}");
        }
    }

    #[test]
    fn groups_are_counted_without_the_whole_match() {
        assert_eq!(check("ab").1.map(|p| p.groups), Some(0));
        assert_eq!(check(r"(a)(?:b)(?P<n>c)").1.map(|p| p.groups), Some(2));
    }

    #[test]
    fn a_pattern_that_can_match_nothing_is_refused() {
        for empty in ["", "a*", "a?", "(a|)", r"(?-u:\b)", "^", "(a*)(b*)"] {
            assert_eq!(codes(empty), ["pattern-empty-match"], "{empty:?}");
        }
        assert_eq!(codes("a+"), Vec::<&str>::new());
    }

    #[test]
    fn syntax_the_engine_does_not_have_is_an_error_with_the_reason() {
        for bad in ["(a", "a)", "(?=a)b", r"(a)\1", "[", "a{2,1}", r"(?<=a)b"] {
            assert_eq!(codes(bad), ["pattern-syntax"], "{bad}");
        }
        let mut out = Findings::default();
        analyze("(a", "p", &mut out);
        assert!(out.0[0].message.contains("unclosed group"), "{}", out.0[0].message);
    }

    #[test]
    fn only_the_i_flag_and_switching_unicode_off_are_allowed() {
        assert_eq!(codes("(?i)a"), Vec::<&str>::new());
        assert_eq!(codes("(?i:a)b"), Vec::<&str>::new());
        for (pattern, flag) in
            [("(?s).+", 's'), ("(?m)^a", 'm'), ("(?x)a b", 'x'), ("(?U)a+", 'U'), ("a(?s:.)", 's')]
        {
            let mut out = Findings::default();
            analyze(pattern, "p", &mut out);
            let f = out.0.iter().find(|f| f.code == "pattern-flag").unwrap_or_else(|| panic!("{pattern}"));
            assert!(f.message.contains(&format!("`{flag}`")), "{}", f.message);
        }
    }

    #[test]
    fn a_unicode_word_boundary_is_a_warning_with_the_fix() {
        let mut out = Findings::default();
        assert!(analyze(r"\bword\b", "p", &mut out).is_some(), "still usable");
        assert_eq!(out.0.len(), 1);
        assert_eq!(out.0[0].code, "pattern-unicode-word-boundary");
        assert_eq!(out.0[0].severity, super::super::Severity::Warning);
        assert!(out.0[0].message.contains(r"(?-u:\b)"));
    }

    #[test]
    fn size_and_length_are_capped() {
        assert_eq!(codes(&"a".repeat(MAX_PATTERN_BYTES + 1)), ["pattern-too-long"]);
        assert_eq!(codes(&"a".repeat(MAX_PATTERN_BYTES)), Vec::<&str>::new());
        // 2^10 copies of a class of a thousand characters is far over 256 KiB once compiled.
        assert_eq!(codes(r"(\w{40}){40}"), ["pattern-too-big"]);
    }

    fn hazard(pattern: &str, group: usize) -> Option<CaptureHazard> {
        let (found, parsed) = check(pattern);
        assert!(
            found.iter().all(|c| !c.starts_with("pattern-") || *c == "pattern-unicode-word-boundary"),
            "{found:?}"
        );
        parsed.unwrap_or_else(|| panic!("{pattern}")).capture_hazard(group)
    }

    #[test]
    fn a_bare_catch_all_capture_is_a_hazard() {
        assert_eq!(hazard("(.+)", 1), Some(CaptureHazard::Unbounded));
        for dash in [r"(.{1,9})", r"(\S{1,9})", "([^a]{1,9})", "([a-z-]{2,9})", r"(\w|-){1,5}"] {
            let group = usize::from(dash.starts_with('('));
            assert_eq!(hazard(dash, group), Some(CaptureHazard::MayStartWithDash), "{dash}");
        }
    }

    #[test]
    fn a_bounded_capture_that_cannot_start_with_a_dash_is_fine() {
        for ok in [
            r"(?-u:\b)([0-9a-f][0-9a-f]{6,39})(?-u:\b)",
            "(abc)",
            "([A-Za-z0-9][A-Za-z0-9._-]{0,63})",
            "(a|b-c)",
            "(x?[0-9]{1,4})",
            "(?i)(HEAD|main)",
        ] {
            assert_eq!(hazard(ok, 1), None, "{ok}");
        }
    }

    #[test]
    fn a_dash_can_come_first_through_an_optional_part_or_an_alternative() {
        assert_eq!(hazard("(-?[0-9]{1,5})", 1), Some(CaptureHazard::MayStartWithDash));
        assert_eq!(hazard("(a|-b)", 1), Some(CaptureHazard::MayStartWithDash));
        assert_eq!(hazard("(-)", 1), Some(CaptureHazard::MayStartWithDash));
        assert_eq!(hazard("([+-][0-9]{1,3})", 1), Some(CaptureHazard::MayStartWithDash));
        assert_eq!(hazard("(?:-|x)(a)", 0), Some(CaptureHazard::MayStartWithDash), "the whole match");
    }

    #[test]
    fn a_capture_that_can_be_empty_or_not_take_part_is_a_hazard() {
        for bad in ["(a*)b", "(a?)b", "(a|)b", "(?:(a)|b)", "(?:(a)|(b))", "(?:(a))?b", "x(?:(a)){0,3}"] {
            let group = 1;
            assert_eq!(hazard(bad, group), Some(CaptureHazard::MayBeEmpty), "{bad}");
        }
        assert_eq!(hazard("(?:(b))+", 1), None, "a repetition that must happen once");
        assert_eq!(hazard("(a|b)c", 1), None, "the alternative is inside the group");
        assert_eq!(hazard("(?:a|(b))c", 0), None, "the whole match is always there");
    }

    #[test]
    fn a_capture_must_be_bounded() {
        assert_eq!(hazard("([a-z]+)", 1), Some(CaptureHazard::Unbounded));
        assert_eq!(hazard("([a-z]{1,1025})", 1), Some(CaptureHazard::Unbounded));
        assert_eq!(hazard("([a-z]{1,1024})", 1), None);
    }

    #[test]
    fn the_whole_match_is_group_zero() {
        assert_eq!(hazard("x([a-z]{1,4})", 0), None);
        assert_eq!(hazard("[a-z]{1,4}x", 0), None);
        assert_eq!(hazard("foo (.+)", 0), Some(CaptureHazard::Unbounded));
    }

    #[test]
    fn a_group_that_is_not_there_has_no_hazard() {
        assert_eq!(hazard("(a)", 5), None);
    }
}
