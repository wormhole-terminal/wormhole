//! What the History tab puts on the clipboard for a commit: its hash, short hash, message, subject or
//! author. The hash, short hash and subject are in the log already; the message and the author's address
//! need the commit's details, which are read first if they are not here yet (a copy from the menu of a
//! commit that was never opened). The clipboard is `platform::clipboard`'s: a test instance's own.

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::git::{Commit, CommitDetails};
use wormhole_core::view::notice::{Copied, Failed};

use super::History;
use super::state::{COPY_PATIENCE, PendingCopy};

/// One of the things a commit offers to copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CopyWhat {
    /// The full 40-character hash.
    Hash,
    ShortHash,
    /// The whole message, subject and body.
    Message,
    Subject,
    /// `Name <email>`.
    Author,
}

impl CopyWhat {
    /// Every kind, in the order the menu lists them.
    pub(crate) const ALL: [Self; 5] =
        [Self::Hash, Self::ShortHash, Self::Message, Self::Subject, Self::Author];

    /// The notice that says it was copied.
    fn notice(self) -> Copied {
        match self {
            Self::Hash => Copied::CommitHash,
            Self::ShortHash => Copied::CommitShortHash,
            Self::Message => Copied::CommitMessage,
            Self::Subject => Copied::CommitSubject,
            Self::Author => Copied::CommitAuthor,
        }
    }

    /// The text, if what it needs is here: `details` only for the message and the author.
    pub(crate) fn text(self, commit: &Commit, details: Option<&CommitDetails>) -> Option<String> {
        Some(match self {
            Self::Hash => commit.sha.clone(),
            Self::ShortHash => commit.short.clone(),
            // What the heading shows: the message's first line once it is read, the log's subject until then.
            Self::Subject => details.map_or_else(|| commit.subject.clone(), |d| d.subject().to_string()),
            Self::Message => details?.message.clone(),
            Self::Author => details?.author.line(),
        })
    }

    /// Whether the text comes from the details.
    fn needs_details(self) -> bool {
        matches!(self, Self::Message | Self::Author)
    }
}

impl History {
    /// Puts `what` of the commit `sha` of terminal `session` on the clipboard and says so. A message or an
    /// author not read yet is read first (one read, the one an opening would make); the copy follows when
    /// it lands, if it is still what the person last asked for and not long ago.
    pub(super) fn copy_commit(
        &mut self,
        session: i64,
        sha: &str,
        what: CopyWhat,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.of.get_mut(&session) else { return };
        let Some(commit) = state.commits.iter().find(|c| c.sha == sha).cloned() else { return };
        let handle = window.window_handle();
        if let Some(text) = what.text(&commit, state.details_of(sha).map(|shown| &shown.details)) {
            put(text, what, handle, cx);
            return;
        }
        debug_assert!(what.needs_details());
        let Some(root) = state.root.clone() else { return };
        state.pending_copy =
            Some(PendingCopy { sha: sha.to_string(), what, window: handle, asked: Instant::now() });
        if let Some(request) = state.want_details(sha) {
            self.fetch_details(session, root, sha.to_string(), request, cx);
        }
    }

    /// Details of `sha` were taken: the copy that waited for them, if it is still the latest and the person
    /// has not gone elsewhere, is made; when git could not read them it says so as a notice does.
    pub(super) fn finish_pending_copy(&mut self, session: i64, sha: &str, cx: &mut Context<Self>) {
        let Some(state) = self.of.get_mut(&session) else { return };
        let Some(pending) = state.pending_copy.take_if(|pending| pending.sha == sha) else { return };
        if pending.asked.elapsed() > COPY_PATIENCE {
            return;
        }
        let text =
            state.commits.iter().find(|c| c.sha == sha).and_then(|commit| {
                pending.what.text(commit, state.details_of(sha).map(|shown| &shown.details))
            });
        match text {
            Some(text) => put(text, pending.what, pending.window, cx),
            None => crate::notice::post_to(pending.window, Failed::CopyCommit, None, cx),
        }
    }

    /// ⌘C on a commit the keys are on: its hash. `false` when the row is not a commit (the key goes on).
    pub(super) fn copy_row(
        &mut self,
        session: i64,
        row: usize,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(state) = self.of.get(&session) else { return false };
        let Some(super::HistoryRow::Commit(ix)) = state.list.items().get(row) else { return false };
        let Some(sha) = state.commits.get(*ix).map(|c| c.sha.clone()) else { return false };
        self.copy_commit(session, &sha, CopyWhat::Hash, window, cx);
        true
    }
}

/// Writes `text` to the clipboard and says what it was in the window that asked.
fn put(text: String, what: CopyWhat, window: AnyWindowHandle, cx: &mut App) {
    crate::platform::clipboard::write(text, cx);
    crate::notice::post_to(window, what.notice(), None, cx);
}

#[cfg(test)]
mod tests {
    use wormhole_core::git::{Commit, CommitDetails, Signature};

    use super::CopyWhat;

    fn commit() -> Commit {
        Commit {
            sha: "a".repeat(40),
            short: "aaaaaaa".into(),
            parent: None,
            merge: false,
            author: "Ada".into(),
            time: 0,
            subject: "fix: it".into(),
        }
    }

    fn details() -> CommitDetails {
        let who = Signature { name: "Ada".into(), email: "ada@example.com".into(), time: 0, offset: 0 };
        CommitDetails {
            sha: "a".repeat(40),
            parents: Vec::new(),
            parents_short: Vec::new(),
            author: who.clone(),
            committer: who,
            refs: Vec::new(),
            message: "fix: it\n\nbecause".into(),
            message_cut: false,
            co_authors: Vec::new(),
        }
    }

    #[test]
    fn the_log_has_what_three_of_them_need() {
        let commit = commit();
        assert_eq!(CopyWhat::Hash.text(&commit, None), Some("a".repeat(40)));
        assert_eq!(CopyWhat::ShortHash.text(&commit, None), Some("aaaaaaa".into()));
        assert_eq!(CopyWhat::Subject.text(&commit, None), Some("fix: it".into()));
        // With the message read, the subject is its first line, which is what the heading shows.
        let mut read = details();
        read.message = "first line\nsecond line of the same paragraph\n\nbody".into();
        assert_eq!(CopyWhat::Subject.text(&commit, Some(&read)), Some("first line".into()));
        assert_eq!(CopyWhat::Message.text(&commit, None), None);
        assert_eq!(CopyWhat::Author.text(&commit, None), None);
    }

    #[test]
    fn the_message_and_the_author_come_from_the_details() {
        let (commit, details) = (commit(), details());
        assert_eq!(CopyWhat::Message.text(&commit, Some(&details)), Some("fix: it\n\nbecause".into()));
        assert_eq!(CopyWhat::Author.text(&commit, Some(&details)), Some("Ada <ada@example.com>".into()));
        let needing: Vec<_> = CopyWhat::ALL.into_iter().filter(|what| what.needs_details()).collect();
        assert_eq!(needing, [CopyWhat::Message, CopyWhat::Author]);
    }
}
