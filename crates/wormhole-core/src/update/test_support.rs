//! Test fixtures: a throwaway minisign key, fake app bundles/archives and a loopback HTTP server.

use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Write as _};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use flate2::Compression;
use flate2::write::GzEncoder;
use semver::Version;

use super::{Asset, DEFAULT_MAX_DOWNLOAD_SIZE, Release, UpdateChannel, UpdaterConfig};

pub const BUNDLE_ID: &str = "dev.wormhole.test";

/// What `scripts/package.sh` puts in the signature of the 0.2.0 test archive:
/// `<app name> <version> <archive file name>`.
pub const TEST_TRUSTED_COMMENT: &str = "Wormhole 0.2.0 Wormhole-0.2.0-macos.app.tar.gz";

pub struct TestKey {
    pk: minisign::PublicKey,
    sk: minisign::SecretKey,
}

impl TestKey {
    pub fn generate() -> Self {
        let kp = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        Self { pk: kp.pk, sk: kp.sk }
    }

    /// The `.pub` file text, as it would be embedded in the app.
    pub fn public_key_text(&self) -> String {
        self.pk.to_box().unwrap().into_string()
    }

    /// The `.minisig` text, as CI would put it in the feed for the test release (0.2.0).
    pub fn sign(&self, data: &[u8]) -> String {
        self.sign_with_comment(data, TEST_TRUSTED_COMMENT)
    }

    /// Signed with another trusted comment, like an archive CI signed for a different release.
    pub fn sign_with_comment(&self, data: &[u8], trusted_comment: &str) -> String {
        minisign::sign(Some(&self.pk), &self.sk, Cursor::new(data), Some(trusted_comment), None)
            .unwrap()
            .into_string()
    }
}

pub fn test_config(current: &str, public_key: &str) -> UpdaterConfig {
    UpdaterConfig {
        base_url: "http://127.0.0.1:1".into(),
        channel: UpdateChannel::Stable,
        public_key: public_key.into(),
        current_version: Version::parse(current).unwrap(),
        platform: "darwin-aarch64".into(),
        system_version: Some(Version::new(14, 0, 0)),
        bundle_identifier: BUNDLE_ID.into(),
        also_accepted: Vec::new(),
        bundle_executable: "Wormhole".into(),
        verify_codesign: false,
        team_id: None,
        max_download_size: DEFAULT_MAX_DOWNLOAD_SIZE,
    }
}

pub fn sample_release(version: &str) -> Release {
    Release {
        version: Version::parse(version).unwrap(),
        notes: String::new(),
        pub_date: None,
        minimum_system_version: None,
        asset: Asset { url: String::new(), sha256: String::new(), signature: String::new(), size: 0 },
        security: None,
    }
}

pub fn info_plist(bundle_id: &str, version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>{bundle_id}</string>
<key>CFBundleShortVersionString</key><string>{version}</string>
<key>CFBundleExecutable</key><string>Wormhole</string>
</dict></plist>"#
    )
}

/// Writes a minimal bundle at `path` whose executable prints `marker`.
pub fn write_fake_app(path: &Path, bundle_id: &str, version: &str, marker: &str) {
    fs::create_dir_all(path.join("Contents/MacOS")).unwrap();
    fs::write(path.join("Contents/Info.plist"), info_plist(bundle_id, version)).unwrap();
    let exe = path.join("Contents/MacOS/Wormhole");
    fs::write(&exe, format!("#!/bin/sh\necho {marker}\n")).unwrap();
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
}

pub fn marker_of(app: &Path) -> String {
    fs::read_to_string(app.join("Contents/MacOS/Wormhole")).unwrap()
}

/// A `.tar.gz` of a fake `Wormhole.app`, shaped like CI's (with a framework symlink).
pub fn fake_app_archive(bundle_id: &str, version: &str) -> Vec<u8> {
    let src = tempfile::tempdir().unwrap();
    let app = src.path().join("Wormhole.app");
    write_fake_app(&app, bundle_id, version, "new");
    let versions = app.join("Contents/Frameworks/X.framework/Versions");
    fs::create_dir_all(versions.join("A")).unwrap();
    fs::write(versions.join("A/X"), b"lib").unwrap();
    std::os::unix::fs::symlink("A", versions.join("Current")).unwrap();

    let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
    builder.follow_symlinks(false);
    builder.append_dir_all("Wormhole.app", &app).unwrap();
    builder.into_inner().unwrap().finish().unwrap()
}

pub enum RawEntry<'a> {
    File(&'a str, &'a [u8]),
    /// A file with explicit mode bits (setuid and friends included).
    FileMode(&'a str, &'a [u8], u32),
    /// A directory with explicit mode bits.
    DirMode(&'a str, u32),
    /// An entry that is neither file, directory nor link: a FIFO or a device node.
    Special(&'a str, tar::EntryType),
    Dir(&'a str),
    Symlink(&'a str, &'a str),
    HardLink(&'a str, &'a str),
}

/// Builds a `.tar.gz` byte-for-byte, bypassing `tar::Builder`'s own path checks, so tests can
/// produce the malicious archives a real attacker would.
pub fn raw_archive(entries: &[RawEntry<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    for entry in entries {
        let mut header = tar::Header::new_old();
        let mut mode = None;
        let (name, kind, data, link): (&str, tar::EntryType, &[u8], Option<&str>) = match *entry {
            RawEntry::File(n, d) => (n, tar::EntryType::Regular, d, None),
            RawEntry::FileMode(n, d, m) => {
                mode = Some(m);
                (n, tar::EntryType::Regular, d, None)
            }
            RawEntry::DirMode(n, m) => {
                mode = Some(m);
                (n, tar::EntryType::Directory, b"", None)
            }
            RawEntry::Special(n, kind) => (n, kind, b"", None),
            RawEntry::Dir(n) => (n, tar::EntryType::Directory, b"", None),
            RawEntry::Symlink(n, t) => (n, tar::EntryType::Symlink, b"", Some(t)),
            RawEntry::HardLink(n, t) => (n, tar::EntryType::Link, b"", Some(t)),
        };
        let old = header.as_old_mut();
        old.name[..name.len()].copy_from_slice(name.as_bytes());
        if let Some(t) = link {
            old.linkname[..t.len()].copy_from_slice(t.as_bytes());
        }
        header.set_entry_type(kind);
        header.set_mode(mode.unwrap_or(if kind == tar::EntryType::Directory { 0o755 } else { 0o644 }));
        header.set_size(data.len() as u64);
        header.set_cksum();
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(data);
        out.resize(out.len().div_ceil(512) * 512, 0);
    }
    out.extend_from_slice(&[0; 1024]);
    let mut gz = GzEncoder::new(Vec::new(), Compression::fast());
    gz.write_all(&out).unwrap();
    gz.finish().unwrap()
}

/// Serves fixed bodies on 127.0.0.1 and returns the base URL. The server thread lives until the
/// test process exits, which is fine for tests.
pub fn serve(routes: HashMap<String, Vec<u8>>) -> String {
    let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
    let base = format!("http://{}", server.server_addr().to_ip().unwrap());
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let response = match routes.get(request.url()) {
                Some(body) => tiny_http::Response::from_data(body.clone()),
                None => tiny_http::Response::from_data(b"not found".to_vec()).with_status_code(404),
            };
            let _ = request.respond(response);
        }
    });
    base
}

pub fn temp_apps_dir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("Wormhole.app");
    write_fake_app(&app, BUNDLE_ID, "0.1.0", "old");
    (dir, app)
}
