//! What the History tab puts on the clipboard for a commit: its hash, short hash, message, subject or
//! author. The hash, short hash and subject are in the log already; the message and the author's address
//! need the commit's details, which are read first if they are not here yet (a copy from the menu of a
//! commit that was never opened).

use gpui_kit::*;
use wormhole_core::git::{self, Commit, CommitDetails};
use wormhole_core::view::notice::Copied;

use super::History;

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
            Self::Subject => commit.subject.clone(),
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
    /// author not read yet is read off the UI thread first; the copy follows when it lands.
    pub(super) fn copy_commit(
        &mut self,
        session: i64,
        sha: &str,
        what: CopyWhat,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.of.get(&session) else { return };
        let Some(commit) = state.commits.iter().find(|c| c.sha == sha).cloned() else { return };
        let handle = window.window_handle();
        let kept = state.details_of(sha).cloned();
        if let Some(text) = what.text(&commit, kept.as_deref()) {
            put(text, what, handle, cx);
            return;
        }
        debug_assert!(what.needs_details());
        let Some(root) = state.root.clone() else { return };
        let task = cx.background_executor().spawn({
            let sha = sha.to_string();
            async move { git::commit_details(&root, &sha).map_err(|e| format!("{e:#}")) }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                let Some(state) = this.of.get_mut(&session) else { return };
                match result {
                    Ok(details) => {
                        let text = what.text(&commit, Some(&details));
                        // Kept like the details of an opened commit, so the next copy costs no process.
                        state.details_loaded(&commit.sha, Ok(details));
                        cx.notify();
                        if let Some(text) = text {
                            put(text, what, handle, cx);
                        }
                    }
                    Err(err) => {
                        tracing::debug!("could not read {}'s details to copy them: {err}", commit.short)
                    }
                }
            });
        })
        .detach();
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
    cx.write_to_clipboard(ClipboardItem::new_string(text));
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
