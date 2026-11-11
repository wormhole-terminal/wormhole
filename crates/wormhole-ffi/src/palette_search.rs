//! The command palette's search (core's `view::search`): which scope a query asks for, how well a result matches
//! it (fuzzy, with the characters to mark), and the order results come in. The palette gathers its results and
//! draws them; this decides what is found. Ranges are **byte** ranges of the title.

use wormhole_core::view::search::{self, Candidate, Group, Match, Ranked, Scope};

use crate::folder_picker::ByteRangeRecord;

/// What the palette searches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum PaletteScopeRecord {
    All,
    Terminals,
    Folders,
    Commits,
    Settings,
    Actions,
}

impl From<Scope> for PaletteScopeRecord {
    fn from(scope: Scope) -> Self {
        match scope {
            Scope::All => Self::All,
            Scope::Terminals => Self::Terminals,
            Scope::Folders => Self::Folders,
            Scope::Commits => Self::Commits,
            Scope::Settings => Self::Settings,
            Scope::Actions => Self::Actions,
        }
    }
}

impl From<PaletteScopeRecord> for Scope {
    fn from(scope: PaletteScopeRecord) -> Self {
        match scope {
            PaletteScopeRecord::All => Self::All,
            PaletteScopeRecord::Terminals => Self::Terminals,
            PaletteScopeRecord::Folders => Self::Folders,
            PaletteScopeRecord::Commits => Self::Commits,
            PaletteScopeRecord::Settings => Self::Settings,
            PaletteScopeRecord::Actions => Self::Actions,
        }
    }
}

/// How well a text matched, and the bytes to mark.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PaletteMatchRecord {
    pub score: i32,
    pub ranges: Vec<ByteRangeRecord>,
}

impl From<&Match> for PaletteMatchRecord {
    fn from(found: &Match) -> Self {
        Self {
            score: found.score,
            ranges: found
                .ranges
                .iter()
                .map(|r| ByteRangeRecord { start: r.start as u32, end: r.end as u32 })
                .collect(),
        }
    }
}

/// What a result is searched by: its title (marked) and other texts that find it too (a terminal's folder).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PaletteCandidateRecord {
    pub title: String,
    pub other: Vec<String>,
}

/// One candidate a query found: its place in what was given, and how.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PaletteRankedRecord {
    pub index: u32,
    pub found: PaletteMatchRecord,
}

impl From<&Ranked> for PaletteRankedRecord {
    fn from(ranked: &Ranked) -> Self {
        Self { index: ranked.index as u32, found: PaletteMatchRecord::from(&ranked.found) }
    }
}

/// How well `word` matches `text`: its letters in order, the first at a word start unless the word is in the text
/// as typed.
#[uniffi::export]
pub fn palette_fuzzy(word: String, text: String) -> Option<PaletteMatchRecord> {
    search::fuzzy(&word, &text).as_ref().map(PaletteMatchRecord::from)
}

/// The candidates `query` finds, best first (equal ones keep their order); an empty query finds them all in order.
#[uniffi::export]
pub fn palette_rank(query: String, candidates: Vec<PaletteCandidateRecord>) -> Vec<PaletteRankedRecord> {
    let found = search::rank(
        &query,
        candidates
            .iter()
            .map(|c| Candidate { title: &c.title, other: c.other.iter().map(String::as_str).collect() }),
    );
    found.iter().map(PaletteRankedRecord::from).collect()
}

/// The scope `input` asks for and the query after its prefix.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PaletteParsedRecord {
    pub scope: PaletteScopeRecord,
    pub query: String,
}

#[uniffi::export]
pub fn palette_parse(input: String) -> PaletteParsedRecord {
    let (scope, query) = search::parse(&input);
    PaletteParsedRecord { scope: scope.into(), query: query.to_string() }
}

/// A scope as the scope row shows it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PaletteScopeInfoRecord {
    pub scope: PaletteScopeRecord,
    pub prefix: Option<String>,
    pub title: String,
    pub placeholder: String,
}

/// The scope row, left to right.
#[uniffi::export]
pub fn palette_scopes() -> Vec<PaletteScopeInfoRecord> {
    Scope::ROW
        .into_iter()
        .map(|scope| PaletteScopeInfoRecord {
            scope: scope.into(),
            prefix: scope.prefix().map(String::from),
            title: scope.title().to_string(),
            placeholder: scope.placeholder().to_string(),
        })
        .collect()
}

/// The scope Tab (`forward`) or ⇧Tab moves to.
#[uniffi::export]
pub fn palette_step_scope(scope: PaletteScopeRecord, forward: bool) -> PaletteScopeRecord {
    Scope::from(scope).step(forward).into()
}

/// How many results of each group the unscoped palette shows.
#[uniffi::export]
pub fn palette_group_max() -> u32 {
    search::GROUP_MAX as u32
}

/// The groups in the order an unscoped palette shows them.
#[uniffi::export]
pub fn palette_group_order() -> Vec<String> {
    Group::ORDER.iter().map(|group| group.title().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates(titles: &[&str]) -> Vec<PaletteCandidateRecord> {
        titles.iter().map(|t| PaletteCandidateRecord { title: (*t).into(), other: vec![] }).collect()
    }

    #[test]
    fn ranges_mark_the_matched_bytes_and_a_prefix_beats_a_word_start_beats_scattered() {
        let found = palette_fuzzy("hsid".into(), "Hide Sidebar".into()).expect("found");
        assert_eq!(
            found.ranges,
            [ByteRangeRecord { start: 0, end: 1 }, ByteRangeRecord { start: 5, end: 8 }]
        );
        let score = |text: &str| palette_fuzzy("sid".into(), text.into()).map(|m| m.score).expect(text);
        assert!(
            score("Sidebar polish") > score("Hide Sidebar") && score("Hide Sidebar") > score("Inside out")
        );
        assert!(palette_fuzzy("ts".into(), "Terminals".into()).is_none());
    }

    #[test]
    fn marks_are_bytes_not_characters() {
        let found = palette_fuzzy("b".into(), "Ünï b".into()).expect("found");
        assert_eq!(
            found.ranges,
            [ByteRangeRecord { start: 6, end: 7 }],
            "after two two-byte letters and a space"
        );
    }

    #[test]
    fn ranking_is_best_first_with_ties_in_order_and_other_text_counts_for_less() {
        let ranked =
            palette_rank("sid".into(), candidates(&["Hide Sidebar", "Sidebar polish", "Inside out", "nope"]));
        let order: Vec<u32> = ranked.iter().map(|r| r.index).collect();
        assert_eq!(order, [1, 0, 2]);
        assert_eq!(
            palette_rank(String::new(), candidates(&["b", "a"])).iter().map(|r| r.index).collect::<Vec<_>>(),
            [0, 1]
        );
        let with_other = vec![
            PaletteCandidateRecord { title: "Go to build".into(), other: vec!["api".into()] },
            PaletteCandidateRecord { title: "api server".into(), other: vec![] },
        ];
        let ranked = palette_rank("api".into(), with_other);
        assert_eq!(ranked[0].index, 1, "in the title beats in another text");
        assert!(ranked[1].found.ranges.is_empty(), "another text is not marked");
        assert!(
            palette_rank(
                "go zzz".into(),
                vec![PaletteCandidateRecord { title: "Go to build".into(), other: vec!["api".into()] }]
            )
            .is_empty()
        );
    }

    #[test]
    fn a_prefix_picks_the_scope_and_tab_walks_the_row() {
        assert_eq!(
            palette_parse("@diff".into()),
            PaletteParsedRecord { scope: PaletteScopeRecord::Terminals, query: "diff".into() }
        );
        assert_eq!(palette_parse("/style.rs".into()).scope, PaletteScopeRecord::All);
        assert_eq!(palette_parse("a@b".into()).query, "a@b");
        let row = palette_scopes();
        assert_eq!(
            row.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(),
            ["All", "Terminals", "Folders", "Commits", "Settings", "Actions"]
        );
        assert_eq!(palette_step_scope(PaletteScopeRecord::Actions, true), PaletteScopeRecord::All);
        assert_eq!(palette_step_scope(PaletteScopeRecord::All, false), PaletteScopeRecord::Actions);
        assert_eq!(palette_group_max(), 5);
        assert_eq!(palette_group_order().first().map(String::as_str), Some("Settings"));
    }
}
