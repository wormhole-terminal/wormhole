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
/// See [`matches_into`] for the rules; this is it with a buffer of its own.
pub fn matches(rules: &CompiledRules, line: &str) -> Vec<Match> {
    let mut found = Vec::new();
    matches_into(rules, line, &mut found);
    found
}

/// [`matches`] into `out`, which is emptied first and keeps its room: a caller that matches line after
/// line (the row builder) reuses one buffer and allocates nothing once it has grown.
///
/// - at most [`MAX_SCAN_BYTES`] of the line are scanned, and each rule finds at most
///   [`MAX_MATCHES_PER_RULE`] matches, counted before overlaps are settled, so the work a line can
///   cost is bounded whatever it holds;
/// - spans that overlap are settled leftmost-longest, a rule listed earlier winning a tie, and a `redact` span beats
///   any other look wherever it lies: half a redacted key left showing would defeat the rule.
///
/// No span is empty, and none overlaps another.
pub fn matches_into(rules: &CompiledRules, line: &str, out: &mut Vec<Match>) {
    out.clear();
    let text = scanned(line);
    match &rules.prefilter {
        Some(set) => {
            let hits = set.matches(text);
            for index in hits.iter() {
                find(rules, index, text, out);
            }
        }
        None => (0..rules.rules.len()).for_each(|index| find(rules, index, text, out)),
    }
    settle(out);
}

/// The matches of rule `index` in `text`, at most [`MAX_MATCHES_PER_RULE`], pushed on `out`.
fn find(rules: &CompiledRules, index: usize, text: &str, out: &mut Vec<Match>) {
    let rule = &rules.rules[index];
    out.extend(
        rule.regex.find_iter(text).filter(|m| !m.is_empty()).take(MAX_MATCHES_PER_RULE).map(|m| Match {
            rule: index,
            start: m.start(),
            end: m.end(),
            look: rule.rule.look,
        }),
    );
}

/// Keep, in place, the spans that win their overlaps, ordered by where they start.
///
/// The candidates are taken in the order of their claim: redacted spans first, then by where they start,
/// longest first, earliest rule first. The ones kept are a prefix of `found`, ordered by start and never
/// overlapping, so whether a candidate touches one of them is decided by its two neighbours there, found
/// by a binary search: the work is that of a sort, not of every candidate against every one kept.
fn settle(found: &mut Vec<Match>) {
    found.sort_unstable_by_key(|m| {
        (m.look != Look::Redact, m.start, std::cmp::Reverse(m.end - m.start), m.rule)
    });
    let mut kept = 0;
    for next in 0..found.len() {
        let candidate = found[next].clone();
        let at = found[..kept].partition_point(|k| k.start < candidate.start);
        let touches_before = at > 0 && found[at - 1].end > candidate.start;
        let touches_after = at < kept && found[at].start < candidate.end;
        if touches_before || touches_after {
            continue;
        }
        // Slot it in at `at`, moving the ones after it up by one.
        found[at..=next].rotate_right(1);
        kept += 1;
    }
    found.truncate(kept);
}
