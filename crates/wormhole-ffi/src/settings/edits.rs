//! The agent access mode Settings (a control, an import, a reset) last set for each profile in this process, so that
//! the agent-access guard can tell the person's own change from `config.json` changed by something else
//! (`aux_backends::access::modes`; the GPUI app's `ui_edits`). The value is kept, not a count of edits: a count says
//! that the person changed the mode, and whatever the file holds at the next look then passes as theirs. A change of
//! any other key is not noted: it says nothing about who widened the mode. The folder is the key: it is what both
//! sides know.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use wormhole_core::config::AgentAccess;

/// The person's last change of a profile's mode in Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Edit {
    /// Which of their changes this is, counted from one: tells a new one from the one a guard already took in.
    pub serial: u64,
    pub mode: AgentAccess,
}

static EDITS: Mutex<BTreeMap<PathBuf, Edit>> = Mutex::new(BTreeMap::new());

/// Settings sets the agent access mode of the profile whose folder is `dir` to `mode`, and `write` puts it in the
/// file. Noted before the file is written, since the guard follows the file and may look the moment it changes, and
/// held until it is: a look that sees the edit ([`last`]) and reads the file after that reads what was written.
pub(crate) fn write(dir: &Path, mode: AgentAccess, write: impl FnOnce()) {
    let mut edits = EDITS.lock().unwrap_or_else(PoisonError::into_inner);
    let serial = edits.get(dir).map_or(0, |edit| edit.serial) + 1;
    edits.insert(dir.to_path_buf(), Edit { serial, mode });
    write();
}

/// The last change Settings made to the agent access mode of the profile whose folder is `dir`, once its write is
/// over.
pub(crate) fn last(dir: &Path) -> Option<Edit> {
    EDITS.lock().unwrap_or_else(PoisonError::into_inner).get(dir).copied()
}
