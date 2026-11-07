//! What Settings' buttons do that no setting stores: reset the tips closed with "Don't show again", and the
//! kinds of file the default terminal takes over (the system call that sets them is AppKit's, so Swift's).
//! The rules and the store key are core's (`onboarding::DismissedTips`, `terminal::default_terminal`).

use wormhole_core::onboarding::{DISMISSED_TIPS_KEY, DismissedTips};
use wormhole_core::storage::store::Store;
use wormhole_core::terminal::default_terminal::SCRIPT_TYPES;

use wormhole_text::keys::{
    settings_profile_delete_question, settings_profile_start_from_copy, settings_terminal_default_failed,
};

use crate::error::FfiError;
use crate::isolation;

fn failed(err: impl std::fmt::Display) -> FfiError {
    FfiError::Start { reason: format!("{err:#}") }
}

/// How many tips were closed for good in this data folder's database.
#[uniffi::export]
pub fn dismissed_tips_count() -> Result<u32, FfiError> {
    isolation::require()?;
    count_in(&Store::open().map_err(failed)?)
}

/// Settings → Reset tips: every tip closed for good shows again where it belongs. How many were closed.
#[uniffi::export]
pub fn reset_tips() -> Result<u32, FfiError> {
    isolation::require()?;
    reset_in(&Store::open().map_err(failed)?)
}

fn count_in(store: &Store) -> Result<u32, FfiError> {
    let stored = store.meta(DISMISSED_TIPS_KEY).map_err(failed)?;
    // The key holds a JSON list; anything unreadable counts as nothing closed, as core's parse does.
    let keys = stored.and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok()).unwrap_or_default();
    Ok(u32::try_from(keys.len()).unwrap_or(u32::MAX))
}

fn reset_in(store: &Store) -> Result<u32, FfiError> {
    let closed = count_in(store)?;
    if closed > 0 {
        let mut tips = DismissedTips::parse(store.meta(DISMISSED_TIPS_KEY).map_err(failed)?.as_deref());
        tips.clear();
        store.set_meta(DISMISSED_TIPS_KEY, &tips.to_json()).map_err(failed)?;
    }
    Ok(closed)
}

/// "Delete <profile>? Its terminals, notes, tasks and settings are removed, ...": the question before a profile goes,
/// in the catalogs' words (they carry the name).
#[uniffi::export]
pub fn profile_delete_question(profile: String) -> String {
    settings_profile_delete_question(&profile)
}

/// "A copy of <profile>": the New profile sheet's choice of what the new profile starts as.
#[uniffi::export]
pub fn profile_copy_option(profile: String) -> String {
    settings_profile_start_from_copy(&profile)
}

/// "Couldn't make Wormhole the default terminal: <error>", in the catalogs' words.
#[uniffi::export]
pub fn default_terminal_failed(error: String) -> String {
    settings_terminal_default_failed(&error)
}

/// The content types the default terminal is the shell for (`.command` scripts and programs without an
/// extension), which "Make default" asks the system to give to this app.
#[uniffi::export]
pub fn default_terminal_script_types() -> Vec<String> {
    SCRIPT_TYPES.iter().map(|kind| (*kind).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use wormhole_core::storage::store::Store;

    use super::{count_in, reset_in};

    #[test]
    fn resetting_tips_forgets_the_closed_ones_and_says_how_many_there_were() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let store = Store::open_at(dir.path()).expect("a database");
        assert_eq!(count_in(&store).expect("count"), 0);
        store.set_meta("dismissed_tips", r#"["a","b"]"#).expect("two closed tips");
        assert_eq!(count_in(&store).expect("count"), 2);
        assert_eq!(reset_in(&store).expect("reset"), 2);
        assert_eq!(count_in(&store).expect("count"), 0);
        // Nothing to reset: nothing written, nothing said.
        assert_eq!(reset_in(&store).expect("reset"), 0);
    }
}
