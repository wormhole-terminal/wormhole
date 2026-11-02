//! A data folder that can't be opened. Without its database the app still runs, but nothing is saved: no
//! windows, no notes, no settings that live there. That must not be silent, so the failure is kept here
//! for the first window to say (`app::launch`), and goes to standard error and the log at once, where
//! a launch from a terminal (or Console) shows it.
//!
//! A global of its own and not a field of `AppState`: the profile data that fails to open is made before
//! the shared state exists.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The folder of a profile that couldn't be used, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unusable {
    /// The profile whose database it is.
    pub profile: String,
    pub folder: PathBuf,
    pub error: String,
}

static UNUSABLE: OnceLock<Unusable> = OnceLock::new();

/// Remember that the data folder of `profile` couldn't be opened because of `error`, and log it. The first
/// failure stays: it is the shown profile's, which opens first.
pub(crate) fn record(profile: &str, folder: &Path, error: &anyhow::Error) {
    let error = format!("{error:#}");
    tracing::error!(profile, folder = %folder.display(), "persistence disabled: {error}");
    // The log may be the very thing that can't be written, and a launch from a terminal should see why.
    eprintln!(
        "Wormhole: the profile {profile} will save nothing: can't use its data folder {}: {error}",
        folder.display()
    );
    let _ = UNUSABLE.set(Unusable { profile: profile.to_owned(), folder: folder.to_path_buf(), error });
}

/// What went wrong with the data folder, if anything did.
pub(crate) fn unusable() -> Option<&'static Unusable> {
    UNUSABLE.get()
}
