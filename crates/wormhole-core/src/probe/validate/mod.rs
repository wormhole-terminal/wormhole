//! `validate`: what a Probe's folder asks for and whether it asks for it correctly
//! (`repo-docs/docs/probes.md`, sections 2, 3 and 5).
//!
//! Every check is a [`Finding`]: an error or a warning with a stable `code`, the manifest `path` it
//! is about (`contributes.rules[2].pattern`) and a sentence for a person or an agent reading the
//! `wh probe validate` output or a log. These sentences are command-line and log text, so they stay
//! English and are not part of the interface's catalogs. The [`Summary`] beside them lists what the
//! Probe would be allowed to do, so someone can review that before enabling it.
//!
//! Validation reads the folder as [`ProbeFolder`] gave it and does nothing else: it runs no
//! command, opens no URL and approves nothing. Checks that need parts of the app that do not exist
//! yet (the decoration pipeline, the keymap layer for Probe keys, running) are not stubbed here.
//!
//! Must not use anything above `keys` and `storage`; it reads settings of no kind.

mod agents;
mod argv;
mod commands;
mod identity;
mod paths;
mod pattern;
mod permissions;
mod program;
mod rules;
mod svg;
mod template;

use std::fmt;

use super::read::ProbeFolder;

// What `rules` compiles with must be what `validate` checked with, so these are shared, not copied.
pub(crate) use argv::capture_index;
pub(crate) use identity::MAX_RULES;
pub(crate) use pattern::SIZE_LIMIT;
pub use permissions::{Combination, Confirmation, DeclaredPermission, ProgramClass, ProgramNote, Summary};

/// How much a finding matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The Probe cannot be enabled as it is.
    Error,
    /// It works, and the author should look.
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Error => "error",
            Self::Warning => "warning",
        })
    }
}

/// One thing `validate` found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    /// A stable, kebab-case name for the kind of finding, for tests and `--json`.
    pub code: &'static str,
    /// The manifest field it is about, as `contributes.rules[2].pattern`; empty for the folder.
    pub path: String,
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}[{}]: {}", self.severity, self.code, self.message)
        } else {
            write!(f, "{}[{}] {}: {}", self.severity, self.code, self.path, self.message)
        }
    }
}

/// The findings, collected in the order the manifest is read.
#[derive(Debug, Default)]
pub(crate) struct Findings(Vec<Finding>);

impl Findings {
    pub(crate) fn error(&mut self, code: &'static str, path: impl Into<String>, message: impl Into<String>) {
        self.push(Severity::Error, code, path, message);
    }

    pub(crate) fn warning(
        &mut self,
        code: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.push(Severity::Warning, code, path, message);
    }

    fn push(
        &mut self,
        severity: Severity,
        code: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.0.push(Finding { severity, code, path: path.into(), message: message.into() });
    }

    /// Errors so far, to tell whether a check added one.
    pub(crate) fn errors(&self) -> usize {
        self.0.iter().filter(|f| f.severity == Severity::Error).count()
    }
}

/// The result of validating a Probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub summary: Summary,
}

impl Report {
    /// Whether the Probe may be enabled as far as this validation can tell: no errors.
    pub fn is_ok(&self) -> bool {
        self.errors().next().is_none()
    }

    pub fn errors(&self) -> impl Iterator<Item = &Finding> {
        self.findings.iter().filter(|f| f.severity == Severity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &Finding> {
        self.findings.iter().filter(|f| f.severity == Severity::Warning)
    }

    /// The first finding with `code`.
    pub fn find(&self, code: &str) -> Option<&Finding> {
        self.findings.iter().find(|f| f.code == code)
    }
}

/// Check `folder`: its manifest against the format, and every contribution against its rules.
pub fn validate(folder: &ProbeFolder) -> Report {
    let mut out = Findings::default();
    identity::check(folder, &mut out);
    permissions::check(folder, &mut out);
    rules::check(folder, &mut out);
    commands::check(folder, &mut out);
    agents::check(folder, &mut out);
    paths::check_contributed_files(folder, &mut out);
    Report { findings: out.0, summary: permissions::summarize(folder) }
}

#[cfg(test)]
mod tests;
