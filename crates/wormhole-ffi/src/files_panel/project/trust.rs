//! Approving a project: the owner is asked first (Touch ID, an Apple Watch or the password, which only the
//! system can do, so Swift answers through [`OwnerAuthenticator`]), and what is trusted is exactly what
//! the tab last showed. Core's `TrustStore` holds the approval, bound to a fingerprint of the files that
//! can make the app run or type something: one changed since reads "changed" and asks again.

use wormhole_core::project::TrustRequest;

use super::service::ProjectService;

/// What the owner's authentication came to.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum OwnerAnswer {
    Approved,
    /// The person, the system or the app dismissed the dialog: nothing is said.
    Cancelled,
    /// The person did not prove ownership, or it can't be done here.
    Failed {
        message: String,
    },
}

/// What Swift implements to ask the owner. Called on the thread that called `approve`, which blocks until
/// it answers: Swift runs `approve` off the main thread. A test instance answers without a prompt
/// (`WORMHOLE_AUTH`); tests never raise one.
#[uniffi::export(with_foreign)]
pub trait OwnerAuthenticator: Send + Sync {
    fn authenticate(&self, reason: String) -> OwnerAnswer;
}

/// What `approve` did.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum ApproveOutcome {
    Approved,
    /// Declined, dismissed, or another approval is already waiting: nothing changed.
    Declined,
    /// Not trusted, and why (the owner failed, or the project does not load).
    Failed {
        message: String,
    },
}

#[uniffi::export]
impl ProjectService {
    /// Trusts the project at `root` after the owner's authentication: Touch ID (or the password) first.
    /// Blocks while the person answers. What is trusted is the content the tab last showed, so files
    /// that changed since make the project read "changed" rather than be approved unseen.
    pub fn approve(&self, root: String) -> ApproveOutcome {
        let Some(folder) = super::service::open(&root) else {
            return ApproveOutcome::Failed { message: format!("{root} has no project folder") };
        };
        let request = {
            let mut hub = self.lock();
            if hub.approving {
                return ApproveOutcome::Declined;
            }
            match hub.approval_request(&folder) {
                Ok(request) => {
                    hub.error = None;
                    hub.approving = true;
                    request
                }
                Err(why) => {
                    let message = format!("Can't trust this folder until it loads: {why}");
                    hub.error = Some(message.clone());
                    return ApproveOutcome::Failed { message };
                }
            }
        };
        // The lock is not held while the person looks at the dialog.
        let answer = self.auth.authenticate(reason(&request));
        let mut hub = self.lock();
        hub.approving = false;
        let outcome = match answer {
            OwnerAnswer::Approved => {
                // Exactly what was shown: if the files changed since, the project reads "changed".
                hub.trust.trust_request(&request);
                tracing::info!(root = %request.root.display(), "project trusted");
                ApproveOutcome::Approved
            }
            OwnerAnswer::Cancelled => ApproveOutcome::Declined,
            OwnerAnswer::Failed { message } => {
                hub.error = Some(format!("Not trusted: {message}"));
                ApproveOutcome::Failed { message }
            }
        };
        hub.save();
        drop(hub);
        self.changed();
        outcome
    }

    /// Takes the approval of the project at `root` away.
    pub fn revoke(&self, root: String) {
        let mut hub = self.lock();
        hub.trust.revoke(std::path::Path::new(&root));
        hub.save();
        drop(hub);
        self.changed();
    }
}

fn reason(request: &TrustRequest) -> String {
    let name = request.root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    format!("trust the project folder \u{201c}{name}\u{201d} and let it run commands")
}
