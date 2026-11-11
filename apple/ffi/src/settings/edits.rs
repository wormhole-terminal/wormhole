//! How many times Settings (a control, an import, a reset) changed each profile's settings in this process, so that
//! the agent-access guard can tell an edit made inside Wormhole from `config.json` changed by something else
//! (`aux_backends::access::modes`; the GPUI app's `ui_edits`). The folder is the key: it is what both sides know.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

static EDITS: Mutex<BTreeMap<PathBuf, u64>> = Mutex::new(BTreeMap::new());

/// Settings changed the profile whose folder is `dir`.
pub(crate) fn note(dir: &Path) {
    *EDITS.lock().unwrap_or_else(PoisonError::into_inner).entry(dir.to_path_buf()).or_default() += 1;
}

/// How many changes Settings made to the profile whose folder is `dir`.
pub(crate) fn count(dir: &Path) -> u64 {
    EDITS.lock().unwrap_or_else(PoisonError::into_inner).get(dir).copied().unwrap_or_default()
}
