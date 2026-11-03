//! Probes: the `state` lines that say where each one stands.

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::probe::approval::{self, KeyProblem, ProbeState};
use wormhole_core::probe::hidden::reveal;
use wormhole_core::storage::paths::DataRoot;

use crate::settings::settings;

/// One line per folder of the Probes folder, `probe <folder> <on|off|changed|invalid|unsupported>` with the
/// reason after `invalid` (`off unsealed` for a stored approval this Wormhole did not seal),
/// `probes-key unavailable` first when the approvals' key could not be read,. Nothing while the flag is off. Reading the key never reaches the Keychain here: a devctl build
/// keeps it in a dev-only file of the instance's scratch data dir.
///
/// This reads the Probes folder on the main thread, which the page never does: it is a test tool, run by a script
/// that waits for the answer, over the handful of folders of a scratch data dir, and `state` already reads the
/// whole window in the same call. The page's own read is on the background executor.
pub(super) fn state(cx: &App) -> String {
    if !settings(cx).feature_enabled(FeatureFlag::Probes) {
        return String::new();
    }
    let mut out = String::new();
    match approval::list(&DataRoot::current()) {
        Ok(entries) => {
            match entries.key_problem {
                Some(KeyProblem::Unreadable) => out.push_str("probes-key unavailable\n"),
                Some(KeyProblem::InRealDataFolder) => out.push_str("probes-key real-data-folder\n"),
                None => {}
            }
            for entry in entries.entries {
                let word = match &entry.state {
                    ProbeState::Off if entry.unsealed => "off unsealed".to_string(),
                    ProbeState::Off => "off".to_string(),
                    ProbeState::On => "on".to_string(),
                    ProbeState::Changed => "changed".to_string(),
                    ProbeState::Invalid(reason) => format!("invalid {}", reveal(reason)),
                    ProbeState::Unsupported => "unsupported".to_string(),
                };
                out.push_str(&format!("probe {} {word}\n", reveal(&entry.folder)));
            }
        }
        Err(err) => out.push_str(&format!("probes {err}\n")),
    }
    out
}
