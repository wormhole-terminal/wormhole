//! Finding what the rules match in one logical line.

use std::ops::Range;

use super::compile::CompiledRules;
use crate::probe::manifest::Look;

/// Matches one rule may make in one logical line; the rest are not looked for.
pub const MAX_MATCHES_PER_RULE: usize = 8;
/// Bytes of a logical line that are scanned; a rule never sees what lies beyond.
pub const MAX_SCAN_BYTES: usize = 2048;

/// One span a rule found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// The rule's index in [`CompiledRules`].
    pub rule: usize,
    /// Byte offsets in the line.
    pub start: usize,
    pub end: usize,
    pub look: Look,
}

impl Match {
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }
}

/// The part of `line` that is scanned: at most [`MAX_SCAN_BYTES`], cut at a character boundary.
pub(super) fn scanned(line: &str) -> &str {
    if line.len() <= MAX_SCAN_BYTES {
        return line;
    }
    let mut end = MAX_SCAN_BYTES;
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    &line[..end]
}

/// What `rules` match in `line`, a logical line, in the order the spans lie in it.
///
/// - at most [`MAX_SCAN_BYTES`] of the line are scanned, and each rule finds at most
///   [`MAX_MATCHES_PER_RULE`] matches, counted before overlaps are settled, so the work a line can
///   cost is bounded whatever it holds;
/// - spans that overlap are settled leftmost-longest, a rule listed earlier winning a tie, and a `redact` span beats
///   any other look wherever it lies: half a redacted key left showing would defeat the rule.
///
/// No span is empty, and none overlaps another.
pub fn matches(rules: &CompiledRules, line: &str) -> Vec<Match> {
    let text = scanned(line);
    let mut found: Vec<Match> = Vec::new();
    let candidates: Vec<usize> = match &rules.prefilter {
        Some(set) => set.matches(text).into_iter().collect(),
        None => (0..rules.rules.len()).collect(),
    };
    for index in candidates {
        let rule = &rules.rules[index];
        found.extend(
            rule.regex.find_iter(text).filter(|m| !m.is_empty()).take(MAX_MATCHES_PER_RULE).map(|m| Match {
                rule: index,
                start: m.start(),
                end: m.end(),
                look: rule.rule.look,
            }),
        );
    }
    settle(found)
}

/// Keep the spans that win their overlaps.
fn settle(mut found: Vec<Match>) -> Vec<Match> {
    // Redacted spans first, then by where they start, longest first, earliest rule first. Taking spans
    // in this order and skipping any that touches one taken makes both rules fall out.
    found.sort_by_key(|m| (m.look != Look::Redact, m.start, std::cmp::Reverse(m.end - m.start), m.rule));
    let mut kept: Vec<Match> = Vec::with_capacity(found.len());
    for candidate in found {
        if !kept.iter().any(|k| candidate.start < k.end && k.start < candidate.end) {
            kept.push(candidate);
        }
    }
    kept.sort_by_key(|m| m.start);
    kept
}
