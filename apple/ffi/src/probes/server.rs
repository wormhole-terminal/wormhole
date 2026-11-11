//! What a hook's `status` action set, for the status bar.

use super::state;
use crate::hook_server::HookServer;

/// A status item a Probe's hook set: the Probe's, labelled so (its name is in the tooltip).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProbeStatusRecord {
    pub probe: String,
    pub item: String,
    /// What it says, at most 80 characters (core caps it).
    pub text: String,
    /// The design token the text is drawn with (`text.link`, `status.danger.text`, ...), or `None` for the
    /// status bar's own text colour.
    pub token: Option<String>,
    /// The Probe's name, then whatever tooltip it gave (`StatusItem::shown_tooltip`).
    pub tooltip: String,
}

#[uniffi::export]
impl HookServer {
    /// The status items the Probes' hooks set, oldest first. Read again after `HookEvent::ProbesChanged`.
    pub fn probe_status_items(&self) -> Vec<ProbeStatusRecord> {
        state::status_items()
            .into_iter()
            .map(|item| ProbeStatusRecord {
                tooltip: item.shown_tooltip(),
                probe: item.probe,
                item: item.item,
                text: item.text,
                token: item.tone.map(|tone| tone.token().to_string()),
            })
            .collect()
    }

    /// Read the Probes again (what the Settings window's approvals do on their own): for a test or a script.
    pub fn reload_probes(&self) {
        super::reload(&self.core);
    }
}
