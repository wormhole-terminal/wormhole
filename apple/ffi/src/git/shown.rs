//! An opened commit as the History block draws it: core's details turned into the words, labelled facts and
//! capped lists the GPUI app's `Shown` makes, so the Swift view lays out what is here and decides nothing
//! (which lines are muted, what "+3" stands for, how a changed-files line reads in the language).

use wormhole_core::git::{Body, CommitDetails, DiffStat, FileChange, Signature};
use wormhole_core::view::panel::ago_words;
use wormhole_core::view::spoken;
use wormhole_text::{keys, text, whole};

use super::records::{GitChange, GitRef};

/// Branches and tags drawn at most; the rest are counted.
const REFS_SHOWN: usize = 8;
/// Parents drawn as links at most (a merge has two; an octopus merge more).
const PARENTS_SHOWN: usize = 4;
/// Co-authors listed at most.
const CO_AUTHORS_SHOWN: usize = 4;

/// How a stretch of a fact's text is tinted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum GitTone {
    Plain,
    Muted,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitSpan {
    pub text: String,
    pub tone: GitTone,
}

/// A labelled fact ("Author", "Changes"): its text as stretches in the colours they are drawn in.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitFact {
    pub label: String,
    pub spans: Vec<GitSpan>,
}

/// A parent: its hash, as git abbreviates it, and what VoiceOver calls the link.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitParent {
    pub sha: String,
    pub short: String,
    pub spoken: String,
}

/// An opened commit: what its block draws, and its files against the first parent.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GitCommitDetails {
    pub sha: String,
    /// The subject as a screen shows it.
    pub subject: String,
    /// What VoiceOver reads for the block.
    pub spoken: String,
    /// The whole message as git gave it: what "Copy message" puts on the clipboard.
    pub message: String,
    /// `Name <email>`: what "Copy author" copies.
    pub author_line: String,
    /// Author, committer (when it differs) and co-authors.
    pub people: Vec<GitFact>,
    pub parents_label: String,
    pub parents: Vec<GitParent>,
    pub parents_more: Option<String>,
    pub refs: Vec<GitRef>,
    pub refs_more: Option<String>,
    /// The body's prose and its trailers, cleaned for the screen, for the whole view.
    pub whole: String,
    pub trailers: String,
    /// The line that says the body goes on past what is drawn.
    pub continues: Option<String>,
    /// The line that says the message was longer than Wormhole reads (a copy is cut too).
    pub too_long: Option<String>,
    /// "2 files +4 −1", the numbers tinted as the file rows tint them.
    pub changes: GitFact,
    pub files: Vec<GitChange>,
}

/// What the compact message is for one line width, and whether it leaves anything out (what "Show more" is for).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitCompact {
    pub text: String,
    pub more: bool,
}

/// The body of an opened commit kept for `compact`, with whether anything is cut besides what compact leaves out.
#[derive(Clone)]
pub(crate) struct Message {
    pub(crate) body: Body,
    cut: bool,
}

impl Message {
    pub(crate) fn of(details: &CommitDetails) -> Self {
        let body = details.body_view();
        Self { cut: body.cut || details.message_cut, body }
    }

    pub(crate) fn compact(&self, line_chars: usize) -> GitCompact {
        GitCompact { text: self.body.compact(line_chars), more: self.body.more(line_chars) || self.cut }
    }
}

fn plain(text: impl Into<String>, tone: GitTone) -> GitSpan {
    GitSpan { text: text.into(), tone }
}

/// "Name email" with the date and its age under it: everything after the name is muted.
fn person(label: &str, who: &Signature, now: i64) -> GitFact {
    let rest = if who.email.is_empty() { String::new() } else { format!(" {}", who.email) };
    let when = keys::history_date_ago(&who.when(), &ago_words(now - who.time));
    GitFact {
        label: label.to_string(),
        spans: vec![plain(&who.name, GitTone::Plain), plain(format!("{rest}\n{when}"), GitTone::Muted)],
    }
}

/// The co-authors, a line each, their addresses muted.
fn co_authors(details: &CommitDetails) -> Option<GitFact> {
    let all = &details.co_authors;
    if all.is_empty() {
        return None;
    }
    let mut spans = Vec::new();
    for (ix, co) in all.iter().take(CO_AUTHORS_SHOWN).enumerate() {
        spans.push(plain(if ix > 0 { format!("\n{}", co.name) } else { co.name.clone() }, GitTone::Plain));
        spans.push(plain(format!(" {}", co.email), GitTone::Muted));
    }
    if let Some(more) = all.len().checked_sub(CO_AUTHORS_SHOWN).filter(|&more| more > 0) {
        spans.push(plain(format!("\n{}", keys::history_more_co_authors(&more.to_string())), GitTone::Muted));
    }
    let label = text(if all.len() == 1 { keys::HISTORY_CO_AUTHOR } else { keys::HISTORY_CO_AUTHORS });
    Some(GitFact { label: label.to_string(), spans })
}

/// "2 files +4 −1", the numbers in the colours of the file rows.
fn changes(stat: DiffStat) -> GitFact {
    let (added, removed) = (format!("+{}", stat.added), format!("\u{2212}{}", stat.removed));
    let line = keys::history_changes_line(&keys::history_files(whole(stat.files)), &added, &removed);
    // The numbers are found where the message put them: the removed ones are the last in it.
    let removed_at = line.rfind(&removed).unwrap_or(line.len());
    let added_at = line[..removed_at].rfind(&added).unwrap_or(removed_at);
    let between = &line[(added_at + added.len()).min(removed_at)..removed_at];
    let spans = vec![
        plain(&line[..added_at], GitTone::Plain),
        plain(added, GitTone::Added),
        plain(between, GitTone::Plain),
        plain(&line[removed_at.min(line.len())..], GitTone::Removed),
    ];
    GitFact { label: text(keys::HISTORY_CHANGES).to_string(), spans }
}

impl GitCommitDetails {
    /// `details` and the files they changed, with ages as of `now`.
    pub(crate) fn of(details: &CommitDetails, files: &[FileChange], now: i64) -> Self {
        let body = details.body_view();
        let short: String = details.sha.chars().take(7).collect();
        let mut people = vec![person(text(keys::HISTORY_AUTHOR), &details.author, now)];
        if details.committer_differs() {
            people.push(person(text(keys::HISTORY_COMMITTER), &details.committer, now));
        }
        people.extend(co_authors(details));
        let parents = details
            .parents
            .iter()
            .zip(&details.parents_short)
            .take(PARENTS_SHOWN)
            .map(|(sha, short)| GitParent {
                sha: sha.clone(),
                short: short.clone(),
                spoken: keys::history_open_parent(short),
            })
            .collect();
        let more = |all: usize, shown: usize, words: fn(&str) -> String| {
            (all > shown).then(|| words(&(all - shown).to_string()))
        };
        Self {
            sha: details.sha.clone(),
            subject: details.shown_subject(),
            spoken: spoken::commit_details(
                &short,
                &details.shown_subject(),
                &details.author.name,
                &details.author.when(),
                details.parents.len(),
            ),
            message: details.message.clone(),
            author_line: details.author.line(),
            people,
            parents_label: text(if details.parents.len() == 1 {
                keys::HISTORY_PARENT
            } else {
                keys::HISTORY_PARENTS
            })
            .to_string(),
            parents,
            parents_more: more(details.parents.len(), PARENTS_SHOWN, keys::history_more_parents),
            refs: details.refs.iter().take(REFS_SHOWN).map(GitRef::from).collect(),
            refs_more: more(details.refs.len(), REFS_SHOWN, keys::history_more_refs),
            whole: body.text,
            trailers: body.trailers,
            continues: body.cut.then(|| text(keys::HISTORY_MESSAGE_CONTINUES).to_string()),
            too_long: details.message_cut.then(|| keys::history_message_too_long(&short)),
            changes: changes(DiffStat::of(files)),
            files: files.iter().map(GitChange::of).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::git::Contact;
    use wormhole_text::{Language, set_language};

    use super::*;

    fn details() -> CommitDetails {
        let author = Signature {
            name: "Ada".into(),
            email: "ada@example.com".into(),
            time: 1_700_000_000,
            offset: 19_800,
        };
        CommitDetails {
            sha: "a".repeat(40),
            parents: vec!["b".repeat(40)],
            parents_short: vec!["b".repeat(7)],
            committer: Signature { name: "Bob".into(), ..author.clone() },
            author,
            refs: Vec::new(),
            message: "fix: it\n\nThe body.\n\nCo-authored-by: Cy <cy@example.com>".into(),
            message_cut: false,
            co_authors: vec![Contact { name: "Cy".into(), email: "cy@example.com".into() }],
        }
    }

    fn joined(fact: &GitFact) -> String {
        fact.spans.iter().map(|span| span.text.as_str()).collect()
    }

    #[test]
    fn the_block_has_everything_it_draws() {
        set_language(Language::ENGLISH);
        {
            let shown = GitCommitDetails::of(&details(), &[], 1_700_100_000);
            assert_eq!(shown.subject, "fix: it");
            assert_eq!(
                shown.people.iter().map(|p| p.label.as_str()).collect::<Vec<_>>(),
                ["Author", "Committer", "Co-author"]
            );
            assert_eq!(
                joined(&shown.people[0]),
                "Ada ada@example.com\n2023-11-15 03:43 +05:30 · 1d\u{a0}ago"
            );
            // The name is plain, the address and the date are muted.
            assert_eq!(shown.people[0].spans[0], plain("Ada", GitTone::Plain));
            assert_eq!(shown.people[0].spans[1].tone, GitTone::Muted);
            assert_eq!(
                (shown.parents_label.as_str(), shown.parents[0].short.as_str()),
                ("Parent", "bbbbbbb")
            );
            assert!(shown.spoken.starts_with("Details of aaaaaaa, fix: it, by Ada"));
            assert_eq!(shown.author_line, "Ada <ada@example.com>");
            assert_eq!(shown.message, details().message);
        }
    }

    #[test]
    fn the_changes_fact_tints_its_numbers_and_lists_stay_capped() {
        set_language(Language::ENGLISH);
        {
            let fact = changes(DiffStat { files: 2, added: 4, removed: 1 });
            assert_eq!(joined(&fact), "2 files +4 \u{2212}1");
            let tinted: Vec<_> = fact
                .spans
                .iter()
                .filter(|s| s.tone != GitTone::Plain)
                .map(|s| (s.text.as_str(), s.tone))
                .collect();
            assert_eq!(tinted, [("+4", GitTone::Added), ("\u{2212}1", GitTone::Removed)]);

            let mut many = details();
            many.parents = (0..6).map(|n| n.to_string().repeat(40)).collect();
            many.parents_short = (0..6).map(|n| n.to_string().repeat(7)).collect();
            let shown = GitCommitDetails::of(&many, &[], 0);
            assert_eq!((shown.parents.len(), shown.parents_more.as_deref()), (PARENTS_SHOWN, Some("+2")));
            assert_eq!(shown.parents_label, "Parents");
        }
    }

    #[test]
    fn compact_says_when_something_is_left_out() {
        let mut long = details();
        long.message = format!("subject\n\n{}", "word ".repeat(80));
        let message = Message::of(&long);
        let narrow = message.compact(30);
        assert!(narrow.more && narrow.text.ends_with('\u{2026}'));
        let mut short = details();
        short.message = "s\n\nShort body.".into();
        assert!(!Message::of(&short).compact(80).more, "a short body shows whole");
        assert!(Message::of(&details()).compact(80).more, "trailers are left out of the compact view");
    }
}
