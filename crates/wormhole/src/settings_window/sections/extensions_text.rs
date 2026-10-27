//! The words for what went wrong with a Probe: core's errors in the catalogs. The English detail core
//! gives (what the manifest reader or the disk said) stays as it is, after the catalog's own sentence.

use wormhole_core::probe::approval::{ApproveError, NotApprovable, ReadOneError, VerifyError};
use wormhole_core::probe::hidden::reveal;
use wormhole_core::probe::scan::ScanError;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

/// Why a Probe could not be read for its sheet.
pub(super) fn read_error_text(err: &ReadOneError) -> String {
    match err {
        ReadOneError::Unreadable(err) => scan_error_text(err),
        ReadOneError::NotApprovable(NotApprovable::NeedsCode) => {
            text(SETTINGS_EXTENSIONS_ERR_NEEDS_CODE).to_string()
        }
        ReadOneError::NotApprovable(NotApprovable::Invalid(reason)) => reveal(reason).into_owned(),
    }
}

fn scan_error_text(err: &ScanError) -> String {
    match err {
        ScanError::BadName => text(SETTINGS_EXTENSIONS_ERR_BAD_NAME).to_string(),
        ScanError::Link => text(SETTINGS_EXTENSIONS_ERR_LINK).to_string(),
        ScanError::IdMismatch { id } => settings_extensions_err_id_mismatch(&reveal(id)),
        ScanError::Read(err) => settings_extensions_err_read(&reveal(&err.to_string())),
    }
}

/// Why an approval could not be recorded.
pub(super) fn approve_error_text(err: &ApproveError) -> String {
    match err {
        ApproveError::Snapshot(err) => settings_extensions_err_copy(&err.to_string()),
        ApproveError::Store(err) => settings_extensions_err_store(&err.to_string()),
        ApproveError::Key(err) => settings_extensions_err_key(&format!("{err:#}")),
        ApproveError::Unconfirmed => text(SETTINGS_EXTENSIONS_ERR_UNCONFIRMED).to_string(),
        ApproveError::InRealDataFolder => text(SETTINGS_EXTENSIONS_DEV_DATA_FOLDER).to_string(),
    }
}

/// What a row says when the approved copy of its Probe is not what was approved.
pub(super) fn verify_text(err: &VerifyError) -> String {
    match err {
        VerifyError::Missing => text(SETTINGS_EXTENSIONS_COPY_MISSING).to_string(),
        VerifyError::Unreadable(why) => settings_extensions_copy_unreadable(&reveal(why)),
        VerifyError::Differs => text(SETTINGS_EXTENSIONS_COPY_DIFFERS).to_string(),
    }
}
