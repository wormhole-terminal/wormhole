//! Everything about one commit that the History list leaves out: the whole message, who wrote and who
//! committed it and when, every parent, the branches and tags that point at it. Read when a commit is
//! opened, not for the whole log: the list stays one cheap record per commit.
//!
//! A repository is someone else's input, so nothing here trusts what git prints: the fields are
//! NUL-separated with the message last (it may hold any other character), the message is capped, and
//! control characters are replaced before anything reaches a screen.

use std::path::Path;

use anyhow::Result;

use super::message::{self, Body, Contact, plain};
use super::{FileChange, GitCmd, GitError};

/// What git may print for the other fields and the message together before the message is given up
/// on (a hostile commit can carry megabytes), after which only its subject is read.
const READ_CAP: u64 = 1 << 20;

/// Refs and parents shown at most: a commit that every release script tagged, or an octopus merge.
const LIST_CAP: usize = 32;

/// A name, an address and a time that git recorded for the author or the committer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    pub name: String,
    pub email: String,
    /// Unix timestamp (seconds).
    pub time: i64,
    /// The time zone it was written in, in seconds east of UTC.
    pub offset: i32,
}

impl Signature {
    /// `Name <email>`, as git writes it in a trailer or a log line; the name alone when there is no address.
    pub fn line(&self) -> String {
        if self.email.is_empty() { self.name.clone() } else { format!("{} <{}>", self.name, self.email) }
    }

    /// The time as its writer's clock showed it, "2026-10-27 14:05 +02:00": numeric, so it reads the same in
    /// every language (the app has no month names to translate, and no time zone database to show it in
    /// the reader's zone).
    pub fn when(&self) -> String {
        let local = self
            .time
            .checked_add(i64::from(self.offset))
            .and_then(|t| chrono::DateTime::from_timestamp(t, 0));
        let Some(local) = local else { return String::new() };
        let sign = if self.offset < 0 { '-' } else { '+' };
        let minutes = self.offset.unsigned_abs() / 60;
        format!("{} {sign}{:02}:{:02}", local.format("%Y-%m-%d %H:%M"), minutes / 60, minutes % 60)
    }
}

/// What a name in [`CommitDetails::refs`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    /// HEAD is at this commit (attached or detached).
    Head,
    Branch,
    /// A remote-tracking branch (`origin/main`).
    Remote,
    Tag,
}

/// A branch, tag or HEAD that points at a commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitRef {
    pub kind: RefKind,
    /// `main`, `origin/main`, `v1.2.0`, `HEAD`.
    pub name: String,
}

/// One commit in full.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitDetails {
    pub sha: String,
    /// Every parent, first one first: more than one for a merge, none for a root commit.
    pub parents: Vec<String>,
    /// The same parents as git abbreviates them (`%p`), for showing.
    pub parents_short: Vec<String>,
    pub author: Signature,
    pub committer: Signature,
    /// Branches, tags and HEAD pointing at the commit, as they were when it was read.
    pub refs: Vec<CommitRef>,
    /// The whole message, subject and body, without the newline git ends it with.
    pub message: String,
    /// The message was longer than [`MESSAGE_CAP`] (or than git was allowed to print): what is kept
    /// ends where it was cut.
    pub message_cut: bool,
    /// From the message's `Co-authored-by:` trailers.
    pub co_authors: Vec<Contact>,
}

impl CommitDetails {
    /// The message's first line.
    pub fn subject(&self) -> &str {
        message::subject(&self.message)
    }

    /// The message after the subject and the blank line that follows it; empty for a one-line message.
    pub fn body(&self) -> &str {
        message::body(&self.message)
    }

    /// The body ready to draw, compact or whole: see [`Body`].
    pub fn body_view(&self) -> Body {
        Body::of(self.body())
    }

    /// Whether the committer is someone else than the author, or committed at another time (an amend,
    /// a rebase, a cherry-pick): then both are worth showing.
    pub fn committer_differs(&self) -> bool {
        self.author != self.committer
    }

    /// More than one parent.
    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }
}

/// What a commit changed, in numbers: [`DiffStat::of`] its files.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiffStat {
    pub files: usize,
    pub added: u64,
    pub removed: u64,
}

impl DiffStat {
    /// The totals of `files`; a binary file counts as a file and adds no lines.
    pub fn of(files: &[FileChange]) -> Self {
        files.iter().fold(Self { files: files.len(), ..Self::default() }, |stat, file| Self {
            added: stat.added + u64::from(file.added.unwrap_or(0)),
            removed: stat.removed + u64::from(file.removed.unwrap_or(0)),
            ..stat
        })
    }
}

/// The fields of one commit, NUL-separated, the message last: it may hold any character but NUL.
/// `%ad` and `%cd` are only the time zones (`--date=format:%z`): the times are the Unix ones.
const FORMAT: &str = "%H%x00%P%x00%p%x00%an%x00%ae%x00%at%x00%ad%x00%cn%x00%ce%x00%ct%x00%cd%x00%D%x00";

/// How many fields [`FORMAT`] has, with the message.
const FIELDS: usize = 13;

/// The commit `sha` of the repository at `root` in full. No signature is checked (that can run gpg),
/// no diff is made, and notes and mailmaps are left out.
pub fn commit_details(root: &Path, sha: &str) -> Result<CommitDetails> {
    let too_large = |err: &anyhow::Error| matches!(GitError::of(err), GitError::OutputTooLarge { .. });
    // A message too big to print: its subject will do, and the details say it was cut. A subject too
    // big for that (it is one line, but a hostile repository may hold any) leaves no message at all.
    for message in ["%B", "%s", ""] {
        match read(root, sha, message) {
            Ok(raw) => {
                let mut details = parse_details(&raw);
                details.message_cut |= message != "%B";
                return Ok(details);
            }
            Err(err) if too_large(&err) => {}
            Err(err) => return Err(err),
        }
    }
    anyhow::bail!("git printed more than {READ_CAP} bytes for the commit's fields")
}

fn read(root: &Path, sha: &str, message: &str) -> Result<String> {
    let raw = GitCmd::new(root)
        .args(["show", "-s", "--no-show-signature", "--no-notes", "--decorate=full", "--encoding=UTF-8"])
        .arg("--date=format:%z")
        .arg(format!("--format={FORMAT}{message}"))
        .max_output(READ_CAP)
        // `^{commit}`: a tag's name shows the commit it names, not the tag object.
        .revs([format!("{sha}^{{commit}}")])
        .paths::<_, &str>([])
        .run()?;
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

/// A commit from [`FORMAT`] and a message; whatever is missing or garbled is left empty or zero.
pub(crate) fn parse_details(raw: &str) -> CommitDetails {
    let mut fields = raw.splitn(FIELDS, '\0');
    let mut next = || fields.next().unwrap_or("");
    let sha = next().trim().to_string();
    let parents = next().split_whitespace().take(LIST_CAP).map(str::to_string).collect();
    let parents_short = next().split_whitespace().take(LIST_CAP).map(|p| plain(p, 40)).collect();
    let author = signature(next(), next(), next(), next());
    let committer = signature(next(), next(), next(), next());
    let refs = parse_refs(next());
    let (message, message_cut) = message::clean(next());
    CommitDetails {
        sha,
        parents,
        parents_short,
        author,
        committer,
        refs,
        co_authors: message::co_authors(&message),
        message,
        message_cut,
    }
}

fn signature(name: &str, email: &str, time: &str, zone: &str) -> Signature {
    Signature {
        name: plain(name, 256),
        email: plain(email, 256),
        time: time.trim().parse().unwrap_or(0),
        offset: zone_offset(zone.trim()),
    }
}

/// `+0530` or `-0700` as seconds east of UTC; 0 for anything else.
fn zone_offset(zone: &str) -> i32 {
    let (sign, digits) = match zone.split_at_checked(1) {
        Some(("+", digits)) => (1, digits),
        Some(("-", digits)) => (-1, digits),
        _ => return 0,
    };
    let (Some(hours), Some(minutes)) = (digits.get(..2), digits.get(2..4)) else { return 0 };
    match (hours.parse::<i32>(), minutes.parse::<i32>()) {
        (Ok(h), Ok(m)) if h < 24 && m < 60 => sign * (h * 3600 + m * 60),
        _ => 0,
    }
}

/// `%D` with `--decorate=full`: `HEAD -> refs/heads/main, tag: refs/tags/v1, refs/remotes/origin/main`.
/// A ref name has no space, so ", " only ever separates two of them; refs of other kinds (a stash,
/// a pull request) are left out, as a remote's own `HEAD`.
fn parse_refs(decorations: &str) -> Vec<CommitRef> {
    let mut refs = Vec::new();
    for item in decorations.trim().split(", ") {
        let item = item.trim();
        let (head, item) = match item.strip_prefix("HEAD -> ") {
            Some(branch) => (true, branch),
            None => (false, item),
        };
        if head || item == "HEAD" {
            refs.push(CommitRef { kind: RefKind::Head, name: "HEAD".into() });
        }
        let named = if let Some(tag) = item.strip_prefix("tag: ") {
            tag.strip_prefix("refs/tags/").map(|name| (RefKind::Tag, name))
        } else if let Some(name) = item.strip_prefix("refs/heads/") {
            Some((RefKind::Branch, name))
        } else {
            item.strip_prefix("refs/remotes/")
                .filter(|name| !name.ends_with("/HEAD"))
                .map(|name| (RefKind::Remote, name))
        };
        if let Some((kind, name)) = named.filter(|(_, name)| !name.is_empty()) {
            refs.push(CommitRef { kind, name: plain(name, 256) });
        }
        if refs.len() >= LIST_CAP {
            break;
        }
    }
    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::message::MESSAGE_CAP;
    use crate::git::testing::{git, repo_with, write};
    use crate::git::{ChangeKind, FileChange};

    fn record(message: &str) -> String {
        format!(
            "{}\0{}\0{}\0Ada\0ada@example.com\01700000000\0+0530\0Bob\0bob@example.com\01700000500\0-0700\0{}\0{message}",
            "a".repeat(40),
            "b".repeat(40),
            "b".repeat(7),
            "HEAD -> refs/heads/main, tag: refs/tags/v1, refs/remotes/origin/main, refs/heads/feature/x"
        )
    }

    #[test]
    fn a_record_becomes_the_commit() {
        let details = parse_details(&record("fix: it\n\nThe body.\n"));
        assert_eq!(details.sha, "a".repeat(40));
        assert_eq!(details.parents, vec!["b".repeat(40)]);
        assert_eq!(details.parents_short, vec!["b".repeat(7)]);
        assert_eq!((details.author.name.as_str(), details.author.email.as_str()), ("Ada", "ada@example.com"));
        assert_eq!((details.author.time, details.committer.time), (1_700_000_000, 1_700_000_500));
        assert_eq!(details.subject(), "fix: it");
        assert_eq!(details.body(), "The body.");
        assert!(details.committer_differs());
        assert!(!details.is_merge());
        assert!(!details.message_cut);
    }

    #[test]
    fn decorations_say_what_points_here() {
        let refs = parse_details(&record("x")).refs;
        let shown: Vec<_> = refs.iter().map(|r| (r.kind, r.name.as_str())).collect();
        assert_eq!(
            shown,
            [
                (RefKind::Head, "HEAD"),
                (RefKind::Branch, "main"),
                (RefKind::Tag, "v1"),
                (RefKind::Remote, "origin/main"),
                (RefKind::Branch, "feature/x"),
            ]
        );
        // A detached HEAD, a remote's own HEAD and refs of other kinds.
        let refs = parse_refs("HEAD, refs/remotes/origin/HEAD, refs/stash, refs/pull/3/head");
        assert_eq!(refs, vec![CommitRef { kind: RefKind::Head, name: "HEAD".into() }]);
        assert!(parse_refs("").is_empty());
    }

    #[test]
    fn a_message_may_hold_any_separator() {
        let message = "subject\n\nwith \x1f and \x1e and \0 and, refs/heads/x\n";
        let details = parse_details(&record(message));
        // Only NUL ends the fields; what follows the first NUL of the message is still the message.
        assert_eq!(details.subject(), "subject");
        assert!(details.body().starts_with("with \u{fffd} and \u{fffd} and \u{fffd} and, refs/heads/x"));
        assert_eq!(details.refs.len(), 5);
    }

    #[test]
    fn a_huge_message_is_cut_in_the_details() {
        let message = format!("subject\n\n{}", "é".repeat(MESSAGE_CAP));
        let details = parse_details(&record(&message));
        assert!(details.message_cut);
        assert!(details.message.len() <= MESSAGE_CAP);
    }

    #[test]
    fn a_time_zone_is_read_and_shown() {
        assert_eq!(zone_offset("+0530"), 19_800);
        assert_eq!(zone_offset("-0700"), -25_200);
        for bad in ["", "0530", "+05", "+9999", "+0a00", "+05:30", "\u{fffd}0530"] {
            assert_eq!(zone_offset(bad), 0, "{bad:?}");
        }
        let details = parse_details(&record("x"));
        assert_eq!((details.author.offset, details.committer.offset), (19_800, -25_200));
        // 2023-11-14T22:13:20Z in +05:30 is the 15th, 03:43.
        assert_eq!(details.author.when(), "2023-11-15 03:43 +05:30");
        assert_eq!(details.committer.when(), "2023-11-14 15:21 -07:00");
        assert_eq!(Signature { time: i64::MAX, ..details.author }.when(), "");
    }

    #[test]
    fn short_or_garbled_records_never_panic() {
        for raw in ["", "\0", "abc\0def", "\0\0\0\0\0\0\0\0\0", "x\0y\0z\0w\0notanumber\0a\0b\0c\0d\0e"] {
            let details = parse_details(raw);
            assert_eq!(details.author.time.min(0), 0);
            let _ = (details.subject(), details.body());
        }
    }

    #[test]
    fn the_stat_adds_the_files_up() {
        let file =
            |added, removed| FileChange { added, removed, ..FileChange::new("f", ChangeKind::Modified) };
        let stat = DiffStat::of(&[file(Some(3), Some(1)), file(None, None), file(Some(10), Some(0))]);
        assert_eq!(stat, DiffStat { files: 3, added: 13, removed: 1 });
        assert_eq!(DiffStat::of(&[]), DiffStat::default());
    }

    fn commit_all(root: &Path, message: &str) {
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", message]);
    }

    /// Commit everything with a message read from a file (a long one does not fit on a command line).
    fn commit_all_from(root: &Path, message: &str) {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("message.txt");
        std::fs::write(&file, message).expect("write the message");
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-F", file.to_str().expect("utf-8 path")]);
    }

    fn head(root: &Path) -> CommitDetails {
        commit_details(root, "HEAD").expect("details")
    }

    #[test]
    fn a_commit_is_read_in_full() {
        let repo = repo_with(&[("a.txt", "1\n")]);
        let root = repo.path();
        write(root, "a.txt", "2\n");
        commit_all(
            root,
            "feat(x): subject line\n\nFirst paragraph\nwraps.\n\nSecond one.\n\nCo-authored-by: Bob <bob@example.com>",
        );
        git(root, &["tag", "v1"]);
        git(root, &["tag", "-a", "v2", "-m", "annotated"]);
        git(root, &["branch", "side"]);
        let details = head(root);
        assert_eq!(details.subject(), "feat(x): subject line");
        assert_eq!(
            details.body(),
            "First paragraph\nwraps.\n\nSecond one.\n\nCo-authored-by: Bob <bob@example.com>"
        );
        assert_eq!(
            (details.author.name.as_str(), details.author.email.as_str()),
            ("Test", "test@example.com")
        );
        assert!(!details.committer_differs());
        assert_eq!(details.parents.len(), 1);
        assert_eq!(details.sha.len(), 40);
        assert_eq!(details.co_authors, vec![Contact { name: "Bob".into(), email: "bob@example.com".into() }]);
        let kinds: Vec<_> = details.refs.iter().map(|r| (r.kind, r.name.as_str())).collect();
        assert!(kinds.contains(&(RefKind::Head, "HEAD")), "{kinds:?}");
        assert!(kinds.contains(&(RefKind::Branch, "main")), "{kinds:?}");
        assert!(kinds.contains(&(RefKind::Branch, "side")), "{kinds:?}");
        assert!(kinds.contains(&(RefKind::Tag, "v1")), "{kinds:?}");
        assert!(kinds.contains(&(RefKind::Tag, "v2")), "an annotated tag shows its commit: {kinds:?}");
    }

    #[test]
    fn a_merge_lists_every_parent() {
        let repo = repo_with(&[("a.txt", "1\n")]);
        let root = repo.path();
        git(root, &["checkout", "-q", "-b", "topic"]);
        write(root, "b.txt", "b\n");
        commit_all(root, "on topic");
        git(root, &["checkout", "-q", "main"]);
        write(root, "c.txt", "c\n");
        commit_all(root, "on main");
        git(root, &["merge", "-q", "--no-ff", "topic", "-m", "Merge topic"]);
        let merge = head(root);
        assert!(merge.is_merge());
        assert_eq!(merge.parents.len(), 2);
        assert_ne!(merge.parents[0], merge.parents[1]);
        // A root commit has none.
        let root_commit = commit_details(root, &format!("{}~1", merge.parents[0])).expect("the first commit");
        assert!(root_commit.parents.is_empty());
        assert_eq!(root_commit.subject(), "initial");
    }

    #[test]
    fn a_committer_who_is_not_the_author_is_told() {
        let repo = repo_with(&[("a.txt", "1\n")]);
        let root = repo.path();
        write(root, "a.txt", "2\n");
        git(root, &["add", "-A"]);
        // The author comes from the options, the committer from the identity the helper sets.
        git(
            root,
            &[
                "commit",
                "-q",
                "-m",
                "theirs",
                "--author=Dee <dee@example.org>",
                "--date=2020-01-02T03:04:05Z",
            ],
        );
        let details = head(root);
        assert_eq!((details.author.name.as_str(), details.author.email.as_str()), ("Dee", "dee@example.org"));
        assert_eq!(details.author.time, 1_577_934_245);
        assert_eq!(details.author.when(), "2020-01-02 03:04 +00:00");
        assert_eq!(details.committer.name, "Test");
        assert!(details.committer_differs());
    }

    #[test]
    fn a_detached_head_is_still_head() {
        let repo = repo_with(&[("a.txt", "1\n")]);
        let root = repo.path();
        git(root, &["checkout", "-q", "--detach"]);
        let kinds: Vec<_> = head(root).refs.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&RefKind::Head));
    }

    #[test]
    fn a_hostile_message_is_capped_and_a_name_is_never_an_option() {
        let repo = repo_with(&[("a.txt", "1\n")]);
        let root = repo.path();
        write(root, "a.txt", "2\n");
        let huge = format!("subject\n\n{}\n", "x".repeat(MESSAGE_CAP * 2));
        commit_all_from(root, &huge);
        let details = head(root);
        assert!(details.message_cut);
        assert!(details.message.len() <= MESSAGE_CAP);
        assert_eq!(details.subject(), "subject");
        // Past what git is allowed to print, the subject is kept and the cut is said.
        write(root, "a.txt", "3\n");
        let enormous = format!("big subject\n\n{}\n", "y".repeat(3 << 20));
        commit_all_from(root, &enormous);
        let details = head(root);
        assert!(details.message_cut);
        assert_eq!(details.message, "big subject");
        // A subject that is itself past what git may print leaves no message, and still no error.
        write(root, "a.txt", "4\n");
        commit_all_from(root, &format!("{}\n", "z".repeat(2 << 20)));
        let details = head(root);
        assert!(details.message_cut);
        assert_eq!(details.message, "");
        assert_eq!(details.sha.len(), 40);
    }

    #[test]
    fn a_rev_that_looks_like_an_option_is_not_one() {
        let repo = repo_with(&[("a.txt", "1\n")]);
        let scratch = tempfile::tempdir().expect("tempdir");
        let target = scratch.path().join("written-by-git");
        assert!(commit_details(repo.path(), &format!("--output={}", target.display())).is_err());
        assert!(!target.exists(), "`--output` would have made git write there");
    }

    #[test]
    fn a_missing_commit_is_an_error_not_a_panic() {
        let repo = repo_with(&[("a.txt", "1\n")]);
        assert!(commit_details(repo.path(), &"0".repeat(40)).is_err());
        assert!(commit_details(repo.path(), "no-such-branch").is_err());
    }
}
