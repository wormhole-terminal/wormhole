//! Replacing the running `.app` bundle with a verified download, and relaunching.
//!
//! Everything happens inside a hidden staging dir created *next to* the bundle, so every rename
//! stays on one volume (atomic, no copying) and a crash leaves at most a stray
//! `.Wormhole-update-*` folder, never a half-written bundle at the real path.

use std::fs;
use std::io;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::os::unix::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::TempDir;

use super::archive::extract;
use super::{DownloadedUpdate, UpdateError, UpdaterConfig, parse_lenient_version};

/// Verifies `update` once more at the bundle level and swaps it in place of `app_bundle`.
///
/// On error the original bundle is left (or put back) at `app_bundle`. Takes the config (unlike
/// the original sketch) for the expected bundle id/executable and the codesign settings.
pub fn install(
    config: &UpdaterConfig,
    update: &DownloadedUpdate,
    app_bundle: &Path,
) -> Result<(), UpdateError> {
    let app_bundle = check_bundle_path(app_bundle)?;
    let staging = create_staging(&app_bundle)?;
    check_bundle_removable(&app_bundle, &staging)?;

    let version = &update.release().version;
    tracing::info!(%version, bundle = %app_bundle.display(), "installing update");
    let new_app = extract(update.file(), &staging.path().join("new"))?;
    validate_bundle(&new_app, config, version)?;
    if config.verify_codesign {
        verify_codesign(&new_app, config.team_id.as_deref())?;
    } else {
        tracing::warn!("skipping codesign verification (disabled in config)");
    }

    let backup = staging.path().join("previous.app");
    if let Err(e) = swap_into_place(&app_bundle, &new_app, &backup) {
        if backup.exists() {
            // Rollback failed too: keep the staging dir so the old bundle isn't deleted with it.
            let kept = staging.keep();
            tracing::error!(backup = %kept.display(), "update failed and the previous version could not be restored");
        }
        return Err(e);
    }
    // Dropping `staging` deletes the previous version; failure there only leaves a hidden folder.
    if let Err(e) = staging.close() {
        tracing::warn!("could not remove the previous version: {e}");
    }
    tracing::info!(%version, "update installed");
    Ok(())
}

/// The checks `install` does before touching anything, for the UI to run before downloading:
/// is this a real, writable bundle (not translocated, not on a read-only or root-owned location)?
pub fn preflight(app_bundle: &Path) -> Result<(), UpdateError> {
    let app_bundle = check_bundle_path(app_bundle)?;
    let staging = create_staging(&app_bundle)?;
    check_bundle_removable(&app_bundle, &staging)
}

/// The `.app` containing the running executable, or `None` when not run from a bundle (e.g.
/// `cargo run`), in which case self-update should be hidden.
pub fn current_app_bundle() -> Option<PathBuf> {
    bundle_from_exe(&std::env::current_exe().ok()?)
}

pub(crate) fn bundle_from_exe(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    let is_app = app.extension().is_some_and(|e| e.eq_ignore_ascii_case("app"));
    (macos.file_name()? == "MacOS" && contents.file_name()? == "Contents" && is_app)
        .then(|| app.to_path_buf())
}

/// Gatekeeper runs quarantined apps opened from Downloads or a DMG from a randomized read-only
/// mount (`…/AppTranslocation/<uuid>/d/Wormhole.app`). Replacing that copy would be pointless.
pub fn is_translocated(app_bundle: &Path) -> bool {
    app_bundle.components().any(|c| c.as_os_str() == "AppTranslocation")
}

/// How long the relaunch helper waits for this process to exit before giving up: 150 polls of 0.2 s.
const RELAUNCH_WAIT_POLLS: u32 = 150;
/// The seconds between two of those polls (a `sleep` argument).
const RELAUNCH_POLL_SECS: &str = "0.2";

/// Starts `app_bundle` again once this process has exited, then returns immediately; the caller
/// should quit right after. The helper is double-forked (`( … ) &`) so it's reparented to launchd
/// and survives our exit, and waits for our pid so `open -n` doesn't race the old instance.
///
/// If this process is still there after [`RELAUNCH_WAIT_POLLS`] (hung while quitting), the
/// helper gives up and launches nothing: a second instance next to a half-dead first one would
/// fight it over the hook socket and the database, and the user can open the app themselves.
pub fn relaunch(app_bundle: &Path) -> io::Result<()> {
    spawn_relaunch_helper(
        app_bundle,
        std::process::id(),
        Path::new("/usr/bin/open"),
        RELAUNCH_WAIT_POLLS,
        RELAUNCH_POLL_SECS,
        "1",
    )
}

/// `opener -n app_bundle` once `pid` is gone; `polls`, `poll_secs` and `settle_secs` are parameters
/// so tests can use a fake opener and not wait for real.
fn spawn_relaunch_helper(
    app_bundle: &Path,
    pid: u32,
    opener: &Path,
    polls: u32,
    poll_secs: &str,
    settle_secs: &str,
) -> io::Result<()> {
    // Everything goes in as positional args ($0..$5), never interpolated, so no quoting issues.
    const SCRIPT: &str = r#"(
        i=0
        while kill -0 "$1" 2>/dev/null; do
            [ "$i" -ge "$3" ] && exit 0
            sleep "$4"
            i=$((i+1))
        done
        sleep "$5"
        exec "$2" -n "$0"
    ) </dev/null >/dev/null 2>&1 &"#;
    let status = Command::new("/bin/sh")
        .arg("-c")
        .arg(SCRIPT)
        .arg(app_bundle)
        .arg(pid.to_string())
        .arg(opener)
        .arg(polls.to_string())
        .arg(poll_secs)
        .arg(settle_secs)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // Own process group, so a signal to ours (e.g. Ctrl-C in a dev terminal) doesn't kill it.
        .process_group(0)
        .status()?;
    if status.success() { Ok(()) } else { Err(io::Error::other(format!("relaunch helper failed: {status}"))) }
}

fn check_bundle_path(app_bundle: &Path) -> Result<PathBuf, UpdateError> {
    if is_translocated(app_bundle) {
        return Err(UpdateError::Translocated(app_bundle.to_path_buf()));
    }
    // Resolve symlinks so we replace the real bundle rather than turning a link into a copy.
    let real = fs::canonicalize(app_bundle).map_err(|_| UpdateError::NotBundled)?;
    if is_translocated(&real) {
        return Err(UpdateError::Translocated(real));
    }
    let is_app = real.extension().is_some_and(|e| e.eq_ignore_ascii_case("app"));
    if !is_app || !real.is_dir() || real.parent().is_none() {
        return Err(UpdateError::NotBundled);
    }
    Ok(real)
}

fn create_staging(app_bundle: &Path) -> Result<TempDir, UpdateError> {
    let parent = app_bundle.parent().ok_or(UpdateError::NotBundled)?;
    tempfile::Builder::new()
        .prefix(".Wormhole-update-")
        .tempdir_in(parent)
        .map_err(|e| write_error(e, parent))
}

/// Renaming the bundle only needs a writable parent, but deleting the old version afterwards
/// needs every directory inside it to be ours. A root-owned install (e.g. from a pkg) would
/// otherwise swap fine and then leave the old copy stuck in /Applications, so refuse up front.
/// Our uid is read off the staging dir we just created, which avoids an `unsafe` getuid call.
fn check_bundle_removable(app_bundle: &Path, staging: &TempDir) -> Result<(), UpdateError> {
    let uid = fs::metadata(staging.path())?.uid();
    let mut stack = vec![app_bundle.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let meta = fs::symlink_metadata(&dir)?;
        if meta.uid() != uid || meta.mode() & 0o200 == 0 {
            tracing::warn!(dir = %dir.display(), owner = meta.uid(), "bundle directory not writable by us");
            return Err(UpdateError::NotWritable(app_bundle.to_path_buf()));
        }
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                stack.push(entry.path());
            }
        }
    }
    Ok(())
}

fn write_error(e: io::Error, path: &Path) -> UpdateError {
    match e.kind() {
        io::ErrorKind::PermissionDenied | io::ErrorKind::ReadOnlyFilesystem => {
            UpdateError::NotWritable(path.to_path_buf())
        }
        _ => UpdateError::Io(e),
    }
}

/// The extracted bundle must be the app we expect: right identifier, the executable present, and
/// the version the feed promised (so an old signed archive can't be replayed as a "new" release).
fn validate_bundle(app: &Path, config: &UpdaterConfig, version: &semver::Version) -> Result<(), UpdateError> {
    let bad = |msg: String| UpdateError::Install(format!("downloaded app is invalid: {msg}"));
    if !fs::symlink_metadata(app).is_ok_and(|m| m.is_dir()) {
        return Err(bad(format!("{} is not a directory", app.display())));
    }
    let exe = app.join("Contents/MacOS").join(&config.bundle_executable);
    match fs::symlink_metadata(&exe) {
        Ok(m) if m.is_file() && m.permissions().mode() & 0o111 != 0 => {}
        _ => return Err(bad(format!("missing executable Contents/MacOS/{}", config.bundle_executable))),
    }

    let info = plist::Value::from_file(app.join("Contents/Info.plist"))
        .map_err(|e| bad(format!("unreadable Info.plist: {e}")))?;
    let dict = info.as_dictionary().ok_or_else(|| bad("Info.plist is not a dictionary".into()))?;
    let string = |key: &str| dict.get(key).and_then(plist::Value::as_string);

    let id = string("CFBundleIdentifier").unwrap_or_default();
    if id != config.bundle_identifier && !config.also_accepted.iter().any(|also| also == id) {
        return Err(bad(format!("bundle identifier {id:?}, expected {:?}", config.bundle_identifier)));
    }
    let short = string("CFBundleShortVersionString").unwrap_or_default();
    // Apple wants plain "x.y.z" here, so a prerelease feed version may appear without its suffix.
    let core = semver::Version::new(version.major, version.minor, version.patch);
    let matches = short == version.to_string() || parse_lenient_version(short).is_some_and(|v| v == core);
    if !matches {
        return Err(bad(format!("bundle version {short:?} does not match release {version}")));
    }
    Ok(())
}

fn verify_codesign(app: &Path, team_id: Option<&str>) -> Result<(), UpdateError> {
    let mut cmd = Command::new("/usr/bin/codesign");
    cmd.args(["--verify", "--deep", "--strict"]);
    if let Some(team) = team_id {
        // Interpolated into a requirement string, so restrict to what Apple team IDs look like.
        if team.is_empty() || !team.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(UpdateError::Install(format!("invalid team id {team:?}")));
        }
        cmd.arg("-R").arg(format!("=anchor apple generic and certificate leaf[subject.OU] = \"{team}\""));
    }
    let out = cmd.arg(app).output()?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(UpdateError::Signature(format!("codesign rejected the new app: {}", stderr.trim())));
    }
    Ok(())
}

/// Two renames with rollback. macOS has an atomic exchange (`renamex_np(RENAME_SWAP)`) but it
/// needs FFI; the window between the renames is microseconds and a failure is rolled back.
pub(crate) fn swap_into_place(app_bundle: &Path, new_app: &Path, backup: &Path) -> Result<(), UpdateError> {
    fs::rename(app_bundle, backup).map_err(|e| write_error(e, app_bundle))?;
    let Err(e) = fs::rename(new_app, app_bundle) else { return Ok(()) };
    tracing::error!("moving new version into place failed: {e}; rolling back");
    match fs::rename(backup, app_bundle) {
        Ok(()) => Err(UpdateError::Install(format!(
            "could not move the new version into place ({e}); nothing was changed"
        ))),
        Err(e2) => Err(UpdateError::Install(format!(
            "could not move the new version into place ({e}) nor restore the previous one ({e2}); \
             it is at {}",
            backup.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::{bundle_from_exe, is_translocated, spawn_relaunch_helper};

    #[test]
    fn bundle_detection() {
        let exe = Path::new("/Applications/Wormhole.app/Contents/MacOS/Wormhole");
        assert_eq!(bundle_from_exe(exe), Some(PathBuf::from("/Applications/Wormhole.app")));
        assert_eq!(bundle_from_exe(Path::new("/Users/me/terminal/target/debug/Wormhole")), None);
        assert_eq!(bundle_from_exe(Path::new("/x/Wormhole.app/Contents/Resources/Wormhole")), None);
    }

    #[test]
    fn translocation_detection() {
        let p = Path::new("/private/var/folders/ab/cd/T/AppTranslocation/1234-ABCD/d/Wormhole.app");
        assert!(is_translocated(p));
        assert!(!is_translocated(Path::new("/Applications/Wormhole.app")));
    }

    /// A stand-in for `/usr/bin/open` that records its arguments.
    fn fake_opener(dir: &Path) -> (PathBuf, PathBuf) {
        let marker = dir.join("opened");
        let opener = dir.join("open");
        fs::write(&opener, format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n", marker.display())).unwrap();
        fs::set_permissions(&opener, fs::Permissions::from_mode(0o755)).unwrap();
        (opener, marker)
    }

    fn wait_for(path: &Path, seconds: u64) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
        while std::time::Instant::now() < deadline {
            if path.exists() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        false
    }

    #[test]
    fn relaunch_opens_the_app_once_the_old_process_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        let (opener, marker) = fake_opener(dir.path());
        let mut old = Command::new("sleep").arg("30").spawn().unwrap();
        spawn_relaunch_helper(Path::new("/Applications/Wormhole.app"), old.id(), &opener, 100, "0.02", "0")
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(250));
        assert!(!marker.exists(), "it must wait for the old instance");
        old.kill().unwrap();
        old.wait().unwrap();
        assert!(wait_for(&marker, 10), "the app was never opened");
        assert_eq!(fs::read_to_string(&marker).unwrap(), "-n\n/Applications/Wormhole.app\n");
    }

    #[test]
    fn relaunch_gives_up_instead_of_starting_a_second_instance() {
        let dir = tempfile::tempdir().unwrap();
        let (opener, marker) = fake_opener(dir.path());
        let mut stuck = Command::new("sleep").arg("30").spawn().unwrap();
        // Three polls (60 ms) stand in for the 30 s; the old process never exits.
        spawn_relaunch_helper(Path::new("/Applications/Wormhole.app"), stuck.id(), &opener, 3, "0.02", "0")
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert!(!marker.exists(), "it opened a second instance next to a running one");
        stuck.kill().unwrap();
        stuck.wait().unwrap();
        // ... and having given up, it does not start the app later either.
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(!marker.exists());
    }
}
