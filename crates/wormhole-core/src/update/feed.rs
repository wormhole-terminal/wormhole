//! The release feed (`latest.json`) and the "is there something newer for me?" decision.

use std::collections::HashMap;
use std::io::Read as _;

use semver::Version;
use serde::Deserialize;

use super::count::{self, Count};
use super::security::Security;
use super::{UpdateError, UpdaterConfig, check_url, feed_url, http, parse_lenient_version};

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
    /// Set when the release fixes a security problem. Presentation only: the feed is unsigned,
    /// so this never changes what is downloaded or how it is verified.
    pub security: Option<Security>,
}

impl Release {
    pub fn is_security_update(&self) -> bool {
        self.security.is_some()
    }
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
    /// Kept as raw JSON: [`Security::from_feed`] reads it leniently, so a malformed value here
    /// can't make the whole feed (and with it the update) unreadable.
    #[serde(default)]
    security: Option<serde_json::Value>,
}

/// Fetches the channel's feeds and decides. Network errors are `Network` (retry later); anything
/// the server sent that we won't accept is `Feed`.
///
/// With more than one feed (Unstable reads Stable's too) the newest offer wins, and an error counts
/// only when no feed answered: a broken Unstable feed must not keep a stable release from anyone,
/// nor make the schedule retry every minute as it does after a network error.
pub fn check(config: &UpdaterConfig) -> Result<UpdateStatus, UpdateError> {
    check_counted(config).map(|checked| checked.status)
}

/// A check's answer, and whether its count reached the download server.
#[derive(Debug, Clone, PartialEq)]
pub struct Checked {
    pub status: UpdateStatus,
    /// False when the count was off, or the channel's own feed was read from the fallback
    /// (`UpdaterConfig::fallback_url`), which never gets it: the day and month are counted later.
    pub counted: bool,
}

/// [`check`], also saying whether the count went out.
pub fn check_counted(config: &UpdaterConfig) -> Result<Checked, UpdateError> {
    let mut best: Option<Result<UpdateStatus, UpdateError>> = None;
    let mut counted = false;
    let user_agent = count::user_agent(config);
    for (i, &channel) in config.channel.feeds().iter().enumerate() {
        // The count's flags go with the channel's own feed only: a check that reads two counts once.
        let flags = config.count.filter(|_| i == 0).and_then(Count::header);
        let mut result = check_feed(&feed_url(&config.base_url, channel), config, &user_agent, flags);
        if result.is_ok() && i == 0 {
            counted = config.count.is_some();
        }
        if let (Err(UpdateError::Network(err)), Some(fallback)) = (&result, &config.fallback_url) {
            tracing::warn!(?channel, "update feed failed, trying GitHub: {err}");
            let url = format!("{}/{}", fallback.trim_end_matches('/'), channel.github_feed_path());
            let plain = count::plain_user_agent(&config.current_version);
            result = check_feed(&url, config, &plain, None);
        }
        if let Err(err) = &result {
            tracing::warn!(?channel, "update feed failed: {err}");
        }
        best = Some(match best {
            Some(prev) => better(prev, result),
            None => result,
        });
    }
    let status = best.unwrap_or(Ok(UpdateStatus::UpToDate))?;
    tracing::info!(?status, counted, current = %config.current_version, "update check finished");
    Ok(Checked { status, counted })
}

/// The more useful of two feeds' answers: an update (the newer one), then a version this Mac
/// can't run, then "up to date", then an error. Ties keep the first, the channel's own feed.
fn better(
    a: Result<UpdateStatus, UpdateError>,
    b: Result<UpdateStatus, UpdateError>,
) -> Result<UpdateStatus, UpdateError> {
    fn rank(r: &Result<UpdateStatus, UpdateError>) -> u8 {
        match r {
            Ok(UpdateStatus::Available(_)) => 3,
            Ok(UpdateStatus::Unsupported { .. }) => 2,
            Ok(UpdateStatus::UpToDate) => 1,
            Err(_) => 0,
        }
    }
    if let (Ok(UpdateStatus::Available(x)), Ok(UpdateStatus::Available(y))) = (&a, &b) {
        return if y.version.cmp_precedence(&x.version).is_gt() { b } else { a };
    }
    if rank(&b) > rank(&a) { b } else { a }
}

fn check_feed(
    url: &str,
    config: &UpdaterConfig,
    user_agent: &str,
    count: Option<&str>,
) -> Result<UpdateStatus, UpdateError> {
    check_url(url)?;
    tracing::debug!(%url, "checking for updates");
    let headers: Vec<(&str, &str)> = count.map(|flags| (count::HEADER, flags)).into_iter().collect();
    let response = http::request(url, http::FEED_TIMEOUT, user_agent, &headers)?;
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
    evaluate(&body, config)
}

/// The pure part of [`check`]: parse the feed and compare against the config.
pub(crate) fn evaluate(json: &[u8], config: &UpdaterConfig) -> Result<UpdateStatus, UpdateError> {
    let feed: Feed = serde_json::from_slice(json).map_err(|e| UpdateError::Feed(e.to_string()))?;
    let version = Version::parse(feed.version.trim())
        .map_err(|e| UpdateError::Feed(format!("invalid version {:?}: {e}", feed.version)))?;

    if !is_newer(&version, &config.current_version, config.channel.takes_prereleases()) {
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
        security: Security::from_feed(feed.security.as_ref()),
    }))
}

/// Strictly newer by semver precedence (build metadata ignored, so `1.0.0+a` vs `1.0.0+b` is not
/// an update). A prerelease is only offered on a channel that takes them, or to someone already
/// on one: if CI ever publishes a main build to the stable feed, stable users stay put.
pub(crate) fn is_newer(candidate: &Version, current: &Version, prereleases: bool) -> bool {
    if !candidate.pre.is_empty() && current.pre.is_empty() && !prereleases {
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

    use super::{UpdateStatus, better, evaluate, is_newer};
    use crate::update::test_support::{TestKey, sample_release, test_config};
    use crate::update::{UpdateChannel, UpdateError};

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
    fn the_security_key_is_optional_and_never_breaks_the_feed() {
        use crate::update::Severity;

        let config = test_config("0.1.0", "unused");
        let release_with = |security: serde_json::Value| {
            let mut feed: serde_json::Value =
                serde_json::from_str(&feed("0.2.0", "darwin-aarch64", "13.0", &sig())).unwrap();
            if !security.is_null() {
                feed["security"] = security;
            }
            match evaluate(feed.to_string().as_bytes(), &config).unwrap() {
                UpdateStatus::Available(release) => release,
                other => panic!("expected Available, got {other:?}"),
            }
        };
        assert_eq!(release_with(serde_json::Value::Null).security, None);
        let release = release_with(serde_json::json!({
            "severity": "critical",
            "advisories": ["GHSA-cfgh-2345-rvwx", "not an id"]
        }));
        assert!(release.is_security_update());
        let security = release.security.unwrap();
        assert_eq!(security.severity, Severity::Critical);
        assert_eq!(security.advisories, ["ghsa-cfgh-2345-rvwx"]);
        // Values from a newer or broken feed: still installable, still a security update.
        for odd in [
            serde_json::json!({ "severity": "apocalyptic" }),
            serde_json::json!({ "severity": ["high"], "advisories": {"a": 1} }),
            serde_json::json!("high"),
            serde_json::json!(3),
            serde_json::json!([]),
        ] {
            let security = release_with(odd.clone()).security;
            assert_eq!(security.map(|s| s.severity), Some(Severity::Unknown), "{odd}");
        }
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
        let is_newer = |a: &Version, b: &Version| is_newer(a, b, false);
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

    #[test]
    fn unstable_takes_the_builds_of_main_between_releases() {
        let v = |s: &str| Version::parse(s).unwrap();
        // CI numbers a main build after 1.2.0 as 1.2.1-dev.<commits since the tag>.
        assert!(is_newer(&v("1.2.1-dev.3"), &v("1.2.0"), true));
        assert!(!is_newer(&v("1.2.1-dev.3"), &v("1.2.0"), false), "not on Stable");
        assert!(is_newer(&v("1.2.1-dev.10"), &v("1.2.1-dev.9"), true), "numeric, not text, order");
        // The next release overtakes them, whatever it is called...
        assert!(is_newer(&v("1.2.1"), &v("1.2.1-dev.40"), false));
        assert!(is_newer(&v("1.3.0"), &v("1.2.1-dev.40"), false));
        // ...and a build after a prerelease release sorts between it and the next.
        assert!(is_newer(&v("1.3.0-beta.1.dev.2"), &v("1.3.0-beta.1"), true));
        assert!(is_newer(&v("1.3.0-beta.2"), &v("1.3.0-beta.1.dev.2"), true));
    }

    #[test]
    fn unstable_offers_a_main_build_that_stable_refuses() {
        let main_build = feed("1.2.1-dev.3", "darwin-aarch64", "13.0", &sig());
        let stable = test_config("1.2.0", "unused");
        assert_eq!(evaluate(main_build.as_bytes(), &stable).unwrap(), UpdateStatus::UpToDate);
        let mut unstable = test_config("1.2.0", "unused");
        unstable.channel = UpdateChannel::Unstable;
        let status = evaluate(main_build.as_bytes(), &unstable).unwrap();
        assert!(
            matches!(status, UpdateStatus::Available(r) if r.version == Version::parse("1.2.1-dev.3").unwrap())
        );
    }

    #[test]
    fn the_newer_feed_wins_and_an_error_only_when_nothing_answered() {
        let available = |v: &str| Ok(UpdateStatus::Available(sample_release(v)));
        let version = |r: Result<UpdateStatus, UpdateError>| match r {
            Ok(UpdateStatus::Available(r)) => r.version.to_string(),
            other => format!("{other:?}"),
        };
        assert_eq!(version(better(available("1.2.1-dev.3"), available("1.3.0"))), "1.3.0");
        assert_eq!(version(better(available("1.3.1-dev.1"), available("1.3.0"))), "1.3.1-dev.1");
        let offline = || Err(UpdateError::Network("offline".into()));
        assert_eq!(version(better(offline(), available("1.3.0"))), "1.3.0");
        assert!(matches!(better(offline(), Ok(UpdateStatus::UpToDate)), Ok(UpdateStatus::UpToDate)));
        assert!(matches!(better(Ok(UpdateStatus::UpToDate), offline()), Ok(UpdateStatus::UpToDate)));
        assert!(matches!(better(offline(), offline()), Err(UpdateError::Network(_))));
    }
}
