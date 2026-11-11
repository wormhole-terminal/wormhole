//! A release feed on 127.0.0.1 with a signed fake `Wormhole.app` archive, and a scratch folder with an older
//! bundle to be replaced. The signing key is generated here: it is never the app's.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use flate2::Compression;
use flate2::write::GzEncoder;
use sha2::{Digest as _, Sha256};

use crate::aux_backends::common::test_support::Wakes;
use crate::aux_backends::{UpdateChannelRecord, UpdateEngine, UpdateSetup};

pub(super) const BUNDLE_ID: &str = "dev.wormhole.swift-test";
const ARCHIVE_PATH: &str = "/releases/0.2.0/Wormhole-0.2.0-macos.app.tar.gz";

pub(super) struct Key {
    pk: minisign::PublicKey,
    sk: minisign::SecretKey,
}

impl Key {
    pub(super) fn generate() -> Self {
        let pair = minisign::KeyPair::generate_unencrypted_keypair().expect("a key");
        Self { pk: pair.pk, sk: pair.sk }
    }

    pub(super) fn public_text(&self) -> String {
        self.pk.to_box().expect("boxed").into_string()
    }

    /// What CI signs: the archive, with the release and file named in the trusted comment.
    pub(super) fn sign(&self, data: &[u8], version: &str, file: &str) -> String {
        let comment = format!("Wormhole {version} {file}");
        minisign::sign(Some(&self.pk), &self.sk, Cursor::new(data), Some(&comment), None)
            .expect("signs")
            .into_string()
    }
}

fn write_app(path: &Path, version: &str, marker: &str) {
    std::fs::create_dir_all(path.join("Contents/MacOS")).expect("bundle folders");
    std::fs::write(
        path.join("Contents/Info.plist"),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>{BUNDLE_ID}</string>
<key>CFBundleShortVersionString</key><string>{version}</string>
<key>CFBundleExecutable</key><string>Wormhole</string>
</dict></plist>"#
        ),
    )
    .expect("plist");
    let exe = path.join("Contents/MacOS/Wormhole");
    std::fs::write(&exe, format!("#!/bin/sh\necho {marker}\n")).expect("executable");
    std::fs::set_permissions(&exe, std::os::unix::fs::PermissionsExt::from_mode(0o755)).expect("mode");
}

pub(super) fn marker_of(app: &Path) -> String {
    std::fs::read_to_string(app.join("Contents/MacOS/Wormhole")).expect("executable")
}

fn archive(version: &str) -> Vec<u8> {
    let src = tempfile::tempdir().expect("temp");
    let app = src.path().join("Wormhole.app");
    write_app(&app, version, "new");
    let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
    builder.follow_symlinks(false);
    builder.append_dir_all("Wormhole.app", &app).expect("tars");
    builder.into_inner().expect("tar").finish().expect("gz")
}

/// A feed's JSON for `version` with the asset `body` signed as `signature` says.
fn feed(base: &str, version: &str, body: &[u8], signature: &str, notes: &str) -> String {
    let platform = wormhole_core::update::current_platform();
    serde_json::json!({
        "version": version,
        "pub_date": "2026-10-01T12:00:00Z",
        "notes": notes,
        "platforms": { platform: {
            "url": format!("{base}{ARCHIVE_PATH}"),
            "sha256": Sha256::digest(body).iter().map(|b| format!("{b:02x}")).collect::<String>(),
            "signature": signature,
            "size": body.len(),
        }}
    })
    .to_string()
}

pub(super) struct Published {
    pub(super) engine: Arc<UpdateEngine>,
    pub(super) wakes: Arc<Wakes>,
    /// The folder the scratch bundle is in; dropped with this.
    pub(super) apps: tempfile::TempDir,
    pub(super) bundle: PathBuf,
}

/// What the loopback server offers: stable 0.2.0 (signed by `signer`), and optionally a preview feed.
pub(super) struct Offer {
    pub(super) signer_is_trusted: bool,
    pub(super) preview: Option<&'static str>,
}

impl Default for Offer {
    fn default() -> Self {
        Self { signer_is_trusted: true, preview: None }
    }
}

/// Serves the feeds and the archive and builds an engine over a scratch bundle at 0.1.0.
pub(super) fn publish(offer: Offer) -> Published {
    let trusted = Key::generate();
    let signer = if offer.signer_is_trusted { None } else { Some(Key::generate()) };
    let body = archive("0.2.0");
    let file = "Wormhole-0.2.0-macos.app.tar.gz";
    let signature = signer.as_ref().unwrap_or(&trusted).sign(&body, "0.2.0", file);

    let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").expect("binds"));
    let base = format!("http://{}", server.server_addr().to_ip().expect("an ip address"));
    let mut routes: HashMap<String, Vec<u8>> = HashMap::new();
    routes.insert(ARCHIVE_PATH.into(), body.clone());
    routes.insert(
        "/stable/latest.json".into(),
        feed(&base, "0.2.0", &body, &signature, "New stuff").into_bytes(),
    );
    if let Some(version) = offer.preview {
        let preview = feed(&base, version, &body, &signature, "A build of main");
        routes.insert("/unstable/latest.json".into(), preview.into_bytes());
    }
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let response = match routes.get(request.url()) {
                Some(data) => tiny_http::Response::from_data(data.clone()),
                None => tiny_http::Response::from_data(b"not found".to_vec()).with_status_code(404),
            };
            let _ = request.respond(response);
        }
    });

    let apps = tempfile::tempdir().expect("temp");
    let bundle = apps.path().join("Wormhole.app");
    write_app(&bundle, "0.1.0", "old");
    let wakes = Wakes::new();
    let engine = UpdateEngine::new(
        UpdateSetup {
            base_url: base,
            public_key: trusted.public_text(),
            current_version: "0.1.0".into(),
            bundle_id: BUNDLE_ID.into(),
            team_id: None,
            app_bundle: Some(bundle.display().to_string()),
            verify_codesign: false,
            fallback_url: None,
            schedule_file: Some(apps.path().join("update-schedule.json").display().to_string()),
            channel: UpdateChannelRecord::Stable,
        },
        wakes.clone(),
    )
    .expect("an engine");
    // The setting is on by default and the tests below ask for each step; the automatic one is its own test.
    engine.set_auto_update(false);
    Published { engine, wakes, apps, bundle }
}
