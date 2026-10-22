//! Where the session host's executable is (`wormhole-host`, crates/wormhole-host), or that there is none
//! to use, in which case the app keeps its terminals in-process.
//!
//! The host sits next to the app's own executable: `Contents/MacOS/` in the bundle, and
//! `target/<profile>/` for `cargo build` and `cargo run`. Nothing is searched for beyond that, and
//! nothing is looked up by `PATH`: a program found elsewhere would be one the app never shipped.

use std::ffi::{OsStr, OsString};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

/// The host's file name, next to the app's executable (`HOST_BINARY` in packaging/app.env).
pub const HOST_BINARY_NAME: &str = "wormhole-host";

/// Names another host executable in a debug build (a test instance, a build of the host kept
/// elsewhere). A release build ignores it: it would let the environment pick which program holds
/// every terminal.
pub const HOST_BIN_VAR: &str = "WORMHOLE_HOST_BIN";

/// The host executable this app should start, or `None` for in-process terminals: there is none beside
/// the app, or the app runs where hosts must not start (see [`runs_from_a_disposable_copy`]).
pub fn host_binary() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let over = cfg!(debug_assertions).then(|| std::env::var_os(HOST_BIN_VAR)).flatten();
    host_binary_for(&exe, over.as_deref())
}

/// [`host_binary`] for a given app executable and override, so the rules can be tested.
fn host_binary_for(exe: &Path, over: Option<&OsStr>) -> Option<PathBuf> {
    if let Some(over) = over.filter(|p| !p.is_empty()) {
        let path = PathBuf::from(OsString::from(over));
        if is_executable_file(&path) {
            return Some(path);
        }
        tracing::warn!(path = %path.display(), "{HOST_BIN_VAR} is not an executable file; no session host");
        return None;
    }
    if runs_from_a_disposable_copy(exe) {
        return None;
    }
    let path = exe.parent()?.join(HOST_BINARY_NAME);
    is_executable_file(&path).then_some(path)
}

/// A host outlives the app and keeps its executable open, so it must not start from a copy that macOS
/// or the user is about to take away: Gatekeeper's translocated copy (`…/AppTranslocation/…`) and a
/// mounted disk image (`/Volumes/…`, which could then not be ejected). A bundle on another volume is
/// refused too, which only costs it persistence. A run that is not from a bundle (`cargo run`) is
/// never refused: its folder is the build's.
fn runs_from_a_disposable_copy(exe: &Path) -> bool {
    let Some(app) = crate::update::bundle_from_exe(exe) else { return false };
    crate::update::is_translocated(&app) || app.starts_with("/Volumes")
}

fn is_executable_file(path: &Path) -> bool {
    // Not `exists()`: a directory or a file without the execute bit would fail at spawn instead.
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::Path;

    use super::{HOST_BINARY_NAME, host_binary_for, runs_from_a_disposable_copy};

    fn touch(path: &Path, mode: u32) {
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, b"#!/bin/sh\n").expect("write");
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("chmod");
    }

    #[test]
    fn the_host_is_found_beside_the_app_in_a_bundle_and_in_a_build_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        let macos = dir.path().join("Wormhole.app/Contents/MacOS");
        touch(&macos.join("Wormhole"), 0o755);
        assert_eq!(host_binary_for(&macos.join("Wormhole"), None), None, "no host built yet");
        touch(&macos.join(HOST_BINARY_NAME), 0o755);
        assert_eq!(host_binary_for(&macos.join("Wormhole"), None), Some(macos.join(HOST_BINARY_NAME)));

        let debug = dir.path().join("target/debug");
        touch(&debug.join("Wormhole"), 0o755);
        touch(&debug.join(HOST_BINARY_NAME), 0o755);
        assert_eq!(host_binary_for(&debug.join("Wormhole"), None), Some(debug.join(HOST_BINARY_NAME)));
    }

    #[test]
    fn a_host_that_cannot_run_is_not_one() {
        let dir = tempfile::tempdir().expect("tempdir");
        let exe = dir.path().join("Wormhole");
        touch(&dir.path().join(HOST_BINARY_NAME), 0o644);
        assert_eq!(host_binary_for(&exe, None), None, "no execute bit");
        fs::remove_file(dir.path().join(HOST_BINARY_NAME)).expect("remove");
        fs::create_dir(dir.path().join(HOST_BINARY_NAME)).expect("mkdir");
        assert_eq!(host_binary_for(&exe, None), None, "a folder");
    }

    #[test]
    fn a_translocated_or_mounted_app_gets_no_host_even_with_one_beside_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let translocated = dir.path().join("AppTranslocation/ABC/d/Wormhole.app/Contents/MacOS");
        touch(&translocated.join(HOST_BINARY_NAME), 0o755);
        assert_eq!(host_binary_for(&translocated.join("Wormhole"), None), None);

        assert!(runs_from_a_disposable_copy(Path::new(
            "/Volumes/Wormhole/Wormhole.app/Contents/MacOS/Wormhole"
        )));
        assert!(!runs_from_a_disposable_copy(Path::new(
            "/Applications/Wormhole.app/Contents/MacOS/Wormhole"
        )));
        assert!(!runs_from_a_disposable_copy(Path::new("/Volumes/Dev/repo/target/debug/Wormhole")));
    }

    #[test]
    fn the_override_names_the_host_and_wins_over_the_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        let other = dir.path().join("elsewhere/my-host");
        touch(&other, 0o755);
        let exe = dir.path().join("app/Wormhole");
        let beside = dir.path().join("app").join(HOST_BINARY_NAME);
        touch(&beside, 0o755);
        assert_eq!(host_binary_for(&exe, Some(other.as_os_str())), Some(other.clone()));
        // A wrong override is an error to show, not a reason to quietly use the other one.
        assert_eq!(host_binary_for(&exe, Some(OsStr::new("/nonexistent/host"))), None);
        // Empty means unset.
        assert_eq!(host_binary_for(&exe, Some(OsStr::new(""))), Some(beside));
    }
}
