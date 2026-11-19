//! A right-panel tab as VoiceOver names it, in the core's words (`view::spoken::panel_tab`): "Changes, 3 changed
//! files", "Notes, 2 open next steps, 1 note". The strip says them, as the GPUI app's tabs do.

use wormhole_core::view::spoken::{self, PanelTab};

/// The words of tab `tab` (its name in lower case: `changes`, `notes`, …). `files` is the count of changed files, where
/// the tab has one and it is known; `open_steps` and `notes` are the Notes tab's.
#[uniffi::export]
pub fn panel_tab_spoken(tab: String, files: Option<u32>, open_steps: u32, notes: u32) -> String {
    let tab = match tab.as_str() {
        "changes" => PanelTab::Changes { files: files.map(|n| n as usize) },
        "history" => PanelTab::History,
        "notes" => PanelTab::Notes { open_steps: open_steps as usize, notes: notes as usize },
        "usage" => PanelTab::Usage,
        "project" => PanelTab::Project,
        "files" => PanelTab::Files,
        other => return other.to_string(),
    };
    spoken::panel_tab(tab)
}
