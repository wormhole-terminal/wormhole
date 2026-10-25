//! What the approval sheet is made from, and what approving it takes.
//!
//! A [`Request`] holds the folder as it was read for the sheet. What is shown, what is fingerprinted
//! and what the copy is made from are those same bytes.

use std::collections::BTreeSet;
use std::fmt;

use super::store::{Approval, ApprovalStore, permission_id};
use crate::probe::read::ProbeFolder;
use crate::probe::validate::{Confirmation, DeclaredPermission, Summary, validate};

/// Why a Probe cannot be put in front of the user to approve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotApprovable {
    /// `validate` found an error; its first reason.
    Invalid(String),
    /// It has code, which nothing runs yet.
    NeedsCode,
}

impl fmt::Display for NotApprovable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(reason) => f.write_str(reason),
            Self::NeedsCode => f.write_str("it runs code, which this version does not support"),
        }
    }
}

impl std::error::Error for NotApprovable {}

/// What changed since the approval a Probe in `Changed` was given under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changes {
    /// Permissions it declares now that the approval did not show.
    pub added: Vec<DeclaredPermission>,
    /// Permissions the approval showed that it no longer declares.
    pub removed: Vec<String>,
}

impl Changes {
    /// Nothing about the permissions differs: the files changed, and nothing the sheet lists.
    pub fn files_only(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

/// Which declared permissions the user leaves on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grants(BTreeSet<String>);

impl Grants {
    /// Everything on, except what an earlier approval of the same Probe had refused.
    pub fn initial(summary: &Summary, earlier: Option<&Approval>) -> Self {
        Self(
            summary
                .permissions
                .iter()
                .map(permission_id)
                .filter(|id| earlier.is_none_or(|a| !a.declared.contains(id) || a.granted.contains(id)))
                .collect(),
        )
    }

    pub fn is_granted(&self, permission: &DeclaredPermission) -> bool {
        self.0.contains(&permission_id(permission))
    }

    pub fn set(&mut self, permission: &DeclaredPermission, on: bool) {
        let id = permission_id(permission);
        if on {
            self.0.insert(id);
        } else {
            self.0.remove(&id);
        }
    }

    pub fn toggle(&mut self, permission: &DeclaredPermission) {
        let on = !self.is_granted(permission);
        self.set(permission, on);
    }
}

/// A Probe about to be approved.
#[derive(Debug, Clone)]
pub struct Request {
    folder: ProbeFolder,
    fingerprint: String,
    summary: Summary,
    earlier: Option<Approval>,
}

impl Request {
    /// The request for `folder`, or why it cannot be approved. `store` gives the earlier approval,
    /// for the diff.
    pub fn new(folder: ProbeFolder, store: &ApprovalStore) -> Result<Self, NotApprovable> {
        let report = validate(&folder);
        if let Some(first) = report.errors().next() {
            let at = if first.path.is_empty() { String::new() } else { format!("{}: ", first.path) };
            return Err(NotApprovable::Invalid(format!("{at}{}", first.message)));
        }
        if report.summary.has_code {
            return Err(NotApprovable::NeedsCode);
        }
        let earlier = store.get(&folder.manifest.probe.id).cloned();
        Ok(Self { fingerprint: folder.fingerprint(), summary: report.summary, folder, earlier })
    }

    pub fn id(&self) -> &str {
        &self.folder.manifest.probe.id
    }

    pub fn folder(&self) -> &ProbeFolder {
        &self.folder
    }

    pub fn summary(&self) -> &Summary {
        &self.summary
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// The first twelve hexadecimal digits, for the sheet.
    pub fn short_fingerprint(&self) -> &str {
        &self.fingerprint[..self.fingerprint.len().min(12)]
    }

    pub fn file_count(&self) -> usize {
        self.folder.files.len()
    }

    /// What changed since the earlier approval; `None` when there was none.
    pub fn changes(&self) -> Option<Changes> {
        let earlier = self.earlier.as_ref()?;
        let now: Vec<String> = self.summary.permissions.iter().map(permission_id).collect();
        let added = self
            .summary
            .permissions
            .iter()
            .filter(|p| !earlier.declared.contains(&permission_id(p)))
            .cloned()
            .collect();
        let removed = earlier.declared.iter().filter(|id| !now.contains(id)).cloned().collect();
        Some(Changes { added, removed })
    }

    /// The grants the sheet starts with.
    pub fn initial_grants(&self) -> Grants {
        Grants::initial(&self.summary, self.earlier.as_ref())
    }

    /// How approving with `grants` must be confirmed: Touch ID when any permission left on is that
    /// level, or when the Probe makes a dangerous combination (declared, whatever is left on: a
    /// combination is the part a person misses), a click otherwise.
    pub fn confirmation(&self, grants: &Grants) -> Confirmation {
        let touch = !self.summary.combinations.is_empty()
            || self
                .summary
                .permissions
                .iter()
                .any(|p| p.confirmation == Confirmation::TouchId && grants.is_granted(p));
        if touch { Confirmation::TouchId } else { Confirmation::Click }
    }

    pub(super) fn approval(&self, grants: &Grants, now: u64) -> Approval {
        Approval {
            fingerprint: self.fingerprint.clone(),
            version: self.folder.manifest.probe.version.clone(),
            declared: self.summary.permissions.iter().map(permission_id).collect(),
            granted: self
                .summary
                .permissions
                .iter()
                .filter(|p| grants.is_granted(p))
                .map(permission_id)
                .collect(),
            approved_at: now,
        }
    }
}
