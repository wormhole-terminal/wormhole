//! What macOS does here and Linux has no equivalent of yet. Nothing builds it (macOS is the only
//! target), so the stand-ins answer as core did before the edge existed: no Keychain, nothing to keep
//! out of backups, no macOS version.

pub(super) mod backup;
pub(super) mod keychain;
pub(super) mod rename;
pub(super) mod system;
