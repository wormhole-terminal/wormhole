//! "Move this window to <profile>": a window and its running terminals go to another profile, which keeps their record
//! from then on. As the GPUI app's `Workspace::rehome`: the record, the notes and tasks, the terminals' text (when the
//! profile it goes to keeps any), the files each terminal changed and the terminals' own flags (pinned, hidden from
//! agents, no text kept) are written into the target's database and deleted from the source's; the window is among the
//! target's windows from then on, which is what its requests (agent access, trust) are judged by.

use std::sync::Arc;

use wormhole_core::config::Config;
use wormhole_core::storage::store::WindowRecord;

use super::frame::FrameRecord;
use super::registry::Windows;
use super::reissue::{reissued, renamed_claims};
use crate::workspace::{Shared, Workspace};

/// The lines of history kept per terminal (`SCROLLBACK_LINES` in the GPUI app's `window/save.rs`).
const SAVED_LINES: usize = 2000;

impl Windows {
    /// The saved windows of this database that have the id of `record` or of one of its terminals or splits.
    fn colliding(&self, record: &WindowRecord) -> Vec<WindowRecord> {
        let saved = self.store_locked().load().unwrap_or_else(|err| {
            tracing::warn!("reading the saved windows: {err:#}");
            Vec::new()
        });
        let ids: Vec<i64> =
            record.sessions.iter().map(|s| s.id).chain(record.splits.iter().map(|s| s.id)).collect();
        saved
            .into_iter()
            .filter(|w| {
                w.id == record.id
                    || w.sessions
                        .iter()
                        .map(|s| s.id)
                        .chain(w.splits.iter().map(|s| s.id))
                        .any(|id| ids.contains(&id))
            })
            .collect()
    }

    /// Whether a window other than `mover` is open under the id of `saved` or runs one of its terminals: it is not
    /// dormant, and its rows are its own to write.
    fn open_apart_from(&self, saved: &WindowRecord, mover: &Arc<Shared>) -> bool {
        let live: Vec<(i64, Arc<Shared>)> = super::registry::locked(&self.live)
            .iter()
            .filter_map(|(id, shared)| Some((*id, shared.upgrade()?)))
            .filter(|(_, shared)| !Arc::ptr_eq(shared, mover))
            .collect();
        live.iter().any(|(id, shared)| {
            *id == saved.id
                || shared.peek_model(|model| {
                    let held = model.ids();
                    saved.sessions.iter().any(|s| held.contains(&s.id))
                })
        })
    }

    /// Give a saved window of this database new ids (see `reissue`), with its notes, tasks, text and claims.
    fn renumber(&self, old: WindowRecord) -> bool {
        let old_ids: Vec<i64> = old.sessions.iter().map(|s| s.id).collect();
        let claims = self.claims_of(old.id, &old_ids);
        let new_window = self.next_window.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (record, renamed) = reissued(&old, new_window, || self.ids.next());
        let texts: Vec<(i64, String)> = old
            .sessions
            .iter()
            .filter(|s| !s.forget_text)
            .filter_map(|s| Some((renamed[&s.id], s.scrollback.clone()?)))
            .collect();
        let new_ids: Vec<i64> = record.sessions.iter().map(|s| s.id).collect();
        let mut store = self.store_locked();
        // The new rows first, then the old: a failure in between leaves a copy, never a loss.
        if let Err(err) = store.save_window(&record) {
            tracing::warn!("renumbering window {}: {err:#}", old.id);
            return false;
        }
        if let Err(err) = store.save_scrollbacks(&texts) {
            tracing::warn!("moving the text of renumbered window {}: {err:#}", old.id);
        }
        if store.front_window().ok().flatten() == Some(old.id)
            && let Err(err) = store.save_front_window(new_window)
        {
            tracing::warn!("keeping the front window: {err:#}");
        }
        if let Err(err) = store.delete_window(old.id) {
            tracing::warn!("removing renumbered window {}: {err:#}", old.id);
        }
        drop(store);
        self.forget_written_claims(old.id);
        self.put_claims(new_window, &new_ids, renamed_claims(claims, &renamed));
        true
    }

    /// The screen text of the terminals of `record` that this profile may carry to `to`: the live terminal's, else what
    /// was saved last; none when `to` keeps no text, or the terminal's row says not to.
    fn text_to_carry(
        &self,
        record: &WindowRecord,
        workspace: &Workspace,
        to: &Windows,
    ) -> Vec<(i64, String)> {
        if !Config::load_for(to.profile()).settings.restore_terminal_text {
            return Vec::new();
        }
        let saved = self.record_of(record.id);
        record
            .sessions
            .iter()
            .filter(|s| !s.forget_text)
            .filter_map(|s| {
                let live = workspace
                    .shared()
                    .terminal_of(s.id)
                    .and_then(|t| t.with_pty(|pty| pty.scrollback().text(SAVED_LINES)).flatten());
                let stored = || saved.as_ref()?.sessions.iter().find(|x| x.id == s.id)?.scrollback.clone();
                Some((s.id, live.or_else(stored)?))
            })
            .collect()
    }
}

#[uniffi::export]
impl Windows {
    /// Window `window` goes to the profile `to` keeps ("Move this window to..."): its record is written into that
    /// profile's database and removed from this one. Unlike `forget` it leaves the window among the live ones, since
    /// the terminals go on running where they are. A window saved in `to` under the same ids (from before the profiles
    /// shared one counter) is renumbered there first, unless it is open; then the move is refused and nothing changes.
    /// False when the write failed (nothing is removed then).
    pub fn rehome(
        &self,
        window: i64,
        workspace: Arc<Workspace>,
        frame: Option<FrameRecord>,
        to: Arc<Windows>,
    ) -> bool {
        if std::ptr::eq(self, &*to) {
            return true;
        }
        // The notes and tasks are read where the window is now, before they are written into the other database.
        let record = {
            let store = self.store_locked();
            self.record_of_window(&store, window, &workspace, frame)
        };
        let ids: Vec<i64> = record.sessions.iter().map(|session| session.id).collect();
        let texts = self.text_to_carry(&record, &workspace, &to);
        let claims = self.claims_of(window, &ids);
        let colliding = to.colliding(&record);
        if colliding.iter().any(|saved| to.open_apart_from(saved, workspace.shared())) {
            tracing::warn!(
                "window {window} stays in its profile: an open window of the other one has its ids"
            );
            return false;
        }
        for saved in colliding {
            if !to.renumber(saved) {
                return false;
            }
        }
        if !to.write(to.store_locked(), window, record) {
            return false;
        }
        if let Err(err) = to.store_locked().save_scrollbacks(&texts) {
            tracing::warn!("moving the terminals' text of window {window}: {err:#}");
        }
        to.put_claims(window, &ids, claims);
        self.forget_written_claims(window);
        super::registry::locked(&self.planned).remove(&window);
        if let Err(err) = self.store_locked().delete_window(window) {
            tracing::warn!("removing window {window} from its old profile: {err:#}");
        }
        workspace.shared().set_windows(to, window);
        true
    }
}
