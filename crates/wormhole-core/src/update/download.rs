//! Streaming the archive to disk and verifying it (size, sha256, minisign) in the same pass.

use std::io::{Read, Write as _};
use std::path::Path;

use sha2::{Digest as _, Sha256};
use tempfile::NamedTempFile;

use super::{Release, UpdateError, UpdaterConfig, check_url, http};

const CHUNK: usize = 64 * 1024;

/// An archive whose checksum and signature have been verified. Only [`download`] creates one, so
/// holding one means "safe to extract". The temp file is deleted on drop.
#[derive(Debug)]
pub struct DownloadedUpdate {
    release: Release,
    file: NamedTempFile,
}

impl DownloadedUpdate {
    pub fn release(&self) -> &Release {
        &self.release
    }

    pub fn path(&self) -> &Path {
        self.file.path()
    }

    /// The open handle, so install reads exactly the bytes that were verified rather than
    /// re-opening a path that could have been swapped in the meantime.
    pub(crate) fn file(&self) -> &std::fs::File {
        self.file.as_file()
    }
}

/// Downloads `release`'s asset to a temp file, reporting `(bytes_done, total)` as it goes, and
/// returns only once size, sha256 and the minisign signature all check out.
///
/// Takes the config (unlike the original sketch) because the public key and size cap live there.
pub fn download(
    config: &UpdaterConfig,
    release: &Release,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<DownloadedUpdate, UpdateError> {
    let asset = &release.asset;
    check_url(&asset.url)?;
    if asset.size > config.max_download_size {
        return Err(UpdateError::TooLarge { limit: config.max_download_size });
    }
    // Decode key and signature before touching the network: a bad embedded key is a bug we
    // want to hear about without downloading 100 MB first.
    let public_key = decode_public_key(&config.public_key)?;
    let signature = minisign_verify::Signature::decode(&asset.signature)
        .map_err(|e| UpdateError::Signature(format!("malformed signature: {e}")))?;
    // Legacy (non-prehashed) signatures can't be streamed and CI never makes them; refusing
    // them here also means this is a clear "signature" error rather than a confusing one later.
    let mut verifier = public_key
        .verify_stream(&signature)
        .map_err(|e| UpdateError::Signature(format!("signature not usable with the embedded key: {e}")))?;

    tracing::info!(version = %release.version, url = %asset.url, size = asset.size, "downloading update");
    let response = http::get(&asset.url, http::DOWNLOAD_TIMEOUT)?;
    if let Some(len) = response.body().content_length()
        && len != asset.size
    {
        // Covers both an oversized response and a truncated/stale object in the bucket.
        if len > config.max_download_size {
            return Err(UpdateError::TooLarge { limit: config.max_download_size });
        }
        return Err(UpdateError::Network(format!("server sent {len} bytes, feed says {}", asset.size)));
    }
    let mut reader = response.into_body().into_reader();

    let mut file = NamedTempFile::with_prefix("wormhole-update-")?;
    let mut hasher = Sha256::new();
    let mut done: u64 = 0;
    let total = Some(asset.size);
    progress(0, total);
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n =
            reader.read(&mut buf).map_err(|e| UpdateError::Network(format!("download interrupted: {e}")))?;
        if n == 0 {
            break;
        }
        done += n as u64;
        // The feed's size is the limit for the stream too, so a lying server can't fill the disk.
        if done > asset.size {
            return Err(if done > config.max_download_size {
                UpdateError::TooLarge { limit: config.max_download_size }
            } else {
                UpdateError::Network(format!("server sent more than the expected {} bytes", asset.size))
            });
        }
        let chunk = &buf[..n];
        hasher.update(chunk);
        verifier.update(chunk);
        file.write_all(chunk)?;
        progress(done, total);
    }
    if done != asset.size {
        return Err(UpdateError::Network(format!("download ended after {done} of {} bytes", asset.size)));
    }
    file.as_file().sync_all()?;

    let actual = hex(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(&asset.sha256) {
        return Err(UpdateError::Checksum { expected: asset.sha256.to_ascii_lowercase(), actual });
    }
    verifier.finalize().map_err(|e| UpdateError::Signature(e.to_string()))?;
    // `finalize` also verified the signature over the trusted comment, so it is now as
    // trustworthy as the archive itself, and it says which release the archive was made for.
    check_trusted_comment(signature.trusted_comment(), release)?;
    tracing::info!(version = %release.version, "update downloaded and verified");

    Ok(DownloadedUpdate { release: release.clone(), file })
}

/// The signed comment must name this release: `scripts/package.sh` signs every archive with
/// `-t "<app name> <version> <archive file name>"`.
///
/// This is what ties the archive to the version the feed announces. The feed is unsigned, so
/// without it whoever controls the feed could offer an older archive (validly signed, same key)
/// as the new release, or a prerelease as the final one: the bundle's own version cannot tell
/// them apart (`CFBundleShortVersionString` has no prerelease part). The app name is not
/// compared (the bundle identifier check covers what the app is); version and file are.
pub(crate) fn check_trusted_comment(comment: &str, release: &Release) -> Result<(), UpdateError> {
    let file = file_name(&release.asset.url);
    let wrong = || {
        UpdateError::Signature(format!(
            "the archive was signed as {comment:?}, not as release {} ({file})",
            release.version
        ))
    };
    if file.is_empty() {
        return Err(wrong());
    }
    match comment.strip_suffix(&format!(" {} {file}", release.version)) {
        Some(app) if !app.is_empty() && !app.contains(char::is_whitespace) => Ok(()),
        _ => Err(wrong()),
    }
}

/// The last path segment of a URL (without query or fragment).
fn file_name(url: &str) -> &str {
    let path = url.split(['?', '#']).next().unwrap_or_default();
    path.rsplit('/').next().unwrap_or_default()
}

/// Accepts the whole `.pub` file (comment line + key) or just the base64 key line, since either
/// is what someone will naturally paste into the source.
pub(crate) fn decode_public_key(text: &str) -> Result<minisign_verify::PublicKey, UpdateError> {
    let text = text.trim();
    let result = if text.lines().count() > 1 {
        minisign_verify::PublicKey::decode(text)
    } else {
        minisign_verify::PublicKey::from_base64(text)
    };
    result.map_err(|e| UpdateError::Signature(format!("invalid embedded public key: {e}")))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// sha256 of a byte slice as lowercase hex (tests and tooling).
#[cfg(test)]
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::{check_trusted_comment, decode_public_key};
    use crate::update::Release;
    use crate::update::test_support::{TestKey, sample_release};

    #[test]
    fn public_key_accepts_file_or_bare_key() {
        let key = TestKey::generate();
        assert!(decode_public_key(&key.public_key_text()).is_ok());
        let bare = key.public_key_text().lines().last().unwrap().to_string();
        assert!(decode_public_key(&bare).is_ok());
        assert!(decode_public_key("RWnope").is_err());
    }

    fn release_at(version: &str, url: &str) -> Release {
        let mut release = sample_release(version);
        release.asset.url = url.into();
        release
    }

    #[test]
    fn the_trusted_comment_names_version_and_file() {
        let url = "https://dl.example.com/wormhole/releases/1.2.3/Wormhole-1.2.3-macos.app.tar.gz";
        let release = release_at("1.2.3", url);
        let ok = |comment: &str| check_trusted_comment(comment, &release).is_ok();
        assert!(ok("Wormhole 1.2.3 Wormhole-1.2.3-macos.app.tar.gz"));
        assert!(ok("Wormhole2 1.2.3 Wormhole-1.2.3-macos.app.tar.gz"), "the app name is not compared");
        assert!(!ok("Wormhole 1.2.2 Wormhole-1.2.2-macos.app.tar.gz"));
        assert!(!ok("Wormhole 1.2.3 Wormhole-1.2.2-macos.app.tar.gz"));
        assert!(!ok("Wormhole 1.2.3 Wormhole-1.2.3-macos.dmg"));
        assert!(!ok("Wormhole 1.2.3-beta.1 Wormhole-1.2.3-macos.app.tar.gz"));
        assert!(!ok("Wormhole 1.2.3 Wormhole-1.2.3-macos.app.tar.gz and more"));
        assert!(!ok(" 1.2.3 Wormhole-1.2.3-macos.app.tar.gz"), "no app name");
        assert!(!ok("x y 1.2.3 Wormhole-1.2.3-macos.app.tar.gz"), "the app name is one word");
        assert!(!ok("timestamp:1700000000\tfile:Wormhole-1.2.3-macos.app.tar.gz"));
        assert!(!ok(""));
    }

    #[test]
    fn prereleases_and_odd_urls() {
        let release =
            release_at("0.3.0-beta.1", "https://h/r/Wormhole-0.3.0-beta.1-macos.app.tar.gz?sig=1#x");
        assert!(
            check_trusted_comment("Wormhole 0.3.0-beta.1 Wormhole-0.3.0-beta.1-macos.app.tar.gz", &release)
                .is_ok()
        );
        assert!(check_trusted_comment("Wormhole 0.3.0 Wormhole-0.3.0-macos.app.tar.gz", &release).is_err());
        // A URL that names no file cannot be matched to anything.
        let release = release_at("1.0.0", "https://h/dir/");
        assert!(check_trusted_comment("Wormhole 1.0.0 ", &release).is_err());
    }

    #[test]
    fn package_sh_signs_what_the_updater_checks() {
        // The two halves live in different languages; if either side's format changes, this
        // fails and says to change both.
        let package = include_str!("../../../../scripts/package.sh");
        let lib = include_str!("../../../../scripts/lib.sh");
        assert!(package.contains(r#"-t "$APP_NAME $APP_VERSION $ARCHIVE_NAME""#), "trusted comment format");
        assert!(lib.contains(r#"ARCHIVE_NAME="$APP_NAME-$APP_VERSION-macos.app.tar.gz""#), "archive name");
        assert!(package.contains(r#"url="$base_url/releases/$APP_VERSION/$ARCHIVE_NAME""#), "asset url");
        assert!(package.contains(r#"--arg version "$APP_VERSION""#), "feed version");
    }
}
