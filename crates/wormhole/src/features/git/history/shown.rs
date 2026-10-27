//! What the opened commit's block draws, made once when the commit's details are read (off the UI
//! thread) and kept with them: the strings, the labels VoiceOver reads, the facts with the spans that
//! tint them. A frame then lays out what is here and builds no text of its own, however long the message
//! or many the refs: the frame cost of the block is its element count, which is bounded.

use std::cell::RefCell;
use std::ops::Range;

use gpui_kit::SharedString;
use wormhole_core::git::{Body, CommitDetails, DiffStat, RefKind, Signature};
use wormhole_core::view::panel::ago_words;
use wormhole_core::view::spoken;
use wormhole_text::{keys, text, whole};

/// Branches and tags drawn as chips at most; the rest are counted.
pub(super) const REFS_SHOWN: usize = 8;
/// Parents drawn as links at most (a merge has two; an octopus merge more).
pub(super) const PARENTS_SHOWN: usize = 4;
/// Co-authors listed at most.
const CO_AUTHORS_SHOWN: usize = 4;

/// How a stretch of a fact's text is tinted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tone {
    Muted,
    Added,
    Removed,
}

/// A labelled fact of the block ("Author", "Changes"): its text in one piece, with the stretches of it
/// that are drawn in another color.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Fact {
    pub label: SharedString,
    pub text: SharedString,
    pub spans: Vec<(Range<usize>, Tone)>,
}

impl Fact {
    /// "Name email" with the date and its age under it: everything after the name is muted.
    fn person(label: &str, who: &Signature, now: i64) -> Self {
        let name_end = who.name.len();
        let who_line =
            if who.email.is_empty() { who.name.clone() } else { format!("{} {}", who.name, who.email) };
        let when = keys::history_date_ago(&who.when(), &ago_words(now - who.time));
        let text = format!("{who_line}\n{when}");
        let spans = vec![(name_end..text.len(), Tone::Muted)];
        Self { label: label.to_string().into(), text: text.into(), spans }
    }

    /// The co-authors, a line each, their addresses muted.
    fn co_authors(details: &CommitDetails) -> Option<Self> {
        let all = &details.co_authors;
        if all.is_empty() {
            return None;
        }
        let (mut text, mut spans) = (String::new(), Vec::new());
        for (ix, co) in all.iter().take(CO_AUTHORS_SHOWN).enumerate() {
            if ix > 0 {
                text.push('\n');
            }
            text.push_str(&co.name);
            let from = text.len();
            text.push(' ');
            text.push_str(&co.email);
            spans.push((from..text.len(), Tone::Muted));
        }
        if let Some(more) = all.len().checked_sub(CO_AUTHORS_SHOWN).filter(|&more| more > 0) {
            let from = text.len();
            text.push('\n');
            text.push_str(&keys::history_more_co_authors(&more.to_string()));
            spans.push((from..text.len(), Tone::Muted));
        }
        let label = text_of(if all.len() == 1 { keys::HISTORY_CO_AUTHOR } else { keys::HISTORY_CO_AUTHORS });
        Some(Self { label, text: text.into(), spans })
    }

    /// "2 files +4 −1", the numbers in the colors of the file rows.
    pub(super) fn changes(stat: DiffStat) -> Self {
        let (added, removed) = (format!("+{}", stat.added), format!("\u{2212}{}", stat.removed));
        let text = keys::history_changes_line(&keys::history_files(whole(stat.files)), &added, &removed);
        // The numbers are found where the message put them: the removed ones are the last in it.
        let removed_at = text.rfind(&removed).unwrap_or(text.len());
        let added_at = text[..removed_at].rfind(&added).unwrap_or(removed_at);
        let spans = vec![
            (added_at..added_at + added.len(), Tone::Added),
            (removed_at..removed_at + removed.len(), Tone::Removed),
        ];
        Self { label: text_of(keys::HISTORY_CHANGES), text: text.into(), spans }
    }
}

fn text_of(key: wormhole_text::Key) -> SharedString {
    text(key).to_string().into()
}

/// A parent: its hash, as git abbreviates it, and what VoiceOver calls the link.
#[derive(Clone, Debug)]
pub(super) struct Parent {
    pub sha: String,
    pub short: SharedString,
    pub spoken: SharedString,
}

/// The compact text for one width, kept: the text, and whether it leaves anything out.
#[derive(Clone, Debug)]
struct Compacted {
    line_chars: usize,
    text: SharedString,
    more: bool,
}

/// An opened commit as its block draws it.
#[derive(Debug)]
pub(super) struct Shown {
    pub details: CommitDetails,
    pub subject: SharedString,
    pub body: Body,
    /// The body's pieces as strings the text system can take without copying them.
    pub whole: SharedString,
    pub trailers: SharedString,
    /// The body is longer than what is drawn: a line says so under it when it is shown whole (a copy has
    /// all of it).
    pub body_cut: bool,
    /// The message is longer than Wormhole reads: a copy is cut too, and a line says so.
    pub message_cut: Option<SharedString>,
    /// What VoiceOver reads for the block.
    pub spoken: SharedString,
    /// Author, committer (when it differs) and co-authors.
    pub people: Vec<Fact>,
    pub parents_label: SharedString,
    pub parents: Vec<Parent>,
    pub parents_more: Option<SharedString>,
    pub refs: Vec<(RefKind, SharedString)>,
    pub refs_more: Option<SharedString>,
    compacted: RefCell<Option<Compacted>>,
}

impl Shown {
    /// The block of `details`, with their ages as of `now` (they are read again at each opening).
    pub(super) fn new(details: CommitDetails, now: i64) -> Self {
        let body = details.body_view();
        let short: String = details.sha.chars().take(7).collect();
        let subject: SharedString = details.shown_subject().into();
        let mut people = vec![Fact::person(text(keys::HISTORY_AUTHOR), &details.author, now)];
        if details.committer_differs() {
            people.push(Fact::person(text(keys::HISTORY_COMMITTER), &details.committer, now));
        }
        people.extend(Fact::co_authors(&details));
        let parents: Vec<Parent> = details
            .parents
            .iter()
            .zip(&details.parents_short)
            .take(PARENTS_SHOWN)
            .map(|(sha, short)| Parent {
                sha: sha.clone(),
                short: short.clone().into(),
                spoken: keys::history_open_parent(short).into(),
            })
            .collect();
        let parents_more = (details.parents.len() > PARENTS_SHOWN)
            .then(|| keys::history_more_parents(&(details.parents.len() - PARENTS_SHOWN).to_string()).into());
        let refs: Vec<_> = details
            .refs
            .iter()
            .take(REFS_SHOWN)
            .map(|r| (r.kind, SharedString::from(r.name.clone())))
            .collect();
        let refs_more = (details.refs.len() > REFS_SHOWN)
            .then(|| keys::history_more_refs(&(details.refs.len() - REFS_SHOWN).to_string()).into());
        let spoken = spoken::commit_details(
            &short,
            &details.shown_subject(),
            &details.author.name,
            &details.author.when(),
            details.parents.len(),
        );
        Self {
            subject,
            whole: body.text.clone().into(),
            trailers: body.trailers.clone().into(),
            body_cut: body.cut,
            message_cut: details.message_cut.then(|| keys::history_message_too_long(&short).into()),
            spoken: spoken.into(),
            people,
            parents_label: text_of(if details.parents.len() == 1 {
                keys::HISTORY_PARENT
            } else {
                keys::HISTORY_PARENTS
            }),
            parents,
            parents_more,
            refs,
            refs_more,
            compacted: RefCell::new(None),
            body,
            details,
        }
    }

    /// The compact text for lines of about `line_chars` characters, and whether it leaves anything out
    /// (which is what "Show more" is offered for). Made once per width.
    pub(super) fn compact(&self, line_chars: usize) -> (SharedString, bool) {
        let mut kept = self.compacted.borrow_mut();
        if let Some(c) = kept.as_ref().filter(|c| c.line_chars == line_chars) {
            return (c.text.clone(), c.more);
        }
        let more = self.body.more(line_chars) || self.body_cut || self.message_cut.is_some();
        let text: SharedString = self.body.compact(line_chars).into();
        *kept = Some(Compacted { line_chars, text: text.clone(), more });
        (text, more)
    }

    /// Whether the last frame offered "Show more": what the menu on the block lists.
    pub(super) fn more_offered(&self) -> bool {
        self.compacted.borrow().as_ref().is_some_and(|c| c.more)
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::git::{CommitDetails, Signature};
    use wormhole_text::{Language, with_language};

    use super::{Fact, Shown, Tone};

    fn details() -> CommitDetails {
        let author = Signature {
            name: "Ada".into(),
            email: "ada@example.com".into(),
            time: 1_700_000_000,
            offset: 19_800,
        };
        let committer = Signature { name: "Bob".into(), ..author.clone() };
        CommitDetails {
            sha: "a".repeat(40),
            parents: vec!["b".repeat(40)],
            parents_short: vec!["b".repeat(7)],
            author,
            committer,
            refs: Vec::new(),
            message: "fix: it\n\nThe body.".into(),
            message_cut: false,
            co_authors: Vec::new(),
        }
    }

    #[test]
    fn it_is_built_once_with_everything_the_block_draws() {
        with_language(Language::ENGLISH, || {
            let shown = Shown::new(details(), 1_700_100_000);
            assert_eq!(shown.subject, "fix: it");
            assert_eq!(shown.people[0].label, "Author");
            assert!(shown.people[0].text.starts_with("Ada ada@example.com\n2023-11-15 03:43 +05:30 · 1d"));
            // The address and the date are muted, the name is not.
            assert_eq!(shown.people[0].spans[0].0.start, "Ada".len());
            assert_eq!(shown.people[0].spans[0].1, Tone::Muted);
            assert_eq!(shown.people[1].label, "Committer");
            assert_eq!(shown.parents[0].short, "bbbbbbb");
            assert_eq!(shown.parents_label, "Parent");
            assert!(shown.spoken.starts_with("Details of aaaaaaa, fix: it, by Ada"));
            let (compact, more) = shown.compact(40);
            assert_eq!(compact, "The body.");
            assert!(!more);
        });
    }

    #[test]
    fn the_changes_fact_tints_its_numbers() {
        with_language(Language::ENGLISH, || {
            let fact = Fact::changes(wormhole_core::git::DiffStat { files: 2, added: 4, removed: 1 });
            assert_eq!(fact.text, "2 files +4 \u{2212}1");
            let tinted: Vec<_> = fact.spans.iter().map(|(r, t)| (&fact.text[r.clone()], *t)).collect();
            assert_eq!(tinted, [("+4", Tone::Added), ("\u{2212}1", Tone::Removed)]);
        });
    }
}
