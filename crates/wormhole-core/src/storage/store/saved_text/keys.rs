//! Where the key the saved text is sealed with lives: one per profile (per database folder), as
//! `storage::secret_key` keeps it. The store is per profile, so profiles share nothing and deleting
//! one deletes its key. The key is kept under a random id the profile's database holds (`meta`), not
//! under the folder's path, which would name the user and the profile and break if the folder moved.
//!
//! A development build keeps it in [`DEV_KEY_FILE`] beside the text, and never in the real data folder
//! (there it keeps no text at all).

use crate::storage::secret_key::Purpose;

/// The development build's key file, in the profile's folder. For testing only: a key next to the
/// text it seals keeps nothing from anyone who can read the folder.
pub(crate) const DEV_KEY_FILE: &str = "scrollback-key.dev-only";

/// The saved text's key: the Keychain item's service is this, its account the profile's random id.
pub(super) const PURPOSE: Purpose = Purpose {
    what: "saved text key",
    keeps: "terminal text",
    keychain_service: "Wormhole saved terminal text",
    dev_key_file: DEV_KEY_FILE,
    dev_key_per_account: false,
};
