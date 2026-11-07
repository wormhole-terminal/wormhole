//! The Extensions page: the Probes in the Probes folder with their states, the approval sheet's data, and the
//! approval itself. Core decides every rule (`probe::approval`: the state of each folder, what a request shows,
//! when a yes needs Touch ID, what an approval seals); this is the seam, with the words in the catalogs.
//!
//! An approval is exactly what the sheet showed: `sheet` reads the Probe once and keeps that request, `approve`
//! confirms it with the grants the person left on (a click, or the owner's authentication where core says Touch
//! ID) and records those bytes. Reading and approving read the approval keys, which may block or ask the
//! system: call them off the main thread. A test instance answers the authentication without a prompt
//! (`WORMHOLE_AUTH`); the tests never raise one.

mod sheet;
#[cfg(test)]
mod tests;
mod words;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use wormhole_core::probe::approval::{self, Entry, KeyProblem, ProbeState, Proof, Request};
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

pub use sheet::{ProbeChangeRecord, ProbePermissionRecord, ProbeSheetRecord};
use words::{approve_error_text, read_error_text, verify_text};

use super::error::SettingsError;
use crate::files_panel::{OwnerAnswer, OwnerAuthenticator};
use crate::isolation;

/// One folder of the Probes folder, as a row.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProbeRowRecord {
    /// The Probe's id, or its folder's name when its manifest does not read: what every call takes.
    pub id: String,
    pub title: String,
    /// "id · version · author".
    pub identity: String,
    pub description: String,
    /// `on`, `off`, `changed`, `unsupported` or `invalid`.
    pub state: String,
    pub state_word: String,
    /// A stored approval that is not this Wormhole's reads as off, in a warning's colour.
    pub unsealed: bool,
    /// The line under the row: what is wrong with a changed, unsupported or unsealed Probe, why an invalid one is.
    pub note: Option<String>,
    pub can_approve: bool,
    pub can_forget: bool,
}

/// The Probes folder as it was read.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProbeListRecord {
    pub rows: Vec<ProbeRowRecord>,
    /// Why every Probe reads as off (the approvals' key cannot be read), when it cannot.
    pub key_problem: Option<String>,
    /// More folders than a list can be: none are listed, and this says so.
    pub too_many: Option<String>,
    /// The Probes folder, as a person says it, and its path.
    pub folder: String,
    pub folder_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ProbeApprovalOutcome {
    Approved,
    /// Cancelled at the system's question, or no sheet was read for this Probe: nothing changed.
    Cancelled,
    Failed {
        message: String,
    },
}

#[derive(uniffi::Object)]
pub struct ProbesDesk {
    root: DataRoot,
    auth: Arc<dyn OwnerAuthenticator>,
    /// The request each sheet was opened with, until it is approved or the sheet is read again.
    requests: Mutex<HashMap<String, Request>>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn row(entry: &Entry) -> ProbeRowRecord {
    let (id, title, identity, description) = match &entry.probe {
        Some(probe) => (
            probe.id.clone(),
            reveal(&probe.name).into_owned(),
            settings_extensions_identity(&reveal(&probe.id), &reveal(&probe.version), &reveal(&probe.author)),
            reveal(&probe.description).into_owned(),
        ),
        None => (entry.folder.clone(), reveal(&entry.folder).into_owned(), String::new(), String::new()),
    };
    let unsealed = entry.unsealed && entry.state == ProbeState::Off;
    let (state, word, note) = match &entry.state {
        ProbeState::On => ("on", SETTINGS_EXTENSIONS_STATE_ON, None),
        ProbeState::Off => (
            "off",
            SETTINGS_EXTENSIONS_STATE_OFF,
            unsealed.then(|| text(SETTINGS_EXTENSIONS_UNSEALED_NOTE).to_string()),
        ),
        ProbeState::Changed => (
            "changed",
            SETTINGS_EXTENSIONS_STATE_CHANGED,
            Some(match &entry.copy_problem {
                Some(problem) => verify_text(problem),
                None => text(SETTINGS_EXTENSIONS_CHANGED_NOTE).to_string(),
            }),
        ),
        ProbeState::Unsupported => (
            "unsupported",
            SETTINGS_EXTENSIONS_STATE_UNSUPPORTED,
            Some(text(SETTINGS_EXTENSIONS_UNSUPPORTED_NOTE).to_string()),
        ),
        ProbeState::Invalid(reason) => (
            "invalid",
            SETTINGS_EXTENSIONS_STATE_INVALID,
            Some(settings_extensions_invalid_reason(&reveal(reason))),
        ),
    };
    ProbeRowRecord {
        id,
        title,
        identity,
        description,
        state: state.to_string(),
        state_word: text(if unsealed { SETTINGS_EXTENSIONS_STATE_UNSEALED } else { word }).to_string(),
        unsealed,
        note,
        can_approve: entry.state.can_approve(),
        can_forget: entry.can_forget(),
    }
}

impl ProbesDesk {
    pub(crate) fn at(root: DataRoot, auth: Arc<dyn OwnerAuthenticator>) -> Arc<Self> {
        Arc::new(Self { root, auth, requests: Mutex::new(HashMap::new()) })
    }

    fn requests(&self) -> std::sync::MutexGuard<'_, HashMap<String, Request>> {
        self.requests.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn grants(request: &Request, granted: &[u32]) -> approval::Grants {
        let mut grants = request.initial_grants();
        for (at, permission) in request.summary().permissions.iter().enumerate() {
            grants.set(permission, granted.contains(&u32::try_from(at).unwrap_or(u32::MAX)));
        }
        grants
    }
}

#[uniffi::export]
impl ProbesDesk {
    /// Fails unless this is an isolated instance of the experiment (the approvals live in its data folder).
    #[uniffi::constructor]
    pub fn new(authenticator: Arc<dyn OwnerAuthenticator>) -> Result<Arc<Self>, SettingsError> {
        isolation::require()?;
        Ok(Self::at(DataRoot::current(), authenticator))
    }

    /// Every folder of the Probes folder with its state. Reads (and tidies the approvals of Probes that are gone,
    /// after a read that saw the whole folder): reads up to 4 MiB per Probe, off the main thread.
    pub fn list(&self) -> ProbeListRecord {
        let folder_path = self.root.probes_dir();
        let shown = |_: &()| folder_path.to_string_lossy().into_owned();
        let empty = ProbeListRecord {
            rows: Vec::new(),
            key_problem: None,
            too_many: None,
            folder: wormhole_core::platform::paths::home()
                .and_then(|home| {
                    folder_path.strip_prefix(home).ok().map(|rest| format!("~/{}", rest.display()))
                })
                .unwrap_or_else(|| shown(&())),
            folder_path: shown(&()),
        };
        let Ok(listing) = approval::list(&self.root) else {
            return ProbeListRecord {
                too_many: Some(settings_extensions_too_many(&MAX_ENTRIES.to_string())),
                ..empty
            };
        };
        if let Err(err) = approval::sweep(&self.root, listing.present().as_ref()) {
            tracing::warn!("tidying the approved Probes: {err}");
        }
        let key_problem = listing.key_problem.as_ref().map(|problem| {
            match problem {
                KeyProblem::Unreadable => text(SETTINGS_EXTENSIONS_KEY_UNAVAILABLE),
                KeyProblem::InRealDataFolder => text(SETTINGS_EXTENSIONS_DEV_DATA_FOLDER),
            }
            .to_string()
        });
        ProbeListRecord { rows: listing.entries.iter().map(row).collect(), key_problem, ..empty }
    }

    /// Reads Probe `id` once and keeps what it read: the sheet shows it, and `approve` records exactly it. A Probe
    /// that cannot be approved (it changed into an invalid one since the list) says why.
    pub fn sheet(&self, id: String) -> Result<ProbeSheetRecord, SettingsError> {
        let request = approval::read_one(&self.root, &id).map_err(|err| SettingsError::Invalid {
            reason: settings_extensions_cannot_approve(&reveal(&id), &read_error_text(&err)),
        })?;
        let record = sheet::of(&request);
        self.requests().insert(id, request);
        Ok(record)
    }

    /// Whether approving with the permissions at `granted` left on needs Touch ID (a permission of that level, or a
    /// dangerous combination): the sheet's button and its line under it.
    pub fn needs_touch_id(&self, id: String, granted: Vec<u32>) -> bool {
        self.requests().get(&id).is_some_and(|request| {
            request.confirmation(&Self::grants(request, &granted)) == Confirmation::TouchId
        })
    }

    /// Approves what the sheet of `id` showed, with the permissions at `granted` on. Blocks while the owner answers
    /// when core says Touch ID. Cancel, a refused authentication and a failed write change nothing.
    pub fn approve(&self, id: String, granted: Vec<u32>) -> ProbeApprovalOutcome {
        let Some(request) = self.requests().get(&id).cloned() else { return ProbeApprovalOutcome::Cancelled };
        let grants = Self::grants(&request, &granted);
        let name = reveal(&request.folder().manifest.probe.name).into_owned();
        let failed = |why: &str| ProbeApprovalOutcome::Failed {
            message: settings_extensions_not_approved(&name, why),
        };
        let proof = match request.confirmation(&grants) {
            Confirmation::Click => Proof::Click,
            Confirmation::TouchId => match self.auth.authenticate(settings_extensions_auth_reason(&name)) {
                OwnerAnswer::Approved => Proof::TouchId,
                OwnerAnswer::Cancelled => return ProbeApprovalOutcome::Cancelled,
                OwnerAnswer::Failed { message } => return failed(&message),
            },
        };
        let confirmed = match request.confirm(&grants, proof) {
            Ok(confirmed) => confirmed,
            Err(err) => return failed(&err.to_string()),
        };
        match approval::approve(&self.root, &request, &confirmed, now()) {
            Ok(()) => {
                self.requests().remove(&id);
                ProbeApprovalOutcome::Approved
            }
            Err(err) => failed(&approve_error_text(&err)),
        }
    }

    /// Turns Probe `id` off, or forgets what is stored for it: its approval and its copy go, with no question.
    pub fn turn_off(&self, id: String) -> Result<(), SettingsError> {
        approval::turn_off(&self.root, &id).map_err(|err| SettingsError::Invalid {
            reason: settings_extensions_not_turned_off(&reveal(&id), &err.to_string()),
        })
    }
}
