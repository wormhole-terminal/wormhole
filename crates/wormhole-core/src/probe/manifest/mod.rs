//! `probe.toml`: the file that makes a folder a Probe (`repo-docs/docs/probes.md`, section 2).
//!
//! Parsing says only whether the text is a manifest of the right shape: the right keys, the right
//! types, the closed sets of values. What a manifest may *mean* (caps, patterns, programs, paths) is
//! `validate`'s. A key the format does not know is kept and reported as a [`ParseWarning`], so a
//! newer Probe on an older app still loads and `validate` can refuse the typo.
//!
//! Nothing here reads a file or knows a folder; it must not use `storage`, `project` or `config`.

mod contrib;
mod ids;

use std::fmt;

use semver::{Version, VersionReq};
use serde::Deserialize;

pub use contrib::{
    Action, ActionKind, AgentDef, AgentStatus, CodeSlot, Command, Contributes, Cwd, FileContribution,
    KeepSpec, Look, Permissions, Reveal, Rule, RunIn, Side, StatusItem, TerminalAccess,
};
pub use ids::{is_local_id, is_probe_id};

use contrib::Unknown;

/// The only manifest generation there is.
pub const API: u32 = 1;
/// The only code API generation there is: early, and free to move.
pub const CODE_API: u32 = 0;

/// `[probe]`: who and what the Probe is.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProbeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    /// Semver.
    pub version: String,
    pub repository: Option<String>,
    /// The app versions it runs on, as a semver range (`">=1.3"`).
    pub wormhole: String,
    pub api: u32,
    /// E2 only, and required with `main`.
    pub code_api: Option<u32>,
    /// E2's entry module; absent means the Probe is files only (E1).
    pub main: Option<String>,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}

impl ProbeInfo {
    /// The Probe's version, when it is one.
    pub fn parsed_version(&self) -> Result<Version, semver::Error> {
        Version::parse(&self.version)
    }

    /// Whether the app at `app` is inside the Probe's range. A range that does not parse supports
    /// nothing; `validate` says why.
    pub fn supports(&self, app: &Version) -> bool {
        VersionReq::parse(&self.wormhole).is_ok_and(|req| req.matches(app))
    }
}

/// A parsed `probe.toml`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Manifest {
    pub probe: ProbeInfo,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default)]
    pub contributes: Contributes,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}

/// A key the format does not know, found while parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseWarning {
    /// Where, as `contributes.rules[2].colour`.
    pub path: String,
    pub message: String,
}

/// A manifest and the warnings its parse gave.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub manifest: Manifest,
    pub warnings: Vec<ParseWarning>,
}

/// Why a text is not a manifest, with its place when TOML knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestError {
    pub message: String,
    /// 1-based.
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(column)) => write!(f, "{line}:{column}: {}", self.message),
            _ => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for ManifestError {}

impl Manifest {
    /// Read `text` as a manifest.
    pub fn parse(text: &str) -> Result<Parsed, ManifestError> {
        let manifest: Manifest = toml::from_str(text).map_err(|err| {
            let (line, column) = err.span().map_or((None, None), |span| {
                let (line, column) = position(text, span.start);
                (Some(line), Some(column))
            });
            ManifestError { message: err.message().trim().to_string(), line, column }
        })?;
        let warnings = manifest.unknown_keys();
        Ok(Parsed { manifest, warnings })
    }

    /// Every key the format does not know, in document order within each table.
    fn unknown_keys(&self) -> Vec<ParseWarning> {
        let mut out = Vec::new();
        let mut add = |path: String, unknown: &Unknown| {
            out.extend(unknown.keys().map(|key| ParseWarning {
                path: if path.is_empty() { key.clone() } else { format!("{path}.{key}") },
                message: format!("unknown key `{key}`"),
            }));
        };
        add(String::new(), &self.unknown);
        add("probe".into(), &self.probe.unknown);
        add("permissions".into(), &self.permissions.unknown);
        let c = &self.contributes;
        add("contributes".into(), &c.unknown);
        for (ix, rule) in c.rules.iter().enumerate() {
            add(format!("contributes.rules[{ix}]"), &rule.unknown);
            if let Some(action) = &rule.action {
                add(format!("contributes.rules[{ix}].action"), &action.unknown);
            }
        }
        for (ix, item) in c.commands.iter().enumerate() {
            add(format!("contributes.commands[{ix}]"), &item.unknown);
        }
        for (ix, item) in c.status.iter().enumerate() {
            add(format!("contributes.status[{ix}]"), &item.unknown);
        }
        for (ix, item) in c.agents.iter().enumerate() {
            add(format!("contributes.agents[{ix}]"), &item.unknown);
        }
        for (ix, item) in c.themes.iter().enumerate() {
            add(format!("contributes.themes[{ix}]"), &item.unknown);
        }
        for (ix, item) in c.icon_themes.iter().enumerate() {
            add(format!("contributes.icon_themes[{ix}]"), &item.unknown);
        }
        for (ix, item) in c.panels.iter().enumerate() {
            add(format!("contributes.panels[{ix}]"), &item.unknown);
        }
        for (ix, item) in c.hovers.iter().enumerate() {
            add(format!("contributes.hovers[{ix}]"), &item.unknown);
        }
        out
    }
}

/// The 1-based line and column of byte `offset` in `text`.
fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset.min(text.len())];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().map_or(0, |last| last.chars().count()) + 1;
    (line, column)
}

#[cfg(test)]
mod tests;
