//! The state of a Probe: what its folder is now against what the user approved.

use super::store::ApprovalStore;
use crate::probe::read::ProbeFolder;
use crate::probe::validate::{Report, Severity};

/// Whether a Probe is on, and if not, why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeState {
    /// Valid and not approved.
    Off,
    /// Approved, and the folder is what was approved.
    On,
    /// Approved once, but the folder differs now: off until approved again.
    Changed,
    /// It cannot be enabled as it is; the first reason, in the words of `validate`.
    Invalid(String),
    /// It has code (`main`), which only a later level runs.
    Unsupported,
}

impl ProbeState {
    /// Whether a Probe in this state may run: only `On` does.
    pub fn is_on(&self) -> bool {
        matches!(self, Self::On)
    }

    /// Whether the user can be asked to approve it now.
    pub fn can_approve(&self) -> bool {
        matches!(self, Self::Off | Self::Changed)
    }

    /// The state of `folder`, as `report` validated it, against `store`.
    pub fn of(folder: &ProbeFolder, report: &Report, store: &ApprovalStore) -> Self {
        if let Some(first) = report.findings.iter().find(|finding| finding.severity == Severity::Error) {
            return Self::Invalid(if first.path.is_empty() {
                first.message.clone()
            } else {
                format!("{}: {}", first.path, first.message)
            });
        }
        if report.summary.has_code {
            return Self::Unsupported;
        }
        match store.get(&folder.manifest.probe.id) {
            None => Self::Off,
            Some(approval) if approval.fingerprint == folder.fingerprint() => Self::On,
            Some(_) => Self::Changed,
        }
    }
}
