//! Where the key the saved text is sealed with lives.
//!
//! One key per profile (per database folder): the store is per profile, so profiles share nothing
//! and deleting one deletes its key. The key is kept under a random id the profile's database holds
//! (`meta`), not under the folder's path, which would name the user and the profile and break if the
//! folder moved.
//!
//! - A release build keeps it in the login Keychain, as a generic password, made the first time a
//!   text is saved or read. The app that made it reads it without a prompt; another program running
//!   as the user gets one.
//! - A development build (debug assertions, or the `dev-key` feature that devctl builds turn on:
//!   `cargo run`, `scripts/dev.sh`, devctl instances, tests, release-profile dev builds) never
//!   touches the Keychain: a rebuilt binary would ask for it again after every build, and an
//!   isolated instance must not reach the user's. It keeps the key in a file beside the text,
//!   [`DEV_KEY_FILE`], which protects nothing and says so in its name, and never in the real data
//!   folder (there it keeps no text at all); a release build deletes such a file it finds.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};

use super::cipher::{KEY_LEN, Key, new_key};
use crate::platform::keychain;
use crate::platform::paths;

/// The development build's key file, in the profile's folder. For testing only: a key next to the
/// text it seals keeps nothing from anyone who can read the folder.
pub(crate) const DEV_KEY_FILE: &str = "scrollback-key.dev-only";

/// The Keychain item's service; its account is the profile's random id.
const KEYCHAIN_SERVICE: &str = "Wormhole saved terminal text";

/// The key kept under `account` in the login Keychain, made there on first use.
pub(super) fn keychain(account: &str) -> Result<Key> {
    match read_keychain(account) {
        Ok(Some(key)) => Ok(key),
        Ok(None) => {
            let key = new_key()?;
            match keychain::add(KEYCHAIN_SERVICE, account, key.as_ref()) {
                Ok(()) => {
                    tracing::info!("made a saved text key in the Keychain");
                    Ok(key)
                }
                // Made by someone else in between: theirs is the one, never overwritten.
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    read_keychain(account)?.context("the Keychain has a saved text key that can't be read")
                }
                Err(err) => bail!("keeping a new saved text key in the Keychain: {err}"),
            }
        }
        Err(err) => Err(err),
    }
}

fn read_keychain(account: &str) -> Result<Option<Key>> {
    match keychain::find(KEYCHAIN_SERVICE, account) {
        Ok(Some(found)) => to_key(found.as_slice(), "the Keychain's saved text key").map(Some),
        Ok(None) => Ok(None),
        // Denied, cancelled, a locked Keychain: the text isn't kept this time.
        Err(err) => bail!("reading the saved text key from the Keychain: {err}"),
    }
}

pub(super) fn forget_keychain(account: &str) -> Result<()> {
    keychain::delete(KEYCHAIN_SERVICE, account)
        .map_err(|err| anyhow::anyhow!("deleting the saved text key: {err}"))
}

fn to_key(bytes: &[u8], what: &str) -> Result<Key> {
    if bytes.len() != KEY_LEN {
        bail!("{what} has {} bytes, not {KEY_LEN}", bytes.len());
    }
    let mut key = Key::new([0; KEY_LEN]);
    key.copy_from_slice(bytes);
    Ok(key)
}

/// The real data folder (the one without `WORMHOLE_DATA_DIR`).
fn real_data_dir() -> Option<PathBuf> {
    paths::data_base().map(|base| base.join(crate::storage::paths::data_dir_name()))
}

/// Whether `dir` is the real data folder or inside it.
fn in_real_data_dir(dir: &Path) -> bool {
    let Some(real) = real_data_dir() else { return false };
    let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    canonical(dir).starts_with(canonical(&real))
}

/// The development build's key, in [`DEV_KEY_FILE`] in `dir` (owner-only), made on first use;
/// none in the real data folder, where a key beside the text would leave the user's text open.
pub(super) fn dev_key_file(dir: &Path) -> Result<Key> {
    if in_real_data_dir(dir) {
        bail!("a development build keeps no terminal text in the real data folder (set WORMHOLE_DATA_DIR)");
    }
    let path = dir.join(DEV_KEY_FILE);
    match crate::storage::files::read_capped(&path, 64, crate::storage::files::LinkPolicy::Follow) {
        Ok(found) => to_key(zeroize::Zeroizing::new(found).as_slice(), &path.display().to_string()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let key = new_key()?;
            crate::storage::files::write_atomic(
                &path,
                key.as_ref(),
                crate::storage::files::LinkPolicy::Follow,
            )
            .with_context(|| format!("writing {}", path.display()))?;
            Ok(key)
        }
        Err(err) => Err(err).with_context(|| format!("reading {}", path.display())),
    }
}

pub(super) fn forget_dev_key_file(dir: &Path) -> Result<()> {
    remove_if_there(&dir.join(DEV_KEY_FILE))
}

/// A release build removes a development key file it finds beside its text (one a development
/// build left there): it protects nothing.
pub(super) fn remove_stray_dev_key(dir: &Path) {
    if let Err(err) = remove_if_there(&dir.join(DEV_KEY_FILE)) {
        tracing::warn!("{err:#}");
    }
}

fn remove_if_there(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
            Err(err).with_context(|| format!("removing {}", path.display()))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{dev_key_file, real_data_dir};

    #[test]
    fn a_development_build_keeps_no_key_in_the_real_data_folder() {
        let Some(real) = real_data_dir() else { return };
        // Only asked, never written: the check comes before anything touches the folder.
        let err = dev_key_file(&real.join("profiles").join("some-profile")).expect_err("refused");
        assert!(format!("{err:#}").contains("real data folder"), "{err:#}");
    }
}
