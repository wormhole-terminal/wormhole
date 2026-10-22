//! Where the key the saved text is sealed with lives.
//!
//! One key per profile (per database folder): the store is per profile, so the key sits with the
//! file it protects, profiles share nothing, and a development build's key file is next to the file
//! it opens and nowhere else.
//!
//! - A release build keeps it in the login Keychain, as a generic password named after the profile's
//!   folder, made the first time a text is saved or read. The app that made it reads it without a
//!   prompt; another program running as the user gets one.
//! - A development build (debug assertions: `cargo run`, `scripts/dev.sh`, devctl instances, tests)
//!   never touches the Keychain: a rebuilt binary would ask for it again after every build, and an
//!   isolated instance must not reach the user's. It keeps the key in a file beside the text,
//!   [`DEV_KEY_FILE`], which protects nothing and says so in its name.

use std::path::Path;

use anyhow::{Context as _, Result, bail};

use super::cipher::{KEY_LEN, Key, new_key};

/// The development build's key file, in the profile's folder. For testing only: a key next to the
/// text it seals keeps nothing from anyone who can read the folder.
pub(crate) const DEV_KEY_FILE: &str = "scrollback-key.dev-only";

/// The Keychain item's service; its account is the profile's folder.
#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "Wormhole saved terminal text";

/// `errSecItemNotFound`: no key yet.
#[cfg(target_os = "macos")]
const ITEM_NOT_FOUND: i32 = -25300;

/// The key of the profile in `dir`, from the login Keychain, made there on first use.
#[cfg(target_os = "macos")]
pub(super) fn keychain(dir: &Path) -> Result<Key> {
    use security_framework::passwords::{
        PasswordOptions, get_generic_password, set_generic_password_options,
    };
    use zeroize::Zeroizing;

    let account = dir.to_str().with_context(|| format!("{} is not UTF-8", dir.display()))?;
    match get_generic_password(KEYCHAIN_SERVICE, account) {
        Ok(found) => {
            let found = Zeroizing::new(found);
            let mut key = Key::new([0; KEY_LEN]);
            if found.len() != KEY_LEN {
                bail!("the Keychain's saved text key has {} bytes, not {KEY_LEN}", found.len());
            }
            key.copy_from_slice(&found);
            Ok(key)
        }
        Err(err) if err.code() == ITEM_NOT_FOUND => {
            let key = new_key()?;
            let mut options = PasswordOptions::new_generic_password(KEYCHAIN_SERVICE, account);
            options.set_label(KEYCHAIN_SERVICE);
            set_generic_password_options(key.as_ref(), options)
                .map_err(|err| anyhow::anyhow!("{err}"))
                .context("keeping a new saved text key in the Keychain")?;
            tracing::info!("made a saved text key in the Keychain");
            Ok(key)
        }
        // Denied, cancelled, a locked Keychain: the text isn't kept this time.
        Err(err) => bail!("reading the saved text key from the Keychain: {err}"),
    }
}

#[cfg(not(target_os = "macos"))]
pub(super) fn keychain(_: &Path) -> Result<Key> {
    bail!("no Keychain on this system")
}

/// The development build's key, in [`DEV_KEY_FILE`] in `dir` (owner-only), made on first use.
pub(super) fn dev_key_file(dir: &Path) -> Result<Key> {
    let path = dir.join(DEV_KEY_FILE);
    match crate::fsutil::read_capped(&path, 64, crate::fsutil::LinkPolicy::Follow) {
        Ok(found) => {
            let found = zeroize::Zeroizing::new(found);
            if found.len() != KEY_LEN {
                bail!("{} has {} bytes, not {KEY_LEN}", path.display(), found.len());
            }
            let mut key = Key::new([0; KEY_LEN]);
            key.copy_from_slice(&found);
            Ok(key)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let key = new_key()?;
            crate::fsutil::write_atomic(&path, key.as_ref(), crate::fsutil::LinkPolicy::Follow)
                .with_context(|| format!("writing {}", path.display()))?;
            Ok(key)
        }
        Err(err) => Err(err).with_context(|| format!("reading {}", path.display())),
    }
}
