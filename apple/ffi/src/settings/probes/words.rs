//! The words for a Probe's permissions and for what went wrong with one: core's data in the catalogs. The
//! English detail core gives (what the manifest reader or the disk said) stays as it is. The same sentences
//! the GPUI app's Extensions page writes (`settings_window/sections/extensions_text.rs`), which lives in the
//! app crate and cannot be linked here.

use wormhole_core::probe::approval::{ApproveError, NotApprovable, ReadOneError, VerifyError};
use wormhole_core::probe::hidden::reveal;
use wormhole_core::probe::scan::ScanError;
use wormhole_core::probe::validate::DeclaredPermission;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

/// A permission as a sentence.
pub(super) fn describe(permission: &DeclaredPermission) -> String {
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
pub(super) fn verify_text(err: &VerifyError) -> String {
    match err {
        VerifyError::Missing => text(SETTINGS_EXTENSIONS_COPY_MISSING).to_string(),
        VerifyError::Unreadable(why) => settings_extensions_copy_unreadable(&reveal(why)),
        VerifyError::Differs => text(SETTINGS_EXTENSIONS_COPY_DIFFERS).to_string(),
    }
}

/// Why a Probe could not be read for its sheet.
pub(super) fn read_error_text(err: &ReadOneError) -> String {
    match err {
        ReadOneError::Unreadable(err) => match err {
            ScanError::BadName => text(SETTINGS_EXTENSIONS_ERR_BAD_NAME).to_string(),
            ScanError::Link => text(SETTINGS_EXTENSIONS_ERR_LINK).to_string(),
            ScanError::IdMismatch { id } => settings_extensions_err_id_mismatch(&reveal(id)),
            ScanError::Read(err) => settings_extensions_err_read(&reveal(&err.to_string())),
        },
        ReadOneError::NotApprovable(NotApprovable::NeedsCode) => {
            text(SETTINGS_EXTENSIONS_ERR_NEEDS_CODE).to_string()
        }
        ReadOneError::NotApprovable(NotApprovable::Invalid(reason)) => reveal(reason).into_owned(),
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

/// A command line for a person to read: each argument written out, and quoted when it has a space.
pub(super) fn argv_line(argv: &[String]) -> String {
    argv.iter()
        .map(|arg| {
            let shown = reveal(arg);
            if shown.is_empty() || shown.contains(char::is_whitespace) || shown.contains('"') {
                format!("\"{}\"", shown.replace('"', "\\\""))
            } else {
                shown.into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::argv_line;

    #[test]
    fn a_command_line_is_written_so_that_its_arguments_can_be_told_apart() {
        let argv = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(argv_line(&argv(&["git", "log", "-1"])), "git log -1");
        assert_eq!(argv_line(&argv(&["say", "a b", ""])), "say \"a b\" \"\"");
        assert!(
            argv_line(&argv(&["x", "\u{202e}evil"])).contains("\\u{202e}"),
            "bidi controls are written out"
        );
    }
}
