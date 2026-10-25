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
/// reason after `invalid`, and one for an approval sheet that is open. Nothing while the flag is off.
pub(super) fn state(cx: &App) -> String {
    if !settings(cx).feature_enabled(FeatureFlag::Probes) {
        return String::new();
    }
    let mut out = String::new();
    match approval::list(&DataRoot::current()) {
        Ok(entries) => {
            for entry in entries {
                let word = match &entry.state {
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
