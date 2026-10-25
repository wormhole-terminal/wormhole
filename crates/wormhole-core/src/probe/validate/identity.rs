//! `[probe]`, the caps and the ids: who the Probe says it is, and that what it contributes is
//! countable and namespaced.

use std::collections::BTreeSet;

use semver::{Version, VersionReq};

use super::Findings;
use super::paths;
use crate::probe::manifest::{API, CODE_API, Permissions, is_local_id, is_probe_id};
use crate::probe::read::ProbeFolder;

/// Contributions of each kind a Probe may have.
pub(crate) const MAX_RULES: usize = 64;
pub(crate) const MAX_COMMANDS: usize = 16;
pub(crate) const MAX_STATUS: usize = 8;
pub(crate) const MAX_AGENTS: usize = 4;

pub(super) fn check(folder: &ProbeFolder, out: &mut Findings) {
    for warning in &folder.warnings {
        out.error(
            "unknown-key",
            &warning.path,
            format!("{}; it is a warning when a Probe loads and an error here", warning.message),
        );
    }
    check_probe(folder, out);
    check_code_fields(folder, out);
    check_ids_and_caps(folder, out);
    check_needs_code(folder, out);
}

fn check_probe(folder: &ProbeFolder, out: &mut Findings) {
    let p = &folder.manifest.probe;
    if !is_probe_id(&p.id) {
        out.error(
            "probe-id",
            "probe.id",
            format!("`{}` is not an id: use 2 to 32 characters from a-z, 0-9 and `-`", p.id),
        );
    }
    // These are what the approval sheet and the list show: empty says nothing and a control
    // character could draw over the rest.
    for (field, text) in [("name", &p.name), ("description", &p.description), ("author", &p.author)] {
        if text.trim().is_empty() {
            out.error("probe-text-empty", format!("probe.{field}"), "must not be empty");
        } else if text.chars().any(char::is_control) {
            out.error("probe-text-control", format!("probe.{field}"), "must not hold control characters");
        }
    }
    if let Err(err) = Version::parse(&p.version) {
        out.error(
            "probe-version",
            "probe.version",
            format!("`{}` is not a semver version: {err}", p.version),
        );
    }
    if let Err(err) = VersionReq::parse(&p.wormhole) {
        out.error(
            "probe-wormhole-range",
            "probe.wormhole",
            format!("`{}` is not a version range such as `>=1.3`: {err}", p.wormhole),
        );
    }
    if p.api != API {
        out.error(
            "probe-api",
            "probe.api",
            format!("api is {}, and `{API}` is the only generation there is", p.api),
        );
    }
    if let Some(repository) = &p.repository
        && !repository.starts_with("https://")
    {
        out.warning("probe-repository", "probe.repository", "should be an https link");
    }
}

/// `main` and `code_api` go together, and `main` names a module of the folder.
fn check_code_fields(folder: &ProbeFolder, out: &mut Findings) {
    let p = &folder.manifest.probe;
    match (&p.main, p.code_api) {
        (None, Some(_)) => out.error(
            "code-api-without-main",
            "probe.code_api",
            "code_api belongs to a Probe with code; add `main`, or remove it",
        ),
        (Some(_), None) => out.error(
            "main-without-code-api",
            "probe.main",
            format!("a Probe with code says which code API it is written for: add `code_api = {CODE_API}`"),
        ),
        _ => {}
    }
    if let Some(generation) = p.code_api
        && generation != CODE_API
    {
        out.error(
            "code-api-unsupported",
            "probe.code_api",
            format!("code_api {generation} does not exist; `{CODE_API}` is the only one"),
        );
    }
    if let Some(main) = &p.main {
        if !(main.ends_with(".ts") || main.ends_with(".js")) {
            out.error("main-extension", "probe.main", format!("`{main}` must be a `.ts` or `.js` module"));
        }
        paths::file_of(folder, main, "probe.main", ("main-path", "main-missing"), out);
    }
}

fn check_ids_and_caps(folder: &ProbeFolder, out: &mut Findings) {
    let c = &folder.manifest.contributes;
    for (kind, count, max, code) in [
        ("rules", c.rules.len(), MAX_RULES, "too-many-rules"),
        ("commands", c.commands.len(), MAX_COMMANDS, "too-many-commands"),
        ("status", c.status.len(), MAX_STATUS, "too-many-status"),
        ("agents", c.agents.len(), MAX_AGENTS, "too-many-agents"),
    ] {
        if count > max {
            out.error(code, format!("contributes.{kind}"), format!("{count} {kind}; a Probe may have {max}"));
        }
    }
    // A contribution's global id is `<probe id>/<local id>`; within a kind that must name one thing.
    let kinds: [(&str, Vec<&str>); 8] = [
        ("rules", c.rules.iter().map(|x| x.id.as_str()).collect()),
        ("commands", c.commands.iter().map(|x| x.id.as_str()).collect()),
        ("status", c.status.iter().map(|x| x.id.as_str()).collect()),
        ("agents", c.agents.iter().map(|x| x.id.as_str()).collect()),
        ("themes", c.themes.iter().map(|x| x.id.as_str()).collect()),
        ("icon_themes", c.icon_themes.iter().map(|x| x.id.as_str()).collect()),
        ("panels", c.panels.iter().map(|x| x.id.as_str()).collect()),
        ("hovers", c.hovers.iter().map(|x| x.id.as_str()).collect()),
    ];
    for (kind, ids) in kinds {
        let mut seen = BTreeSet::new();
        for (ix, id) in ids.into_iter().enumerate() {
            let path = format!("contributes.{kind}[{ix}].id");
            if !is_local_id(id) {
                out.error(
                    "contribution-id",
                    path,
                    format!("`{id}` is not an id: use 1 to 32 characters from a-z, 0-9 and `-`"),
                );
            } else if !seen.insert(id) {
                out.error(
                    "contribution-id-duplicate",
                    path,
                    format!("`{}/{id}` is already a {kind} id of this Probe", folder.manifest.probe.id),
                );
            }
        }
    }
}

/// The `[permissions]` keys that only code (E2) can use, for the ones this Probe sets.
fn code_keys(p: &Permissions) -> impl Iterator<Item = &'static str> {
    [("terminal", !p.terminal.is_empty()), ("net", !p.net.is_empty()), ("fs", !p.fs.is_empty())]
        .into_iter()
        .filter_map(|(key, used)| used.then_some(key))
}

/// What only code can provide, for a Probe without any.
fn check_needs_code(folder: &ProbeFolder, out: &mut Findings) {
    let m = &folder.manifest;
    if m.probe.main.is_some() {
        return;
    }
    for key in code_keys(&m.permissions) {
        out.error(
            "needs-code",
            format!("permissions.{key}"),
            format!("`{key}` needs code (E2), which is not supported yet"),
        );
    }
    for (kind, count) in [("panels", m.contributes.panels.len()), ("hovers", m.contributes.hovers.len())] {
        if count > 0 {
            out.error(
                "needs-code",
                format!("contributes.{kind}"),
                format!("{kind} are drawn by code (E2), which is not supported yet"),
            );
        }
    }
}
