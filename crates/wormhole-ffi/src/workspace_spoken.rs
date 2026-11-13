//! What VoiceOver says for the right panel's tabs and the status bar's items, in the core's sentences
//! (`view::spoken`): "Changes, 3 changed files", "Fix pile needs input, waiting 2 minutes", "2 terminals working".

use std::time::{Duration, Instant};

use wormhole_core::view::layout::status_bar::{Activity, Counted, Entry, Summary};
use wormhole_core::view::spoken::{PanelTab, panel_tab, status_attention, status_counted};

use crate::workspace::Workspace;

#[uniffi::export]
impl Workspace {
    /// A right-panel tab as VoiceOver names it. `tab` is the tab's raw name (`notes`, `changes`, `history`, `usage`,
    /// `project`, `files`); `open_steps` and `notes` are the Notes tab's counts, `files` the changed files when known.
    pub fn spoken_panel_tab(&self, tab: String, open_steps: u32, notes: u32, files: Option<u32>) -> String {
        panel_tab(match tab.as_str() {
            "notes" => PanelTab::Notes { open_steps: open_steps as usize, notes: notes as usize },
            "changes" => PanelTab::Changes { files: files.map(|n| n as usize) },
            "history" => PanelTab::History,
            "usage" => PanelTab::Usage,
            "project" => PanelTab::Project,
            _ => PanelTab::Files,
        })
    }

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
