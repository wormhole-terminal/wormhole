//! Self-update: check the release feed, download and verify an archive, swap the `.app` bundle.
//!
//! Everything here is blocking; the app calls it from a background thread. The flow is
//! [`check`] → [`download`] → [`install`] → [`relaunch`], with [`UpdateSchedule`] deciding when to
//! check and which version the user skipped.
//!
//! Trust model: the feed (`<base>/stable/latest.json`) is *not* signed, so nothing in it is
//! trusted beyond "where to look". The archive's minisign signature, checked against the public
//! key compiled into the app, is the authority; the feed's sha256 is only an early integrity check.
//! Because an attacker who controls the feed could pair an old (validly signed) archive with a new
//! version number, [`install`] also checks the version inside the extracted bundle's Info.plist.

mod archive;
mod download;
mod feed;
mod http;
mod install;
mod schedule;
mod security;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use std::fmt;
use std::path::{Path, PathBuf};

pub use download::{DownloadedUpdate, download};
pub use feed::{Asset, Release, UpdateStatus, check};
pub use install::{current_app_bundle, install, is_translocated, preflight, relaunch};
pub use schedule::UpdateSchedule;
pub use security::{MAX_ADVISORIES, Security, Severity};

/// Releases larger than this are refused before and during download.
pub const DEFAULT_MAX_DOWNLOAD_SIZE: u64 = 500 * 1024 * 1024;

/// The channel path below the configurable base URL; CI publishes to exactly this key.
pub const FEED_PATH: &str = "stable/latest.json";

#[derive(Debug, Clone)]
pub struct UpdaterConfig {
    /// Full feed URL, normally `<base>/stable/latest.json` (see [`UpdaterConfig::new`]).
    pub feed_url: String,
    /// minisign public key: either the whole `.pub` file text or just its base64 key line.
    pub public_key: String,
    pub current_version: semver::Version,
    /// Feed platform key such as `darwin-aarch64`; see [`current_platform`].
    pub platform: String,
    /// The running macOS version; `None` skips the `minimum_system_version` check.
    pub system_version: Option<semver::Version>,
    /// The extracted bundle's `CFBundleIdentifier` must equal this ...
    pub bundle_identifier: String,
    /// ... or one of these: the identifier the app moves to when it is renamed, so a version from
    /// before the move still installs the one after it. The signature and the team check apply
    /// to every update all the same.
    pub also_accepted: Vec<String>,
    /// The extracted bundle must contain `Contents/MacOS/<bundle_executable>`.
    pub bundle_executable: String,
    /// Run `codesign --verify --deep --strict` on the new bundle. Off only for dev/tests, where
    /// bundles are unsigned.
    pub verify_codesign: bool,
    /// When set, codesign must also confirm the new bundle is signed by this Apple team, so a
    /// validly-signed-by-someone-else app can't be swapped in even if minisign were bypassed.
    pub team_id: Option<String>,
    pub max_download_size: u64,
}

impl UpdaterConfig {
    /// Config for the running machine with production defaults (codesign verification on).
    pub fn new(
        base_url: &str,
        public_key: impl Into<String>,
        current_version: semver::Version,
        bundle_identifier: impl Into<String>,
    ) -> Self {
        Self {
            feed_url: feed_url(base_url),
            public_key: public_key.into(),
            current_version,
            platform: current_platform(),
            system_version: macos_version(),
            bundle_identifier: bundle_identifier.into(),
            also_accepted: Vec::new(),
            bundle_executable: "Wormhole".into(),
            verify_codesign: true,
            team_id: None,
            max_download_size: DEFAULT_MAX_DOWNLOAD_SIZE,
        }
    }
}

pub fn feed_url(base_url: &str) -> String {
    format!("{}/{FEED_PATH}", base_url.trim_end_matches('/'))
}

/// The feed's platform key for this build, e.g. `darwin-aarch64` or `darwin-x86_64`.
pub fn current_platform() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    format!("{os}-{}", std::env::consts::ARCH)
}

/// Where macOS keeps its own version. `sw_vers` is a front end for this file.
const SYSTEM_VERSION_PLIST: &str = "/System/Library/CoreServices/SystemVersion.plist";

/// The running macOS version, or `None` elsewhere or if it can't be determined.
///
/// Read from the system's version file, not by running `sw_vers`: this is called while the app
/// starts, on the UI thread, and a process spawn (fork, exec, dyld) costs milliseconds there for
/// one short string. `sw_vers` remains the fallback if the file can't be read.
pub fn macos_version() -> Option<semver::Version> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    version_from_system_plist(Path::new(SYSTEM_VERSION_PLIST)).or_else(version_from_sw_vers)
}

fn version_from_system_plist(path: &Path) -> Option<semver::Version> {
    let value = plist::Value::from_file(path).ok()?;
    parse_lenient_version(value.as_dictionary()?.get("ProductVersion")?.as_string()?)
}

fn version_from_sw_vers() -> Option<semver::Version> {
    let out = std::process::Command::new("/usr/bin/sw_vers").arg("-productVersion").output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_lenient_version(String::from_utf8_lossy(&out.stdout).trim())
}

/// Parses Apple-style versions ("13", "14.5", "15.0.1") that strict semver rejects.
pub(crate) fn parse_lenient_version(s: &str) -> Option<semver::Version> {
    let s = s.trim();
    if let Ok(v) = semver::Version::parse(s) {
        return Some(v);
    }
    let mut parts = s.split('.');
    let mut next = || -> Option<u64> { parts.next().map_or(Some(0), |p| p.parse().ok()) };
    let (major, minor, patch) = (next()?, next()?, next()?);
    if parts.next().is_some() {
        return None;
    }
    Some(semver::Version::new(major, minor, patch))
}

#[derive(Debug)]
pub enum UpdateError {
    /// Transport failure or unexpected HTTP status; worth retrying later.
    Network(String),
    /// The feed is malformed, not HTTPS, or has no build for this platform.
    Feed(String),
    /// The archive's minisign signature is missing, malformed or doesn't match the embedded key.
    Signature(String),
    Checksum {
        expected: String,
        actual: String,
    },
    /// Declared or actual download size exceeds the limit.
    TooLarge {
        limit: u64,
    },
    /// Not running from a `.app` bundle (e.g. `cargo run`), so there is nothing to replace.
    NotBundled,
    /// The bundle or its folder can't be modified by this user (e.g. a root-owned install in
    /// /Applications, or a read-only disk image).
    NotWritable(PathBuf),
    /// Gatekeeper App Translocation: the app runs from a randomized read-only mount because it
    /// was opened straight from Downloads/a DMG. Updating it would be lost on the next launch.
    Translocated(PathBuf),
    Install(String),
    Io(std::io::Error),
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(e) => write!(f, "network error: {e}"),
            Self::Feed(e) => write!(f, "invalid update feed: {e}"),
            Self::Signature(e) => write!(f, "update signature check failed: {e}"),
            Self::Checksum { expected, actual } => {
                write!(f, "update checksum mismatch (expected {expected}, got {actual})")
            }
            Self::TooLarge { limit } => write!(f, "update is larger than the {} limit", binary_size(*limit)),
            Self::NotBundled => {
                write!(f, "Wormhole is not running from an app bundle, so it can't update itself")
            }
            Self::NotWritable(p) => write!(
                f,
                "can't update {}: you don't have permission to change it. Reinstall Wormhole by \
                 dragging the new version into Applications, or ask an administrator",
                p.display()
            ),
            Self::Translocated(_) => write!(
                f,
                "Wormhole is running from a temporary location (macOS App Translocation). Move \
                 Wormhole.app to your Applications folder, open it from there, and try again"
            ),
            Self::Install(e) => write!(f, "installing the update failed: {e}"),
            Self::Io(e) => write!(f, "I/O error during update: {e}"),
        }
    }
}

/// "500 MiB": the limits are binary (`500 * 1024 * 1024`), so they are shown that way, not as the
/// 524 MB that dividing by a million gave.
fn binary_size(bytes: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    if bytes >= MIB { format!("{} MiB", bytes / MIB) } else { format!("{bytes} bytes") }
}

impl std::error::Error for UpdateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for UpdateError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Only HTTPS, except plain HTTP to loopback so tests can use a local server. Loopback is safe to
/// allow in release builds too: reaching it requires already running code on this machine, and
/// the signature check still applies.
pub(crate) fn check_url(url: &str) -> Result<(), UpdateError> {
    let lower = url.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("https://") {
        return if rest.is_empty() { Err(UpdateError::Feed(format!("invalid URL: {url}"))) } else { Ok(()) };
    }
    if let Some(rest) = lower.strip_prefix("http://") {
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        // Userinfo would let "http://localhost@evil.example/" masquerade as loopback.
        let host = if authority.contains('@') {
            None
        } else if authority.starts_with("[::1]") {
            Some("[::1]")
        } else {
            authority.split(':').next()
        };
        if matches!(host, Some("127.0.0.1" | "localhost" | "[::1]")) {
            return Ok(());
        }
    }
    Err(UpdateError::Feed(format!("refusing non-HTTPS URL: {url}")))
}

#[cfg(test)]
mod unit_tests {
    use super::{
        DEFAULT_MAX_DOWNLOAD_SIZE, UpdateError, check_url, feed_url, parse_lenient_version,
        version_from_system_plist,
    };
    use semver::Version;

    #[test]
    fn url_policy() {
        assert!(check_url("https://updates.example.com/stable/latest.json").is_ok());
        assert!(check_url("HTTPS://updates.example.com/x").is_ok());
        assert!(check_url("http://127.0.0.1:8080/x").is_ok());
        assert!(check_url("http://localhost/x").is_ok());
        assert!(check_url("http://[::1]:9/x").is_ok());
        assert!(check_url("http://updates.example.com/x").is_err());
        assert!(check_url("http://localhost@evil.example/x").is_err());
        assert!(check_url("http://localhost.evil.example/x").is_err());
        assert!(check_url("file:///etc/passwd").is_err());
        assert!(check_url("ftp://example.com").is_err());
    }

    #[test]
    fn the_size_limit_is_shown_in_the_units_it_is_set_in() {
        let shown = |limit| UpdateError::TooLarge { limit }.to_string();
        assert_eq!(shown(DEFAULT_MAX_DOWNLOAD_SIZE), "update is larger than the 500 MiB limit");
        assert_eq!(shown(1024 * 1024), "update is larger than the 1 MiB limit");
        assert_eq!(shown(100), "update is larger than the 100 bytes limit");
    }

    #[test]
    fn the_system_version_file_is_read_without_a_process() {
        let dir = tempfile::tempdir().unwrap();
        let plist = |body: &str| {
            let path = dir.path().join("SystemVersion.plist");
            std::fs::write(
                &path,
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>{body}</dict></plist>"#
                ),
            )
            .unwrap();
            path
        };
        let read = |body: &str| version_from_system_plist(&plist(body));
        let real =
            "<key>ProductName</key><string>macOS</string><key>ProductVersion</key><string>14.5</string>";
        assert_eq!(read(real), Some(Version::new(14, 5, 0)));
        assert_eq!(read("<key>ProductVersion</key><string>15.0.1</string>"), Some(Version::new(15, 0, 1)));
        assert_eq!(read("<key>ProductName</key><string>macOS</string>"), None);
        assert_eq!(read("<key>ProductVersion</key><string>soon</string>"), None);
        assert_eq!(version_from_system_plist(&dir.path().join("missing.plist")), None);
    }

    #[test]
    fn feed_url_joins_base() {
        assert_eq!(feed_url("https://u.example.com/"), "https://u.example.com/stable/latest.json");
        assert_eq!(feed_url("https://u.example.com"), "https://u.example.com/stable/latest.json");
    }

    #[test]
    fn lenient_versions() {
        assert_eq!(parse_lenient_version("13"), Some(Version::new(13, 0, 0)));
        assert_eq!(parse_lenient_version("14.5"), Some(Version::new(14, 5, 0)));
        assert_eq!(parse_lenient_version("15.0.1"), Some(Version::new(15, 0, 1)));
        assert_eq!(parse_lenient_version("1.2.3-beta.1"), Some(Version::parse("1.2.3-beta.1").unwrap()));
        assert_eq!(parse_lenient_version("1.2.3.4"), None);
        assert_eq!(parse_lenient_version("abc"), None);
    }
}
