//! The project trust window's backend (`features/project/trust_window.rs`): what a project's `.wormhole/` folder
//! would run, in core's words (`project::trust`), its standing against the profile's approvals
//! (`TrustStore`), and the approval itself.
//!
//! Approving binds to what was *shown*: `review` keeps the request it described, `hold_reviewed` fixes it before
//! the owner is asked and `approve_reviewed` trusts exactly that, so a file that changed since (also while the
//! dialog was up) reads "changed" again and must be reviewed anew. Nothing here asks
//! for Touch ID: that is the platform's (Swift's `Authenticating`), asked before `approve_reviewed` is called.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use wormhole_core::platform::watch::{self as os_watch, Watch};
use wormhole_core::project::{ProjectFolder, TrustRequest, TrustStatus, TrustStore};
use wormhole_core::storage::paths::{DataRoot, ProfileId};

use super::common::{AuxError, AuxListener, Waker, locked, resolve_data_dir};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReviewStatusRecord {
    Trusted,
    Changed,
    Untrusted,
}

/// One line a project would run, in three pieces (`TrustLine`): what runs it, what it runs, where or how.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReviewLineRecord {
    pub what: String,
    pub runs: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TrustReviewRecord {
    pub name: String,
    /// The project folder's name on disk (`.wormhole`, or an older project's `.andromeda`).
    pub directory: String,
    pub status: ReviewStatusRecord,
    pub lines: Vec<ReviewLineRecord>,
    /// Why there is nothing to review: no project folder, or a file of it that does not load.
    pub problem: Option<String>,
}

#[derive(uniffi::Object)]
pub struct TrustDesk {
    root: PathBuf,
    profile_dir: PathBuf,
    shown: Mutex<Option<TrustRequest>>,
    /// What the owner is being asked about: taken from `shown` before the dialog, so a `review` that runs while
    /// it is up (the watch, a `git pull`) never changes what the answer approves.
    held: Mutex<Option<TrustRequest>>,
    waker: Arc<Waker>,
    watch: Mutex<Option<Watch>>,
}

#[uniffi::export]
impl TrustDesk {
    /// The project at `root`, approved in `profile` (the default one when `None`) of the data folder.
    #[uniffi::constructor]
    pub fn new(
        root: String,
        profile: Option<String>,
        data_dir: Option<String>,
        listener: Arc<dyn AuxListener>,
    ) -> Result<Arc<Self>, AuxError> {
        let data = DataRoot::new(resolve_data_dir(data_dir)?);
        let paths = match profile {
            Some(id) => data.profile(&ProfileId::parse(&id).map_err(AuxError::failed)?),
            None => data.default_profile(),
        };
        Ok(Arc::new(Self {
            root: PathBuf::from(root),
            profile_dir: paths.dir().to_path_buf(),
            shown: Mutex::new(None),
            held: Mutex::new(None),
            waker: Arc::new(Waker::new(listener)),
            watch: Mutex::new(None),
        }))
    }

    /// The project as it is on disk now, and keeps what it shows for `approve_reviewed`. Re-arms the wake.
    pub fn review(&self) -> TrustReviewRecord {
        self.waker.rearm();
        let name = self.root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let directory = wormhole_core::project::dir_name_at(&self.root);
        let Some(folder) = ProjectFolder::open(&self.root) else {
            *locked(&self.shown) = None;
            let problem = Some(format!("There is no {directory} folder in this project."));
            return TrustReviewRecord {
                name,
                directory,
                status: ReviewStatusRecord::Untrusted,
                lines: Vec::new(),
                problem,
            };
        };
        let snapshot = folder.snapshot();
        let (store, _) = TrustStore::load_or_recover(&self.profile_dir);
        let status = match store.status_of(&snapshot) {
            TrustStatus::Trusted => ReviewStatusRecord::Trusted,
            TrustStatus::Changed => ReviewStatusRecord::Changed,
            TrustStatus::Untrusted => ReviewStatusRecord::Untrusted,
        };
        let request = snapshot.trust_request();
        let problem = match &request {
            Some(_) => None,
            None => Some(snapshot.errors().next().map(|e| e.to_string()).unwrap_or_default()),
        };
        let lines = request
            .iter()
            .flat_map(|r| &r.commands)
            .map(|l| ReviewLineRecord { what: l.what.clone(), runs: l.runs.clone(), note: l.note.clone() })
            .collect();
        *locked(&self.shown) = request;
        TrustReviewRecord { name, directory, status, lines, problem }
    }

    /// Call before asking the owner: fixes what the answer approves to what `review` last showed. `false` when
    /// nothing reviewed can be approved.
    pub fn hold_reviewed(&self) -> bool {
        let shown = locked(&self.shown).clone();
        let holds = shown.is_some();
        *locked(&self.held) = shown;
        holds
    }

    /// Trusts exactly what `hold_reviewed` took, once. Call only after the owner authenticated: files changed
    /// since the review read "changed" afterwards and must be reviewed anew.
    pub fn approve_reviewed(&self) -> Result<(), AuxError> {
        let request = locked(&self.held)
            .take()
            .ok_or_else(|| AuxError::failed("there is nothing reviewed to approve"))?;
        let (mut store, _) = TrustStore::load_or_recover(&self.profile_dir);
        store.trust_request(&request);
        store.save().map_err(AuxError::failed)?;
        self.waker.notify();
        Ok(())
    }

    /// Takes the approval back.
    pub fn revoke(&self) -> Result<(), AuxError> {
        let (mut store, _) = TrustStore::load_or_recover(&self.profile_dir);
        if store.revoke(&self.root) {
            store.save().map_err(AuxError::failed)?;
        }
        self.waker.notify();
        Ok(())
    }

    /// Follows the project folder: a wake when a file of it changes (`git pull`). `false` where it cannot.
    pub fn follow(&self) -> bool {
        let Some(folder) = ProjectFolder::open(&self.root) else { return false };
        let waker = Arc::clone(&self.waker);
        match os_watch::watch(&[folder.dir.as_path()], move |_| waker.notify()) {
            Ok(watch) => {
                *locked(&self.watch) = Some(watch);
                true
            }
            Err(_) => false,
        }
    }

    pub fn unfollow(&self) {
        *locked(&self.watch) = None;
    }
}
