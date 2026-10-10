//! The release feed (`latest.json`) and the "is there something newer for me?" decision.

use std::collections::HashMap;
use std::io::Read as _;

use semver::Version;
use serde::Deserialize;

use super::{UpdateError, UpdaterConfig, check_url, http, parse_lenient_version};

/// The feed is a few KB; anything bigger is not our feed.
const MAX_FEED_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    /// Markdown.
    pub notes: String,
    /// RFC 3339 as published; only displayed, so kept as text.
    pub pub_date: Option<String>,
    pub minimum_system_version: Option<Version>,
    /// The build for [`UpdaterConfig::platform`].
    pub asset: Asset,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Asset {
    pub url: String,
    /// Lowercase hex.
    pub sha256: String,
    /// Full `.minisig` text.
    pub signature: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    UpToDate,
    Available(Release),
    /// A newer version exists but needs a newer macOS than this Mac runs.
    Unsupported {
        version: Version,
        minimum_system_version: Version,
    },
}

#[derive(Deserialize)]
struct Feed {
    version: String,
    #[serde(default)]
    pub_date: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    minimum_system_version: Option<String>,
    platforms: HashMap<String, Asset>,
}

/// Fetches the feed and decides. Network errors are `Network` (retry later); anything the server
/// sent that we won't accept is `Feed`.
pub fn check(config: &UpdaterConfig) -> Result<UpdateStatus, UpdateError> {
    check_url(&config.feed_url)?;
    tracing::debug!(url = %config.feed_url, "checking for updates");
    let response = http::get(&config.feed_url, http::FEED_TIMEOUT)?;
    let mut body = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(MAX_FEED_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|e| UpdateError::Network(format!("reading feed: {e}")))?;
    if body.len() as u64 > MAX_FEED_BYTES {
        return Err(UpdateError::Feed("feed is too large".into()));
    }
    let status = evaluate(&body, config)?;
    tracing::info!(?status, current = %config.current_version, "update check finished");
    Ok(status)
}

/// The pure part of [`check`]: parse the feed and compare against the config.
pub(crate) fn evaluate(json: &[u8], config: &UpdaterConfig) -> Result<UpdateStatus, UpdateError> {
    let feed: Feed = serde_json::from_slice(json).map_err(|e| UpdateError::Feed(e.to_string()))?;
    let version = Version::parse(feed.version.trim())
        .map_err(|e| UpdateError::Feed(format!("invalid version {:?}: {e}", feed.version)))?;

    if !is_newer(&version, &config.current_version) {
        return Ok(UpdateStatus::UpToDate);
    }

    let minimum_system_version = match feed.minimum_system_version.as_deref() {
        Some(s) => Some(
            parse_lenient_version(s)
                .ok_or_else(|| UpdateError::Feed(format!("invalid minimum_system_version {s:?}")))?,
        ),
        None => None,
    };
    if let (Some(min), Some(system)) = (&minimum_system_version, &config.system_version)
        && system.cmp_precedence(min).is_lt()
    {
        return Ok(UpdateStatus::Unsupported { version, minimum_system_version: min.clone() });
    }

    let mut platforms = feed.platforms;
    let asset = platforms.remove(&config.platform).ok_or_else(|| {
        UpdateError::Feed(format!("release {version} has no build for {}", config.platform))
    })?;
    validate_asset(&asset, config)?;

    Ok(UpdateStatus::Available(Release {
        version,
        notes: feed.notes.unwrap_or_default(),
        pub_date: feed.pub_date,
        minimum_system_version,
        asset,
    }))
}

/// Strictly newer by semver precedence (build metadata ignored, so `1.0.0+a` vs `1.0.0+b` is not
/// an update). A prerelease is only offered to someone already on a prerelease: if CI ever
/// publishes a beta to the stable feed, stable users stay put.
pub(crate) fn is_newer(candidate: &Version, current: &Version) -> bool {
    if !candidate.pre.is_empty() && current.pre.is_empty() {
        return false;
    }
    candidate.cmp_precedence(current).is_gt()
}

/// Rejects obviously bad assets before any bytes are downloaded.
fn validate_asset(asset: &Asset, config: &UpdaterConfig) -> Result<(), UpdateError> {
    check_url(&asset.url)?;
    if asset.sha256.len() != 64 || !asset.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(UpdateError::Feed(format!("invalid sha256 {:?}", asset.sha256)));
    }
    if asset.size == 0 {
        return Err(UpdateError::Feed("asset size is 0".into()));
    }
    if asset.size > config.max_download_size {
        return Err(UpdateError::TooLarge { limit: config.max_download_size });
    }
    minisign_verify::Signature::decode(&asset.signature)
        .map_err(|e| UpdateError::Signature(format!("malformed signature in feed: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::{UpdateStatus, evaluate, is_newer};
    use crate::update::UpdateError;
    use crate::update::test_support::{TestKey, test_config};

    fn feed(version: &str, platform: &str, min_os: &str, sig: &str) -> String {
        serde_json::json!({
            "version": version,
            "pub_date": "2026-10-01T12:00:00Z",
            "notes": "Fixes",
            "minimum_system_version": min_os,
            "platforms": {
                platform: {
                    "url": "https://cdn.example.com/releases/0.2.0/Wormhole-0.2.0-macos.app.tar.gz",
                    "sha256": "ab".repeat(32),
                    "signature": sig,
                    "size": 1234
                }
            }
        })
        .to_string()
    }

    fn sig() -> String {
        TestKey::generate().sign(b"whatever")
    }

    #[test]
    fn valid_feed_is_available() {
        let config = test_config("0.1.0", "unused");
        let status = evaluate(feed("0.2.0", "darwin-aarch64", "13.0", &sig()).as_bytes(), &config).unwrap();
        let UpdateStatus::Available(release) = status else { panic!("expected Available, got {status:?}") };
        assert_eq!(release.version, Version::new(0, 2, 0));
        assert_eq!(release.notes, "Fixes");
        assert_eq!(release.pub_date.as_deref(), Some("2026-10-01T12:00:00Z"));
        assert_eq!(release.minimum_system_version, Some(Version::new(13, 0, 0)));
        assert_eq!(release.asset.size, 1234);
    }

    #[test]
    fn same_or_older_is_up_to_date() {
        for current in ["0.2.0", "0.3.0", "0.2.0+build.7"] {
            let config = test_config(current, "unused");
            let status =
                evaluate(feed("0.2.0", "darwin-aarch64", "13.0", &sig()).as_bytes(), &config).unwrap();
            assert_eq!(status, UpdateStatus::UpToDate, "current {current}");
        }
    }

    #[test]
    fn missing_platform_is_an_error() {
        let config = test_config("0.1.0", "unused");
        let err = evaluate(feed("0.2.0", "darwin-x86_64", "13.0", &sig()).as_bytes(), &config).unwrap_err();
        assert!(matches!(err, UpdateError::Feed(ref m) if m.contains("darwin-aarch64")), "{err}");
    }

    #[test]
    fn missing_platform_is_fine_when_up_to_date() {
        let config = test_config("0.2.0", "unused");
        let status = evaluate(feed("0.2.0", "darwin-x86_64", "13.0", &sig()).as_bytes(), &config).unwrap();
        assert_eq!(status, UpdateStatus::UpToDate);
    }

    #[test]
    fn too_old_macos_is_unsupported() {
        let mut config = test_config("0.1.0", "unused");
        config.system_version = Some(Version::new(13, 6, 0));
        let status = evaluate(feed("0.2.0", "darwin-aarch64", "14.0", &sig()).as_bytes(), &config).unwrap();
        assert_eq!(
            status,
            UpdateStatus::Unsupported {
                version: Version::new(0, 2, 0),
                minimum_system_version: Version::new(14, 0, 0)
            }
        );
        // Unknown system version: don't block.
        config.system_version = None;
        let status = evaluate(feed("0.2.0", "darwin-aarch64", "14.0", &sig()).as_bytes(), &config).unwrap();
        assert!(matches!(status, UpdateStatus::Available(_)));
    }

    #[test]
    fn malformed_feeds_are_rejected() {
        let config = test_config("0.1.0", "unused");
        assert!(matches!(evaluate(b"{", &config), Err(UpdateError::Feed(_))));
        assert!(matches!(evaluate(b"{\"version\":\"0.2.0\"}", &config), Err(UpdateError::Feed(_))));
        let bad_version = feed("v2", "darwin-aarch64", "13.0", &sig());
        assert!(matches!(evaluate(bad_version.as_bytes(), &config), Err(UpdateError::Feed(_))));
        let bad_sig = feed("0.2.0", "darwin-aarch64", "13.0", "not a signature");
        assert!(matches!(evaluate(bad_sig.as_bytes(), &config), Err(UpdateError::Signature(_))));
    }

    #[test]
    fn insecure_or_oversized_assets_are_rejected() {
        let config = test_config("0.1.0", "unused");
        let http = feed("0.2.0", "darwin-aarch64", "13.0", &sig()).replace("https://cdn", "http://cdn");
        assert!(matches!(evaluate(http.as_bytes(), &config), Err(UpdateError::Feed(_))));
        let huge = feed("0.2.0", "darwin-aarch64", "13.0", &sig()).replace("1234", "600000000");
        assert!(matches!(evaluate(huge.as_bytes(), &config), Err(UpdateError::TooLarge { .. })));
        let bad_hash = feed("0.2.0", "darwin-aarch64", "13.0", &sig()).replace(&"ab".repeat(32), "zz");
        assert!(matches!(evaluate(bad_hash.as_bytes(), &config), Err(UpdateError::Feed(_))));
    }

    #[test]
    fn version_ordering() {
        let v = |s: &str| Version::parse(s).unwrap();
        assert!(is_newer(&v("0.2.0"), &v("0.1.9")));
        assert!(is_newer(&v("1.0.0"), &v("0.99.99")));
        assert!(!is_newer(&v("0.1.0"), &v("0.2.0")), "downgrade");
        assert!(!is_newer(&v("0.2.0"), &v("0.2.0")));
        assert!(!is_newer(&v("0.2.0+b2"), &v("0.2.0+b1")), "build metadata is not a version");
        // Prereleases: the final release supersedes its betas...
        assert!(is_newer(&v("0.2.0"), &v("0.2.0-beta.2")));
        assert!(is_newer(&v("0.2.0-beta.10"), &v("0.2.0-beta.2")));
        assert!(!is_newer(&v("0.2.0-beta.1"), &v("0.2.0-beta.2")));
        // ...but stable users are never moved onto a prerelease.
        assert!(!is_newer(&v("0.3.0-beta.1"), &v("0.2.0")));
        assert!(!is_newer(&v("0.2.0-rc.1"), &v("0.2.0")));
    }
}
