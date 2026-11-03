//! Extensions: the Probes in the Probes folder with their states, and the approval sheet's data. The reads
//! are the GPUI page's (`probe::approval::list` and `read_one`, in the background), and an approval goes the
//! way its sheet's does: the grants the sheet opened with, a click or Touch ID by what they need, then
//! `approval::approve` of exactly the bytes that were read for the sheet.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::App;
use serde_json::{Map, Value, json};
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::probe::approval::{self, Entry as Listed, KeyProblem, ProbeState, Request};
use wormhole_core::probe::hidden::reveal;
use wormhole_core::probe::scan::MAX_ENTRIES;
use wormhole_core::probe::validate::Confirmation;
use wormhole_core::storage::paths::DataRoot;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::Entry;
use crate::platform::auth::{self, AuthError};
use crate::settings::settings;
use crate::settings_window::sections::{describe_permission, verify_text};

const GROUP: &str = "Extensions:0";

/// What the Probes folder held when it was last read, with a request for each Probe that can be approved.
pub(crate) struct Probes {
    listing: Result<(Vec<Listed>, Option<KeyProblem>), ()>,
    requests: HashMap<String, Request>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn read(root: &DataRoot) -> Probes {
    match approval::list(root) {
        Ok(listing) => {
            if let Err(err) = approval::sweep(root, listing.present().as_ref()) {
                tracing::warn!("tidying the approved Probes: {err}");
            }
            let requests = listing
                .entries
                .iter()
                .filter(|entry| entry.state.can_approve())
                .filter_map(|entry| {
                    let id = entry.probe.as_ref().map_or(entry.folder.as_str(), |probe| probe.id.as_str());
                    approval::read_one(root, id).ok().map(|request| (id.to_string(), request))
                })
                .collect();
            Probes { listing: Ok((listing.entries, listing.key_problem)), requests }
        }
        Err(_) => Probes { listing: Err(()), requests: HashMap::new() },
    }
}

fn id_of(entry: &Listed) -> &str {
    entry.probe.as_ref().map_or(entry.folder.as_str(), |probe| probe.id.as_str())
}

fn enabled(cx: &App) -> bool {
    settings(cx).feature_enabled(FeatureFlag::Probes)
}

pub(super) fn lists(out: &mut Map<String, Value>, sheets: &mut Map<String, Value>, cx: &mut App) {
    let mut list = Vec::new();
    if enabled(cx) {
        match super::cache(cx).and_then(|cache| cache.probes.as_ref()) {
            None => list.push(Entry::new("looking", "value", text(SETTINGS_EXTENSIONS_LOOKING)).into_value()),
            Some(Probes { listing: Err(()), .. }) => list.push(
                Entry::new("too-many", "value", &settings_extensions_too_many(&MAX_ENTRIES.to_string()))
                    .into_value(),
            ),
            Some(Probes { listing: Ok((entries, key_problem)), requests }) => {
                if let Some(problem) = key_problem {
                    let words = match problem {
                        KeyProblem::Unreadable => text(SETTINGS_EXTENSIONS_KEY_UNAVAILABLE),
                        KeyProblem::InRealDataFolder => text(SETTINGS_EXTENSIONS_DEV_DATA_FOLDER),
                    };
                    list.push(Entry::new("key-problem", "value", words).into_value());
                }
                if entries.is_empty() {
                    list.push(Entry::new("none", "value", text(SETTINGS_EXTENSIONS_NONE_TITLE)).into_value());
                }
                for entry in entries {
                    list.push(probe_entry(entry, requests.get(id_of(entry))));
                    if let Some(request) = requests.get(id_of(entry)) {
                        sheets.insert(id_of(entry).to_string(), sheet(request));
                    }
                }
            }
        }
    }
    out.insert(GROUP.into(), Value::Array(list));
}

fn probe_entry(entry: &Listed, request: Option<&Request>) -> Value {
    let (title, identity, description) = match &entry.probe {
        Some(probe) => (
            reveal(&probe.name).into_owned(),
            settings_extensions_identity(&reveal(&probe.id), &reveal(&probe.version), &reveal(&probe.author)),
            reveal(&probe.description).into_owned(),
        ),
        None => (
            reveal(&entry.folder).into_owned(),
            text(SETTINGS_EXTENSIONS_FOLDER_HINT).to_string(),
            String::new(),
        ),
    };
    let (value, note) = match &entry.state {
        ProbeState::On => ("on", None),
        ProbeState::Off if entry.unsealed => {
            ("off", Some(text(SETTINGS_EXTENSIONS_UNSEALED_NOTE).to_string()))
        }
        ProbeState::Off => ("off", None),
        ProbeState::Changed => (
            "changed",
            Some(match &entry.copy_problem {
                Some(problem) => verify_text(problem),
                None => text(SETTINGS_EXTENSIONS_CHANGED_NOTE).to_string(),
            }),
        ),
        ProbeState::Unsupported => {
            ("unsupported", Some(text(SETTINGS_EXTENSIONS_UNSUPPORTED_NOTE).to_string()))
        }
        ProbeState::Invalid(reason) => ("invalid", Some(settings_extensions_invalid_reason(&reveal(reason)))),
    };
    let mut out = Entry::new(id_of(entry), "probe", &title).with("hint", identity).with("value", value);
    if !description.is_empty() {
        out = out.with("info", description);
    }
    if let Some(note) = note {
        out = out.with("note", note);
    }
    if request.is_some() {
        out = out.with("sheet", id_of(entry));
    }
    out.into_value()
}

/// What the approval sheet shows of a request: who, what changed since the last approval, what it asks for.
fn sheet(request: &Request) -> Value {
    let probe = &request.folder().manifest.probe;
    let name = reveal(&probe.name).into_owned();
    let changes = request.changes();
    let title = if changes.is_some() {
        settings_extensions_sheet_title_again(&name)
    } else {
        settings_extensions_sheet_title(&name)
    };
    let mut since = Vec::new();
    if let Some(changes) = &changes {
        if changes.files_only() {
            since.push(text(SETTINGS_EXTENSIONS_FILES_CHANGED).to_string());
        }
        since.extend(
            changes.added.iter().map(|a| settings_extensions_now_asks(&reveal(&describe_permission(a)))),
        );
        since.extend(changes.removed.iter().map(|gone| settings_extensions_no_longer_asks(&reveal(gone))));
    }
    let summary = request.summary();
    let grants = request.initial_grants();
    // [icon, words, detail, the permission's place (what the tick sends back), whether it starts ticked]
    let mut perms: Vec<Value> = summary
        .permissions
        .iter()
        .enumerate()
        .map(|(at, p)| {
            let icon = match p.key {
                "process" => "run",
                "notify" => "notify",
                "open" => "link",
                _ => "other",
            };
            json!([icon, describe_permission(p), "", at.to_string(), grants.is_granted(p)])
        })
        .collect();
    perms.extend(
        summary
            .combinations
            .iter()
            .map(|c| json!(["run", settings_extensions_combination(&reveal(&c.description)), "", "", false])),
    );
    json!({
        "title": title,
        "identity": settings_extensions_identity(&reveal(&probe.id), &reveal(&probe.version), &reveal(&probe.author)),
        "since": since,
        "perms": perms,
    })
}

pub(super) fn load(cx: &mut App) {
    if !enabled(cx) {
        return;
    }
    let root = DataRoot::current();
    cx.spawn(async move |cx| {
        let read = cx.background_executor().spawn(async move { read(&root) }).await;
        cx.update(|cx| {
            if cx.has_global::<crate::settings_window::native::Native>() {
                super::cache_mut(cx).probes = Some(read);
                crate::settings_window::native::push(cx);
            }
        })
    })
    .detach();
}

/// The folder in the Finder (the footer's button).
pub(super) fn show_folder(cx: &mut App) {
    crate::settings_window::open_folder(&DataRoot::current().probes_dir(), cx);
}

pub(super) fn perform(key: &str, action: &str, cx: &mut App) -> bool {
    let mut parts = key.splitn(3, ':');
    let (Some("Extensions"), Some(_), Some(id)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    match action {
        "turn_off" => turn_off(id.to_string(), cx),
        // "approve=<the places of the permissions left ticked>"
        _ => {
            if let Some(ticked) = action.strip_prefix("approve=") {
                approve(id, ticked, cx);
            }
        }
    }
    true
}

fn turn_off(id: String, cx: &mut App) {
    let root = DataRoot::current();
    cx.spawn(async move |cx| {
        let result = cx.background_executor().spawn(async move { approval::turn_off(&root, &id) }).await;
        if let Err(err) = result {
            tracing::warn!("turning a Probe off: {err}");
        }
        cx.update(load)
    })
    .detach();
}

/// Approve exactly what the sheet showed: the grants it opened with, confirmed by what they need.
fn approve(id: &str, ticked: &str, cx: &mut App) {
    let Some(request) =
        super::cache(cx).and_then(|cache| cache.probes.as_ref()).and_then(|p| p.requests.get(id).cloned())
    else {
        return;
    };
    let places: Vec<usize> = ticked.split(',').filter_map(|at| at.parse().ok()).collect();
    let mut grants = request.initial_grants();
    for (at, permission) in request.summary().permissions.iter().enumerate() {
        grants.set(permission, places.contains(&at));
    }
    match request.confirmation(&grants) {
        Confirmation::Click => finish(request, grants, approval::Proof::Click, Ok(()), cx),
        Confirmation::TouchId => {
            let name = reveal(&request.folder().manifest.probe.name).into_owned();
            let rx = auth::authenticate_rx(&settings_extensions_auth_reason(&name));
            cx.spawn(async move |cx| {
                let answer = rx.recv().await.unwrap_or(Err(AuthError::Cancelled));
                cx.update(|cx| finish(request, grants, approval::Proof::TouchId, answer, cx))
            })
            .detach();
        }
    }
}

fn finish(
    request: Request,
    grants: approval::Grants,
    proof: approval::Proof,
    answer: Result<(), AuthError>,
    cx: &mut App,
) {
    if let Err(err) = answer {
        if !matches!(err, AuthError::Cancelled) {
            tracing::warn!("approving a Probe: {err}");
        }
        return;
    }
    let confirmed = match request.confirm(&grants, proof) {
        Ok(confirmed) => confirmed,
        Err(err) => {
            tracing::warn!("approving a Probe: {err}");
            return;
        }
    };
    let root = DataRoot::current();
    cx.spawn(async move |cx| {
        let result = cx
            .background_executor()
            .spawn(async move { approval::approve(&root, &request, &confirmed, now()) })
            .await;
        if let Err(err) = result {
            tracing::warn!("approving a Probe: {err}");
        }
        cx.update(load)
    })
    .detach();
}
