//! Probes: approving and turning off from a script, and the `state` lines that say where each one stands.
//!
//! A script goes through the same path as a person: `probe-approve` opens the approval sheet for the Probe
//! with every permission on and presses Approve, so a Touch ID level approval asks the system, which
//! `WORMHOLE_AUTH` answers in an instance that sets it (never a real prompt).

use anyhow::{Result, anyhow};
use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::probe::approval::{self, ProbeState};
use wormhole_core::probe::hidden::reveal;
use wormhole_core::probe::manifest::is_probe_id;
use wormhole_core::storage::paths::DataRoot;

use super::Ctx;
use crate::settings::settings;

/// The Probe id of `arg`, when Probes are switched on in this instance.
fn probe_id(arg: &str, cx: &App) -> Result<String> {
    if !settings(cx).feature_enabled(FeatureFlag::Probes) {
        return Err(anyhow!("the probes feature flag is off (`set feature_flags {{\"probes\": true}}`)"));
    }
    let id = arg.trim();
    if is_probe_id(id) { Ok(id.to_string()) } else { Err(anyhow!("expected a Probe id, not {id:?}")) }
}

/// `probe-approve <id>`: the approval sheet for that Probe, approved with all its permissions on.
pub(super) fn approve(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let id = probe_id(arg, ctx.cx)?;
    crate::settings_window::devctl::probe_approve(id, ctx.cx);
    Ok(String::new())
}

/// `probe-off <id>`: turn that Probe off (its approval and its copy go).
pub(super) fn off(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let id = probe_id(arg, ctx.cx)?;
    crate::settings_window::devctl::probe_off(id, ctx.cx);
    Ok(String::new())
}

/// One line per folder of the Probes folder, `probe <folder> <on|off|changed|invalid|unsupported>` with the
/// reason after `invalid` (`off unsealed` for a stored approval this Wormhole did not seal),
/// `probes-key unavailable` first when the approvals' key could not be read, and one for an approval sheet
/// that is open. Nothing while the flag is off. Reading the key never reaches the Keychain here: a devctl build
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
            if entries.key_unavailable {
                out.push_str("probes-key unavailable\n");
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
    out.push_str(&crate::settings_window::devctl::probe_sheet_line(cx));
    out
}
