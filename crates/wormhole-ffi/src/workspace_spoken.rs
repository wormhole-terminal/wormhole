//! What VoiceOver says for the status bar's items, in the core's sentences (`view::spoken`): "Fix pile needs input,
//! waiting 2 minutes", "2 terminals working". A panel tab's words are `panel_spoken`'s.

use std::time::{Duration, Instant};

use wormhole_core::view::layout::status_bar::{Activity, Counted, Entry, Summary};
use wormhole_core::view::spoken::{status_attention, status_counted};

use crate::workspace::Workspace;

#[uniffi::export]
impl Workspace {
    /// The status bar's needs-you button: "<name> needs input, waiting 2 minutes". `since_ms` is the terminal summary's `since`.
    pub fn spoken_status_attention(&self, name: String, since_ms: u64) -> String {
        let now = Instant::now();
        let waited = Duration::from_millis(self.now_ms().saturating_sub(since_ms));
        let entries = [Entry {
            activity: Activity::NeedsYou,
            since: now.checked_sub(waited).unwrap_or(now),
            agent: None,
            name: &name,
        }];
        status_attention(&Summary::of(&entries), &entries, now).unwrap_or(name)
    }

    /// One of the numbers after the button: "2 terminals working", "1 terminal finished". `activity` is `working`, `needs`,
    /// `done`, `failed` or anything else for the quiet ones.
    pub fn spoken_status_count(&self, activity: String, count: u32) -> String {
        let activity = match activity.as_str() {
            "working" => Activity::Working,
            "needs" => Activity::NeedsYou,
            "done" => Activity::Done,
            "failed" => Activity::Failed,
            _ => Activity::Quiet,
        };
        status_counted(&Counted { activity, count: count as usize, terminal: 0 })
    }
}
