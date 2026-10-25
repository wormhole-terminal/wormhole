//! The state of a Probe: what its folder is now against what the user approved.

use super::store::ApprovalStore;
use crate::probe::read::ProbeFolder;
use crate::probe::validate::{Report, Severity};
use crate::storage::paths::DataRoot;

/// Whether a Probe is on, and if not, why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeState {
    /// Valid and not approved.
    Off,
    /// Approved, the folder is what was approved, and the copy it runs from is there.
    On,
    /// Approved once, but the folder differs now (or the approved copy is gone): off until approved again.
    Changed,
    /// It cannot be enabled as it is; the first reason, in the words of `validate`.
    Invalid(String),
    /// It has code (`main`), which only a later level runs.
    Unsupported,
}

impl ProbeState {
    /// Whether a Probe in this state may run: only `On` does. A step that runs one asks
    /// [`runnable`](super::runnable), which adds the check of the copy's bytes.
    pub fn is_on(&self) -> bool {
        matches!(self, Self::On)
    }

    /// Whether the user can be asked to approve it now.
    pub fn can_approve(&self) -> bool {
        matches!(self, Self::Off | Self::Changed)
    }

    /// The state of `folder`, as `report` validated it, against `store` and the copies under `root`.
    pub fn of(root: &DataRoot, folder: &ProbeFolder, report: &Report, store: &ApprovalStore) -> Self {
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
        let id = &folder.manifest.probe.id;
        match store.get(id) {
            None => Self::Off,
            // The copy's existence only; its bytes are `Snapshot::verify`'s, which reads them.
            Some(approval)
                if approval.fingerprint == folder.fingerprint()
                    && store.snapshot(root, id).is_some_and(|copy| copy.exists()) =>
            {
                Self::On
            }
            Some(_) => Self::Changed,
        }
    }
}
