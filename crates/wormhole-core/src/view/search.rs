//! The command palette's search: which scope a query asks for, how well a result matches
//! it (fuzzy, with the matched characters to mark), the order results are ranked in, and the fixed
//! order of the groups they are shown in. The app gathers the results (terminals of every window,
//! folders, commits, settings, actions) and draws them; this decides what is found and where.
//!
//! A query is a scope prefix (`@` terminals, `~` folders, `#` commits, `,` settings, `>`
//! actions) and words. Files have no scope (backlog): their `/` is not a prefix, so "/src" is a
//! plain query that looks for a slash. Every word has to be found in a result, its letters in
//! order but not necessarily together: a run of them, then more from the start of a later word
//! ("shm" finds "theme shape metrics", "sp" "sidebar polish"); a word as typed is found anywhere.
//! Matches at word starts and long runs rank higher, as does a match early in the title.

use std::ops::Range;

/// What the palette searches. The scope row lists them in [`Scope::ROW`]'s order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    All,
    Terminals,
    Folders,
    Commits,
    Settings,
    Actions,
}

impl Scope {
    /// The scope row, left to right (without Files). Tab steps through it.
    pub const ROW: [Scope; 6] =
        [Scope::All, Scope::Terminals, Scope::Folders, Scope::Commits, Scope::Settings, Scope::Actions];

    /// The character that, typed first, narrows the palette to this scope.
    pub fn prefix(self) -> Option<char> {
        match self {
            Scope::All => None,
            Scope::Terminals => Some('@'),
            Scope::Folders => Some('~'),
            Scope::Commits => Some('#'),
            Scope::Settings => Some(','),
            Scope::Actions => Some('>'),
        }
    }

    /// Its name on the scope row and in the search field's chip.
    pub fn title(self) -> &'static str {
        match self {
            Scope::All => "All",
            Scope::Terminals => "Terminals",
            Scope::Folders => "Folders",
            Scope::Commits => "Commits",
            Scope::Settings => "Settings",
            Scope::Actions => "Actions",
        }
    }

    /// What the empty search field says.
    pub fn placeholder(self) -> &'static str {
        match self {
            Scope::All => "Jump to a terminal, folder, commit or setting…",
            Scope::Terminals => "Terminals in every window…",
            Scope::Folders => "Folders with a terminal, and recent ones…",
            Scope::Commits => "Commits on this branch…",
            Scope::Settings => "Find a setting…",
            Scope::Actions => "Run an action…",
        }
    }

    /// The scope Tab (`forward`) or ⇧Tab moves to, wrapping around.
    pub fn step(self, forward: bool) -> Scope {
        let n = Self::ROW.len();
        let at = Self::ROW.iter().position(|s| *s == self).unwrap_or(0);
        Self::ROW[if forward { (at + 1) % n } else { (at + n - 1) % n }]
    }

    /// The one group a narrowed palette shows.
    pub fn group(self) -> Option<Group> {
        match self {
            Scope::All => None,
            Scope::Terminals => Some(Group::Terminals),
            Scope::Folders => Some(Group::Folders),
            Scope::Commits => Some(Group::Commits),
            Scope::Settings => Some(Group::Settings),
            Scope::Actions => Some(Group::Actions),
        }
    }
}

/// The scope `input` asks for and the query that follows its prefix. Without a prefix (or with
/// Files' `/`, which isn't one) it is all of it, as typed.
pub fn parse(input: &str) -> (Scope, &str) {
    let trimmed = input.trim_start();
    let Some(first) = trimmed.chars().next() else { return (Scope::All, input) };
    match Scope::ROW.into_iter().find(|scope| scope.prefix() == Some(first)) {
        Some(scope) => (scope, trimmed[first.len_utf8()..].trim_start()),
        None => (Scope::All, input),
    }
}

/// A kind of result, as the palette groups them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Group {
    Settings,
    Actions,
    Terminals,
    Folders,
    Commits,
}

impl Group {
    /// The order groups are shown in when everything is searched: settings and actions
    /// before content, then terminals, folders and commits.
    pub const ORDER: [Group; 5] =
        [Group::Settings, Group::Actions, Group::Terminals, Group::Folders, Group::Commits];

    pub fn title(self) -> &'static str {
        match self {
            Group::Settings => "Settings",
            Group::Actions => "Actions",
            Group::Terminals => "Terminals",
            Group::Folders => "Folders",
            Group::Commits => "Commits",
        }
    }
}

/// How many results of each group the unscoped palette shows; the group's label still counts all
/// of them, and its scope lists them all.
pub const GROUP_MAX: usize = 5;

/// How well a text matched, and the characters to mark (byte ranges into the text).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Match {
    pub score: i32,
    pub ranges: Vec<Range<usize>>,
}

/// Points for a matched character, and what adds to or takes from them.
const MATCH: i32 = 16;
/// The character starts the text, or a word in it ("Show changes", "sidebar_polish", "diffPerf").
const WORD_START: i32 = 10;
const TEXT_START: i32 = 6;
/// It follows the previous matched character directly.
const RUN: i32 = 12;
/// Each character skipped between two matched ones, and before the first (up to a limit).
const GAP: i32 = 1;
const LEAD_GAP_MAX: i32 = 8;
/// A word found in another of a result's texts (its folder, its description) counts this share
/// of what it would in the title.
const OTHER_TEXT_SHARE: i32 = 2;

/// A character of the text being searched: where it is, and its lowercase form.
struct Ch {
    at: usize,
    len: usize,
    lower: char,
    word_start: bool,
}

fn chars(text: &str) -> Vec<Ch> {
    let mut out: Vec<Ch> = Vec::with_capacity(text.len());
    let mut prev: Option<char> = None;
    for (at, c) in text.char_indices() {
        let word_start = match prev {
            None => true,
            Some(p) => !p.is_alphanumeric() && c.is_alphanumeric() || p.is_lowercase() && c.is_uppercase(),
        };
        out.push(Ch { at, len: c.len_utf8(), lower: lower(c), word_start });
        prev = Some(c);
    }
    out
}

fn lower(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// How well `word` (one word of a query) matches `text`: its characters in order, the first at a
/// word start unless the word is in the text as it was typed. `None` when it isn't there.
pub fn fuzzy(word: &str, text: &str) -> Option<Match> {
    let query: Vec<char> = word.chars().map(lower).collect();
    if query.is_empty() {
        return Some(Match::default());
    }
    let text = chars(text);
    if query.len() > text.len() {
        return None;
    }
    let contiguous = |picks: &[usize]| picks.windows(2).all(|w| w[1] == w[0] + 1);
    // The best alignment anywhere, if it starts a word or is the word as typed; else the best one
    // that starts a word; else the word as typed, wherever it is.
    let (score, picks) = align(&query, &text, false)
        .filter(|(_, picks)| text[picks[0]].word_start || contiguous(picks))
        .or_else(|| align(&query, &text, true))
        .or_else(|| as_typed(&query, &text))?;
    let mut ranges: Vec<Range<usize>> = Vec::new();
    for &j in &picks {
        let ch = &text[j];
        match ranges.last_mut() {
            Some(last) if last.end == ch.at => last.end = ch.at + ch.len,
            _ => ranges.push(ch.at..ch.at + ch.len),
        }
    }
    Some(Match { score, ranges })
}

/// The score of `query[0]` matched at `text[j]`.
fn first_score(text: &[Ch], j: usize) -> i32 {
    let bonus = match j {
        0 => TEXT_START + WORD_START,
        _ if text[j].word_start => WORD_START,
        _ => 0,
    };
    MATCH + bonus - (j as i32 * GAP).min(LEAD_GAP_MAX)
}

/// The best placement of `query`'s characters in `text`, in order (with `word_start`, the first
/// at a word start), and its score: dynamic programming over (query character, text position).
fn align(query: &[char], text: &[Ch], word_start: bool) -> Option<(i32, Vec<usize>)> {
    const NONE: i32 = i32::MIN / 2;
    let (m, n) = (query.len(), text.len());
    // best[i][j]: the best score with query[..=i] placed and query[i] at text[j]; from[i][j]: where
    // query[i - 1] was then.
    let mut best = vec![vec![NONE; n]; m];
    let mut from = vec![vec![usize::MAX; n]; m];
    for j in 0..n {
        if text[j].lower == query[0] && (!word_start || text[j].word_start) {
            best[0][j] = first_score(text, j);
        }
    }
    for i in 1..m {
        // The best score with query[i - 1] two or more characters before j, less the gap: one
        // character more skipped at each step.
        let (mut skipped, mut skipped_at) = (NONE, usize::MAX);
        for j in i..n {
            if text[j].lower == query[i] {
                let run = best[i - 1][j - 1];
                // After a gap the next character has to start a word ("sp" in "sidebar polish",
                // not "theme" in "the README").
                let skip = if text[j].word_start { skipped } else { NONE };
                let (prev, at) =
                    if run > NONE && run + RUN >= skip { (run + RUN, j - 1) } else { (skip, skipped_at) };
                if prev > NONE {
                    best[i][j] = prev + MATCH + if text[j].word_start { WORD_START } else { 0 };
                    from[i][j] = at;
                }
            }
            let decayed = if skipped > NONE { skipped - GAP } else { NONE };
            let here = best[i - 1][j - 1];
            if here > NONE && here - GAP > decayed {
                (skipped, skipped_at) = (here - GAP, j - 1);
            } else {
                skipped = decayed;
            }
        }
    }
    let (end, score) =
        (0..n).map(|j| (j, best[m - 1][j])).max_by_key(|&(j, score)| (score, usize::MAX - j))?;
    if score <= NONE {
        return None;
    }
    let mut picks = vec![0; m];
    let mut j = end;
    for i in (0..m).rev() {
        picks[i] = j;
        j = from[i][j];
    }
    Some((score, picks))
}

/// `query` as one run of characters in `text`, where it first is.
fn as_typed(query: &[char], text: &[Ch]) -> Option<(i32, Vec<usize>)> {
    let start = (0..=text.len() - query.len())
        .find(|&j| query.iter().enumerate().all(|(i, c)| text[j + i].lower == *c))?;
    let score = first_score(text, start) + (query.len() as i32 - 1) * (MATCH + RUN);
    Some((score, (start..start + query.len()).collect()))
}

/// What a result is searched by: its title, which the palette marks, and other texts that find it
/// too (a terminal's folder, a setting's description) without being marked.
pub struct Candidate<'a> {
    pub title: &'a str,
    pub other: Vec<&'a str>,
}

impl<'a> Candidate<'a> {
    pub fn new(title: &'a str) -> Self {
        Self { title, other: Vec::new() }
    }

    pub fn with(mut self, other: &'a str) -> Self {
        self.other.push(other);
        self
    }
}

/// How well a whole query matches `candidate`: every word has to be found, in the title (marked)
/// or one of its other texts. `None` when a word isn't.
pub fn score(query: &str, candidate: &Candidate) -> Option<Match> {
    let mut total = Match::default();
    for word in query.split_whitespace() {
        let found = match fuzzy(word, candidate.title) {
            Some(found) => found,
            None => {
                let other =
                    candidate.other.iter().filter_map(|text| fuzzy(word, text)).map(|m| m.score).max()?;
                Match { score: other / OTHER_TEXT_SHARE, ranges: Vec::new() }
            }
        };
        total.score += found.score;
        total.ranges.extend(found.ranges);
    }
    total.ranges.sort_by_key(|r| r.start);
    total.ranges.dedup_by(|b, a| {
        // Merge overlapping or touching ranges (two words that matched side by side).
        if b.start <= a.end {
            a.end = a.end.max(b.end);
            true
        } else {
            false
        }
    });
    Some(total)
}

/// One result that a query found: which one, and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ranked {
    pub index: usize,
    pub found: Match,
}

/// The results `query` finds among `candidates`, best first (equal ones keep their order). An
/// empty query finds them all, in their order.
pub fn rank<'a>(query: &str, candidates: impl IntoIterator<Item = Candidate<'a>>) -> Vec<Ranked> {
    let mut found: Vec<Ranked> = candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, candidate)| score(query, &candidate).map(|found| Ranked { index, found }))
        .collect();
    if !query.trim().is_empty() {
        found.sort_by(|a, b| b.found.score.cmp(&a.found.score).then(a.index.cmp(&b.index)));
    }
    found
}

/// A group of results as the palette shows it: how many were found and the ones it lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub group: Group,
    pub total: usize,
    pub shown: Vec<Ranked>,
}

/// The palette's groups for `scope`: everything searched lists the groups that found something in
/// [`Group::ORDER`], each cut to [`GROUP_MAX`]; a scope lists its own group, all of it.
pub fn sections(scope: Scope, found: impl IntoIterator<Item = (Group, Vec<Ranked>)>) -> Vec<Section> {
    let mut found: Vec<(Group, Vec<Ranked>)> = found
        .into_iter()
        .filter(|(group, ranked)| !ranked.is_empty() && scope.group().is_none_or(|g| g == *group))
        .collect();
    found.sort_by_key(|(group, _)| Group::ORDER.iter().position(|g| g == group));
    found
        .into_iter()
        .map(|(group, mut ranked)| {
            let total = ranked.len();
            if scope == Scope::All {
                ranked.truncate(GROUP_MAX);
            }
            Section { group, total, shown: ranked }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Candidate, GROUP_MAX, Group, Ranked, Scope, fuzzy, parse, rank, score, sections};

    fn marked<'a>(text: &'a str, word: &str) -> Vec<&'a str> {
        fuzzy(word, text).map(|m| m.ranges.iter().map(|r| &text[r.clone()]).collect()).unwrap_or_default()
    }

    fn titles<'a>(query: &str, titles: &[&'a str]) -> Vec<&'a str> {
        rank(query, titles.iter().map(|t| Candidate::new(t))).iter().map(|r| titles[r.index]).collect()
    }

    #[test]
    fn a_prefix_picks_the_scope_and_files_slash_is_a_plain_query() {
        assert_eq!(parse("@diff"), (Scope::Terminals, "diff"));
        assert_eq!(parse("> new tab"), (Scope::Actions, "new tab"));
        assert_eq!(parse("~"), (Scope::Folders, ""));
        assert_eq!(parse("#fix"), (Scope::Commits, "fix"));
        assert_eq!(parse(",theme"), (Scope::Settings, "theme"));
        assert_eq!(parse("theme"), (Scope::All, "theme"));
        assert_eq!(parse(""), (Scope::All, ""));
        // Files is in the backlog: its prefix is not recognised and stays part of the query.
        assert_eq!(parse("/style.rs"), (Scope::All, "/style.rs"));
        // Only first: "a@b" looks for "a@b".
        assert_eq!(parse("a@b"), (Scope::All, "a@b"));
        for scope in Scope::ROW {
            let typed = format!("{}x", scope.prefix().map(String::from).unwrap_or_default());
            assert_eq!(parse(&typed), (scope, "x"), "{scope:?}");
        }
    }

    #[test]
    fn tab_walks_the_scope_row_and_wraps() {
        assert_eq!(Scope::All.step(true), Scope::Terminals);
        assert_eq!(Scope::Actions.step(true), Scope::All);
        assert_eq!(Scope::All.step(false), Scope::Actions);
        let titles: Vec<&str> = Scope::ROW.iter().map(|s| s.title()).collect();
        assert_eq!(titles, ["All", "Terminals", "Folders", "Commits", "Settings", "Actions"], "no Files");
    }

    #[test]
    fn letters_in_order_are_found_and_marked() {
        assert_eq!(marked("Theme Studio", "theme"), ["Theme"]);
        assert_eq!(marked("theme_picker.rs", "theme"), ["theme"]);
        // Word starts: s-hape m-etrics.
        assert_eq!(marked("theme shape metrics", "shm"), ["sh", "m"]);
        assert_eq!(marked("sidebar polish", "sp"), ["s", "p"]);
        // Inside a word only as typed, all together.
        assert_eq!(marked("feat(theme): let themes change", "eme"), ["eme"]);
        assert_eq!(fuzzy("hme", "theme"), None, "scattered inside a word");
        assert_eq!(fuzzy("theme", "Describe fuel in the README"), None, "on into the middle of a word");
        assert_eq!(fuzzy("zz", "theme"), None);
        // Case doesn't matter; camel case starts a word.
        assert_eq!(marked("diffPerf", "dp"), ["d", "P"]);
        // Bytes, not characters: after a multi-byte character.
        assert_eq!(marked("Settings › Theme", "the"), ["The"]);
    }

    #[test]
    fn word_starts_and_runs_rank_above_scattered_letters() {
        assert_eq!(
            titles("theme", &["Translucent Theme Studio window", "Theme", "theme shape metrics"])[0],
            "Theme"
        );
        // A word start beats the same letters inside a word.
        assert_eq!(titles("tab", &["Stable", "New tab"]), ["New tab", "Stable"]);
        // Earlier beats later.
        assert_eq!(
            titles("show", &["Hide and show sidebar", "Show changes"]),
            ["Show changes", "Hide and show sidebar"]
        );
        // Equal matches keep their order.
        assert_eq!(titles("a", &["a one", "a two"]), ["a one", "a two"]);
        // Nothing typed: everything, in order.
        assert_eq!(titles(" ", &["b", "a"]), ["b", "a"]);
    }

    #[test]
    fn every_word_has_to_be_found_and_other_texts_find_without_marking() {
        let terminal = Candidate::new("diff perf").with("api-server");
        assert!(score("diff api", &terminal).is_some());
        assert!(score("diff zebra", &terminal).is_none());
        let found = score("perf api", &terminal).expect("found");
        assert_eq!(found.ranges.as_slice(), std::slice::from_ref(&(5..9)), "only the title is marked");
        // Two words side by side make one mark.
        let found = score("new tab", &Candidate::new("New tab")).expect("found");
        assert_eq!(found.ranges, [0..3, 4..7]);
        let title = score("diff perf", &Candidate::new("diff perf")).expect("found");
        let other = score("api", &terminal).expect("found");
        assert!(title.score / 2 > other.score, "a word in the title counts for more");
    }

    #[test]
    fn groups_come_in_a_fixed_order_and_are_cut_when_everything_is_searched() {
        let ranked =
            |n: usize| (0..n).map(|index| Ranked { index, found: Default::default() }).collect::<Vec<_>>();
        let found = || {
            vec![
                (Group::Commits, ranked(2)),
                (Group::Terminals, ranked(1)),
                (Group::Folders, ranked(0)),
                (Group::Settings, ranked(3)),
                (Group::Actions, ranked(12)),
            ]
        };
        let all = sections(Scope::All, found());
        let groups: Vec<Group> = all.iter().map(|s| s.group).collect();
        assert_eq!(
            groups,
            [Group::Settings, Group::Actions, Group::Terminals, Group::Commits],
            "empty ones go"
        );
        let actions = &all[1];
        assert_eq!((actions.total, actions.shown.len()), (12, GROUP_MAX));
        // A scope shows its own group, all of it.
        let scoped = sections(Scope::Actions, found());
        assert_eq!(scoped.len(), 1);
        assert_eq!((scoped[0].group, scoped[0].shown.len()), (Group::Actions, 12));
        assert_eq!(Scope::Commits.group(), Some(Group::Commits));
        assert_eq!(Scope::All.group(), None);
    }
}
