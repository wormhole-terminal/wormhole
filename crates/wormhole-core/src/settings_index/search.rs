//! Setting-level search: every word of the query has to be found in a setting, in its
//! label, description, search words, config keys, or the titles of its group and section. What
//! is found where decides the order: a label that starts a word with the query first, a config
//! key or the description after, the search words last. Outside the label a word has to start
//! a word there ("line" finds "Lines kept", not "readline"). Results come in nav order, a section's
//! best first, with the range to mark in the label or the description.

use std::ops::Range;

use super::{Entry, Group, SectionId, entries};
use crate::config::Settings;

/// What the person typed: lowercased words, all of which have to be found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Query {
    words: Vec<String>,
}

impl Query {
    pub fn new(text: &str) -> Self {
        Self { words: text.split_whitespace().map(str::to_lowercase).collect() }
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// The query as typed, normalised ("line height").
    pub fn text(&self) -> String {
        self.words.join(" ")
    }
}

/// A setting the query found.
pub struct Hit {
    pub section: SectionId,
    pub group: &'static Group,
    pub entry: &'static Entry,
    /// Higher is a better match.
    pub score: u32,
    /// Where the first word that is in the label sits in it (bytes), to mark.
    pub label_range: Option<Range<usize>>,
    /// The same in the description, when the label has none of the words.
    pub description_range: Option<Range<usize>>,
    /// A config key a word was found in.
    pub key: Option<&'static str>,
}

/// Points for where a word was found; a word scores its best place.
const LABEL_WORD_START: u32 = 100;
const LABEL_INSIDE: u32 = 70;
const KEY: u32 = 60;
const DESCRIPTION_WORD_START: u32 = 40;
const KEYWORDS: u32 = 20;
const GROUP_TITLE: u32 = 15;
const SECTION_TITLE: u32 = 10;
/// The whole query begins the label ("line h" for "Line height").
const LABEL_PREFIX_BONUS: u32 = 50;

/// Every setting `query` finds, sections in nav order and each section's best first. An empty
/// query finds nothing.
pub fn search(query: &Query) -> Vec<Hit> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(usize, Hit)> = entries()
        .enumerate()
        .filter_map(|(order, (section, group, entry))| {
            score(query, section, group, entry).map(|hit| (order, hit))
        })
        .collect();
    let section_order = |section: SectionId| SectionId::ALL.iter().position(|s| *s == section);
    hits.sort_by(|(a_order, a), (b_order, b)| {
        section_order(a.section)
            .cmp(&section_order(b.section))
            .then(b.score.cmp(&a.score))
            .then(a_order.cmp(b_order))
    });
    hits.into_iter().map(|(_, hit)| hit).collect()
}

/// Like [`search`], but only in the sections shown for this build and settings, so a hidden
/// feature's settings (a preview section off in this build) never surface in the results.
pub fn search_visible(query: &Query, settings: &Settings) -> Vec<Hit> {
    let mut hits = search(query);
    hits.retain(|hit| hit.section.is_visible(settings));
    hits
}

/// Where `word` is in `text` (lowercase) and whether that is the start of a word there.
fn find(text: &str, word: &str) -> Option<(usize, bool)> {
    let mut first = None;
    for (at, _) in text.match_indices(word) {
        let starts_word = text[..at].chars().next_back().is_none_or(|c| !c.is_alphanumeric());
        if starts_word {
            return Some((at, true));
        }
        first.get_or_insert((at, false));
    }
    first
}

/// The byte range of a match found in the lowercased copy, when lowercasing kept every byte
/// where it was (so the range fits the original text too).
fn range_in(original: &str, lower: &str, at: usize, len: usize) -> Option<Range<usize>> {
    (original.len() == lower.len() && original.is_char_boundary(at) && original.is_char_boundary(at + len))
        .then(|| at..at + len)
}

fn score(query: &Query, section: SectionId, group: &'static Group, entry: &'static Entry) -> Option<Hit> {
    let label = entry.label.to_lowercase();
    let description = entry.description.to_lowercase();
    let keywords = entry.keywords.to_lowercase();
    let group_title = group.title.to_lowercase();
    let section_title = section.title().to_lowercase();
    let mut hit =
        Hit { section, group, entry, score: 0, label_range: None, description_range: None, key: None };
    for word in &query.words {
        let mut best = 0;
        if let Some((at, starts)) = find(&label, word) {
            best = best.max(if starts { LABEL_WORD_START } else { LABEL_INSIDE });
            if hit.label_range.is_none() {
                hit.label_range = range_in(entry.label, &label, at, word.len());
            }
        }
        if let Some(key) = entry.keys.iter().find(|key| key.contains(word.as_str())) {
            best = best.max(KEY);
            hit.key.get_or_insert(key);
        }
        // Outside the label a word counts only where a word starts with it: "line" should
        // not find "readline".
        if let Some((at, true)) = find(&description, word) {
            best = best.max(DESCRIPTION_WORD_START);
            if hit.description_range.is_none() {
                hit.description_range = range_in(entry.description, &description, at, word.len());
            }
        }
        if let Some((_, true)) = find(&keywords, word) {
            best = best.max(KEYWORDS);
        }
        if group_title.contains(word.as_str()) {
            best = best.max(GROUP_TITLE);
        }
        if section_title.contains(word.as_str()) {
            best = best.max(SECTION_TITLE);
        }
        if best == 0 {
            return None;
        }
        hit.score += best;
    }
    if label.starts_with(&query.text()) {
        hit.score += LABEL_PREFIX_BONUS;
    }
    // One place to mark: the label when it has a word, else the description.
    if hit.label_range.is_some() {
        hit.description_range = None;
    }
    Some(hit)
}

#[cfg(test)]
mod tests {
    use super::{Query, SectionId, search};

    fn ids(query: &str) -> Vec<&'static str> {
        search(&Query::new(query)).iter().map(|hit| hit.entry.id).collect()
    }

    #[test]
    fn every_word_has_to_be_found_in_any_order_and_case() {
        assert_eq!(ids("tab width"), ["tab-width"]);
        assert_eq!(ids("  WIDTH   tab "), ["tab-width"]);
        assert!(ids("tab zebra").is_empty());
        assert!(ids("").is_empty() && ids("   ").is_empty());
    }

    #[test]
    fn descriptions_search_words_and_config_keys_find_settings() {
        // The wrap switch's description says the diff scrolls sideways.
        assert_eq!(ids("sideways"), ["diff-wrap"]);
        // A config key, as it is written in config.json.
        assert_eq!(ids("line_height"), ["line-height"]);
        assert!(ids("scrollback_lines").contains(&"scrollback"));
        // A word only the search words have.
        assert!(ids("vibrancy").contains(&"blur"));
        let hit = search(&Query::new("line_height")).into_iter().next().expect("found");
        assert_eq!(hit.key, Some("line_height"));
    }

    #[test]
    fn results_come_in_nav_order_with_the_best_of_a_section_first() {
        let hits = search(&Query::new("line"));
        let sections: Vec<SectionId> = hits.iter().map(|hit| hit.section).collect();
        let mut sorted = sections.clone();
        sorted.sort_by_key(|s| SectionId::ALL.iter().position(|x| x == s));
        assert_eq!(sections, sorted, "sections in nav order");
        // "Line height" (the label starts with it) before rows that only mention a line.
        let fonts: Vec<&str> =
            hits.iter().filter(|h| h.section == SectionId::Fonts).map(|h| h.entry.id).collect();
        assert_eq!(fonts.first(), Some(&"line-height"));
        for id in ["line-height", "scrollback", "density", "diff-wrap"] {
            assert!(hits.iter().any(|h| h.entry.id == id), "\"line\" finds {id}");
        }
    }

    #[test]
    fn a_label_match_ranks_above_a_description_match() {
        let hits = search(&Query::new("blur"));
        let blur = hits.iter().find(|h| h.entry.id == "blur").expect("the blur row");
        let opacity = hits.iter().find(|h| h.entry.id == "opacity");
        assert!(opacity.is_none_or(|o| o.score < blur.score));
        // "tree" alone is also in the Worktrees folder's label now; "view" keeps it to the one row.
        let hits = search(&Query::new("tree view"));
        assert_eq!(hits.first().map(|h| h.entry.id), Some("changes-view"));
    }

    #[test]
    fn the_match_is_marked_where_it_was_found() {
        let hits = search(&Query::new("wrap"));
        let wrap = hits.iter().find(|h| h.entry.id == "diff-wrap").expect("found");
        assert_eq!(wrap.label_range, Some(0..4), "\"Wrap long lines\"");
        assert_eq!(wrap.description_range, None, "one mark per row");

        let hits = search(&Query::new("kept"));
        let scrollback = hits.iter().find(|h| h.entry.id == "scrollback").expect("found");
        assert_eq!(scrollback.label_range, None);
        let range = scrollback.description_range.clone().expect("in the description");
        assert_eq!(&scrollback.entry.description[range], "kept");

        // Inside a word of the label too ("lines" in "Wrap long lines").
        let hits = search(&Query::new("ines"));
        let wrap = hits.iter().find(|h| h.entry.id == "diff-wrap").expect("found");
        assert_eq!(wrap.label_range.clone().map(|r| &wrap.entry.label[r]), Some("ines"));
        // But not inside a word of the description: "readline" is not a line.
        assert!(!ids("line").contains(&"meta"));
    }

    #[test]
    fn section_and_group_titles_find_their_settings() {
        // "bell" is the Terminal page's group: both of its rows.
        let found = ids("bell");
        assert!(found.contains(&"bell-sound") && found.contains(&"bell-visual"));
        assert!(ids("updates").contains(&"auto-update"));
    }
}
