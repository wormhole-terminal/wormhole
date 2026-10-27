//! A secret key an area of the app keeps for itself, and where it lives.
//!
//! The saved terminal text (`store::saved_text`) seals its records with one, the Probes' approvals
//! (`probe::approval`) with another. What differs between them is a [`Purpose`] (the Keychain item's
//! service, the development key file's name, the words of an error); where the key is kept, and how,
//! is the same, and is written once here:
//!
//! - A release build keeps it in the login Keychain, as a generic password, under an account the
//!   caller chooses (a random id, see [`random_id`]: never a path, which would name the user). The app
//!   that made it reads it without a prompt; another program running as the user gets one.
//! - A development build (debug assertions, or the `dev-key` feature that devctl builds turn on:
//!   `cargo run`, `scripts/dev.sh`, devctl instances, release-profile dev builds) and every test build
//!   (`cargo test --release` too) never touches the Keychain: a rebuilt binary would ask for it again
//!   after every build, and an isolated instance or a test must not reach the user's. It keeps the key
//!   in a file beside what it protects (one per account, in a folder, when [`Purpose::dev_key_per_account`]),
//!   which protects nothing and says so in its name, and never in the real data folder; a release
//!   build deletes such a file it finds.
//!
//! Reading a key may block, or show the system's prompt (a locked Keychain): call these off the main
//! thread.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow, bail};
use ring::rand::{SecureRandom, SystemRandom};
use zeroize::Zeroizing;

use crate::platform::keychain;
use crate::platform::paths;

/// The key's length: 256 bits.
pub(crate) const KEY_LEN: usize = 32;

/// A secret key. Wiped when dropped; the copy the Keychain call makes (`CFData`) and any expanded key
/// a cipher or MAC builds from it are not: both live only in this process's memory.
pub(crate) type Key = Zeroizing<[u8; KEY_LEN]>;

/// What one kind of key is called and where it is kept.
pub(crate) struct Purpose {
    /// The words of its errors and log lines, `saved text key`.
    pub(crate) what: &'static str,
    /// What a development build refuses to keep in the real data folder, for the same error.
    pub(crate) keeps: &'static str,
    /// The Keychain item's service.
    pub(crate) keychain_service: &'static str,
    /// The development build's key file, in the folder of what the key protects. For testing only.
    pub(crate) dev_key_file: &'static str,
    /// Whether each account has a key of its own in a development build too: then `dev_key_file` is a
    /// folder with one file per account, so deleting one account's key leaves the others.
    pub(crate) dev_key_per_account: bool,
}

/// A development build was asked for a key in the real data folder, where it keeps none: not the
/// Keychain's doing, and set right by `WORMHOLE_DATA_DIR`.
#[derive(Debug)]
pub(crate) struct InRealDataFolder {
    keeps: &'static str,
}

impl std::fmt::Display for InRealDataFolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a development build keeps no {} in the real data folder (set WORMHOLE_DATA_DIR)",
            self.keeps
        )
    }
}

impl std::error::Error for InRealDataFolder {}

/// A fresh random key.
pub(crate) fn new_key() -> Result<Key> {
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    SystemRandom::new().fill(key.as_mut()).map_err(|_| anyhow!("the system has no random numbers"))?;
    Ok(key)
}

/// 128 random bits as hex: a key's account, which says nothing about the user or the folder.
pub(crate) fn random_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    SystemRandom::new().fill(&mut bytes).map_err(|_| anyhow!("the system has no random numbers"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Whether `text` has the shape [`random_id`] makes: what a stored account must look like before it
/// is used as a Keychain account.
pub(crate) fn is_random_id(text: &str) -> bool {
    text.len() == 32 && text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Whether this build keeps its keys in a file instead of the Keychain: a debug build, one with the
/// `dev-key` feature (devctl builds, release-profile dev builds), or a test build, which must never
/// reach the user's Keychain even when built with `--release`.
pub(crate) fn uses_dev_key() -> bool {
    cfg!(debug_assertions) || cfg!(feature = "dev-key") || cfg!(test)
}

/// The key of `purpose` kept under `account` (Keychain) or in `dir` (development build), made on
/// first use.
pub(crate) fn open(purpose: &Purpose, dir: &Path, account: &str) -> Result<Key> {
    if uses_dev_key() { dev_key_file(purpose, dir, account) } else { keychain_key(purpose, account) }
}

/// The same key, but never made: `None` when there is none. For checking something against it,
/// where a key that isn't there yet means there is nothing to check, and a read must not leave items
/// behind in the user's Keychain for any account a file names.
pub(crate) fn find(purpose: &Purpose, dir: &Path, account: &str) -> Result<Option<Key>> {
    if uses_dev_key() { find_dev_key_file(purpose, dir, account) } else { find_keychain(purpose, account) }
}

/// Throw the key away.
pub(crate) fn forget(purpose: &Purpose, dir: &Path, account: &str) -> Result<()> {
    if uses_dev_key() {
        forget_dev_key_file(purpose, dir, account)
    } else {
        forget_keychain(purpose, account)
    }
}

/// The key kept under `account` in the login Keychain, made there on first use.
fn keychain_key(purpose: &Purpose, account: &str) -> Result<Key> {
    match find_keychain(purpose, account) {
        Ok(Some(key)) => Ok(key),
        Ok(None) => {
            let key = new_key()?;
            match keychain::add(purpose.keychain_service, account, key.as_ref()) {
                Ok(()) => {
                    tracing::info!("made a {} in the Keychain", purpose.what);
                    Ok(key)
                }
                // Made by someone else in between: theirs is the one, never overwritten.
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    find_keychain(purpose, account)?
                        .with_context(|| format!("the Keychain has a {} that can't be read", purpose.what))
                }
                Err(err) => bail!("keeping a new {} in the Keychain: {err}", purpose.what),
            }
        }
        Err(err) => Err(err),
    }
}

fn find_keychain(purpose: &Purpose, account: &str) -> Result<Option<Key>> {
    match keychain::find(purpose.keychain_service, account) {
        Ok(Some(found)) => to_key(found.as_slice(), &format!("the Keychain's {}", purpose.what)).map(Some),
        Ok(None) => Ok(None),
        // Denied, cancelled, a locked Keychain.
        Err(err) => bail!("reading the {} from the Keychain: {err}", purpose.what),
    }
}

fn forget_keychain(purpose: &Purpose, account: &str) -> Result<()> {
    keychain::delete(purpose.keychain_service, account)
        .map_err(|err| anyhow!("deleting the {}: {err}", purpose.what))
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

/// Where a development build keeps the key of `purpose` and `account` in `dir`.
fn dev_key_path(purpose: &Purpose, dir: &Path, account: &str) -> Result<PathBuf> {
    if !purpose.dev_key_per_account {
        return Ok(dir.join(purpose.dev_key_file));
    }
    // The account becomes a file name: only the shape `random_id` makes is let through.
    if !is_random_id(account) {
        bail!("{account:?} is not an account a {} is kept under", purpose.what);
    }
    Ok(dir.join(purpose.dev_key_file).join(account))
}

/// The development build's key file of `purpose` in `dir` (owner-only), if it is there. None is
/// kept in the real data folder, where a key beside what it protects would leave the user's data open.
fn find_dev_key_file(purpose: &Purpose, dir: &Path, account: &str) -> Result<Option<Key>> {
    refuse_real_data_dir(purpose, dir)?;
    let path = dev_key_path(purpose, dir, account)?;
    match crate::storage::files::read_capped(&path, 64, crate::storage::files::LinkPolicy::Follow) {
        Ok(found) => to_key(Zeroizing::new(found).as_slice(), &path.display().to_string()).map(Some),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("reading {}", path.display())),
    }
}

/// [`find_dev_key_file`], made on first use.
fn dev_key_file(purpose: &Purpose, dir: &Path, account: &str) -> Result<Key> {
    if let Some(key) = find_dev_key_file(purpose, dir, account)? {
        return Ok(key);
    }
    let path = dev_key_path(purpose, dir, account)?;
    if purpose.dev_key_per_account {
        let folder = dir.join(purpose.dev_key_file);
        crate::storage::files::create_dir_all_private(&folder)
            .with_context(|| format!("making {}", folder.display()))?;
    }
    let key = new_key()?;
    crate::storage::files::write_atomic(&path, key.as_ref(), crate::storage::files::LinkPolicy::Follow)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(key)
}

fn refuse_real_data_dir(purpose: &Purpose, dir: &Path) -> Result<()> {
    if in_real_data_dir(dir) {
        return Err(InRealDataFolder { keeps: purpose.keeps }.into());
    }
    Ok(())
}

fn forget_dev_key_file(purpose: &Purpose, dir: &Path, account: &str) -> Result<()> {
    remove_if_there(&dev_key_path(purpose, dir, account)?)
}

/// A release build removes a development key file it finds beside what it protects (one a
/// development build left there): it protects nothing.
pub(crate) fn remove_stray_dev_key(purpose: &Purpose, dir: &Path) {
    let path = dir.join(purpose.dev_key_file);
    let removed = if purpose.dev_key_per_account {
        // `remove_dir_all` removes a link planted at the name, never what it points to.
        match std::fs::remove_dir_all(&path) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                Err(err).with_context(|| format!("removing {}", path.display()))
            }
            _ => Ok(()),
        }
    } else {
        remove_if_there(&path)
    };
    if let Err(err) = removed {
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
    use super::{
        InRealDataFolder, Purpose, dev_key_file, find, forget, is_random_id, random_id, real_data_dir,
        remove_stray_dev_key, uses_dev_key,
    };

    const PURPOSE: Purpose = Purpose {
        what: "test key",
        keeps: "test data",
        keychain_service: "Wormhole test key (never made)",
        dev_key_file: "test-key.dev-only",
        dev_key_per_account: false,
    };

    const PER_ACCOUNT: Purpose =
        Purpose { dev_key_file: "test-keys.dev-only", dev_key_per_account: true, ..PURPOSE };

    #[test]
    fn a_test_build_never_reaches_the_keychain() {
        assert!(uses_dev_key(), "not even one built with --release");
    }

    #[test]
    fn a_development_build_keeps_no_key_in_the_real_data_folder() {
        let Some(real) = real_data_dir() else { return };
        // Only asked, never written: the check comes before anything touches the folder.
        let err = dev_key_file(&PURPOSE, &real.join("profiles").join("some-profile"), "unused")
            .expect_err("refused");
        assert!(err.downcast_ref::<InRealDataFolder>().is_some(), "{err:#}");
        assert!(format!("{err:#}").contains("real data folder"), "{err:#}");
    }

    #[test]
    fn a_key_per_account_is_its_own_file_and_forgetting_one_leaves_the_others() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (one, two) = (random_id().expect("id"), random_id().expect("id"));
        let first = dev_key_file(&PER_ACCOUNT, dir.path(), &one).expect("made");
        let second = dev_key_file(&PER_ACCOUNT, dir.path(), &two).expect("made");
        assert_ne!(*first, *second);
        assert_eq!(find(&PER_ACCOUNT, dir.path(), &one).expect("read").as_deref(), Some(&*first));
        forget(&PER_ACCOUNT, dir.path(), &one).expect("forgotten");
        assert!(find(&PER_ACCOUNT, dir.path(), &one).expect("read").is_none());
        assert_eq!(find(&PER_ACCOUNT, dir.path(), &two).expect("read").as_deref(), Some(&*second));
        // An account is a file name only in the shape `random_id` makes.
        assert!(find(&PER_ACCOUNT, dir.path(), "../escape").is_err());
        remove_stray_dev_key(&PER_ACCOUNT, dir.path());
        assert!(!dir.path().join(PER_ACCOUNT.dev_key_file).exists());
    }

    #[test]
    fn a_random_id_has_the_shape_it_is_checked_by() {
        let id = random_id().expect("an id");
        assert!(is_random_id(&id), "{id}");
        assert!(!is_random_id(""));
        assert!(!is_random_id(&id.to_uppercase()));
        assert!(!is_random_id("../../etc/passwd"));
        assert!(!is_random_id(&id[1..]));
    }
}
