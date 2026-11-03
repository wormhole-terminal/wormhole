//! The words for a Probe's permissions and for an approved copy that is not what was approved: core's data in the
//! catalogs. The English detail core gives (what the manifest reader or the disk said) stays as it is.

use wormhole_core::probe::approval::VerifyError;
use wormhole_core::probe::hidden::reveal;
use wormhole_core::probe::validate::DeclaredPermission;
use wormhole_text::keys::{
    SETTINGS_EXTENSIONS_COPY_DIFFERS, SETTINGS_EXTENSIONS_COPY_MISSING, SETTINGS_EXTENSIONS_PERM_CODE,
    SETTINGS_EXTENSIONS_PERM_NOTIFY, settings_extensions_copy_unreadable, settings_extensions_perm_agents,
    settings_extensions_perm_env, settings_extensions_perm_open, settings_extensions_perm_other,
    settings_extensions_perm_process,
};
use wormhole_text::text;

/// A permission as a sentence, for the places that list it without a switch.
pub(in crate::settings_window) fn describe(permission: &DeclaredPermission) -> String {
    let detail = reveal(&permission.detail);
    match permission.key {
        "process" => settings_extensions_perm_process(&detail),
        "env" => settings_extensions_perm_env(&detail),
        "open" => settings_extensions_perm_open(&detail),
        "notify" => text(SETTINGS_EXTENSIONS_PERM_NOTIFY).to_string(),
        "agents" => settings_extensions_perm_agents(detail.strip_prefix("install-hooks:").unwrap_or(&detail)),
        "main" => text(SETTINGS_EXTENSIONS_PERM_CODE).to_string(),
        key => settings_extensions_perm_other(key, &detail),
    }
}

/// What a row says when the approved copy of its Probe is not what was approved.
pub(in crate::settings_window) fn verify_text(err: &VerifyError) -> String {
    match err {
        VerifyError::Missing => text(SETTINGS_EXTENSIONS_COPY_MISSING).to_string(),
        VerifyError::Unreadable(why) => settings_extensions_copy_unreadable(&reveal(why)),
        VerifyError::Differs => text(SETTINGS_EXTENSIONS_COPY_DIFFERS).to_string(),
    }
}
