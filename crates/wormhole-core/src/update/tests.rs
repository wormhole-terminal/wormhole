//! End-to-end tests against a loopback HTTP server: check → download → verify → install.

use std::collections::HashMap;
use std::fs;
use std::io::Write as _;
use std::path::Path;

use semver::Version;

use super::archive::extract;
use super::download::sha256_hex;
use super::install::swap_into_place;
use super::test_support::{
    BUNDLE_ID, RawEntry, TEST_TRUSTED_COMMENT, TestKey, fake_app_archive, marker_of, raw_archive, serve,
    temp_apps_dir, test_config, write_fake_app,
};
use super::{UpdateChannel, UpdateError, UpdateStatus, UpdaterConfig, check, download, install, preflight};

const ARCHIVE_PATH: &str = "/releases/0.2.0/Wormhole-0.2.0-macos.app.tar.gz";

struct Published {
    config: UpdaterConfig,
}

/// Serves a feed for 0.2.0 plus `served` at the asset URL. `advertised` is what the feed's
/// sha256/size/signature describe, so tests can make the two disagree.
fn publish(key: &TestKey, advertised: &[u8], served: Vec<u8>, signature: Option<String>) -> Published {
    publish_with(key, advertised, served, signature, |_| {})
}

fn publish_with(
    key: &TestKey,
    advertised: &[u8],
    served: Vec<u8>,
    signature: Option<String>,
    edit: impl FnOnce(&mut serde_json::Value),
) -> Published {
    // Two servers because the feed must contain the archive server's (random) port.
    let archive_base = serve(HashMap::from([(ARCHIVE_PATH.to_string(), served)]));
    let mut feed = serde_json::json!({
        "version": "0.2.0",
        "pub_date": "2026-10-01T12:00:00Z",
        "notes": "New stuff",
        "minimum_system_version": "13.0",
        "platforms": {
            "darwin-aarch64": {
                "url": format!("{archive_base}{ARCHIVE_PATH}"),
                "sha256": sha256_hex(advertised),
                "signature": signature.unwrap_or_else(|| key.sign(advertised)),
                "size": advertised.len(),
            }
        }
    });
    edit(&mut feed);
    let feed_base =
        serve(HashMap::from([("/stable/latest.json".to_string(), feed.to_string().into_bytes())]));
    let mut config = test_config("0.1.0", &key.public_key_text());
    config.base_url = feed_base;
    Published { config }
}

fn available(config: &UpdaterConfig) -> super::Release {
    match check(config).unwrap() {
        UpdateStatus::Available(r) => r,
        other => panic!("expected Available, got {other:?}"),
    }
}

/// Entries in the apps folder other than the bundle: staging dirs or backups left behind.
fn leftovers(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n != "Wormhole.app")
        .collect()
}

#[test]
fn full_update_flow() {
    let key = TestKey::generate();
    let archive = fake_app_archive(BUNDLE_ID, "0.2.0");
    let p = publish(&key, &archive, archive.clone(), None);

    let release = available(&p.config);
    assert_eq!(release.version, Version::new(0, 2, 0));
    assert_eq!(release.notes, "New stuff");

    let mut calls = Vec::new();
    let update = download(&p.config, &release, |done, total| calls.push((done, total))).unwrap();
    let size = archive.len() as u64;
    assert_eq!(calls.first(), Some(&(0, Some(size))));
    assert_eq!(calls.last(), Some(&(size, Some(size))));
    assert_eq!(fs::read(update.path()).unwrap(), archive);

    let (dir, app) = temp_apps_dir();
    preflight(&app).unwrap();
    install(&p.config, &update, &app).unwrap();
    assert!(marker_of(&app).contains("new"));
    let current = app.join("Contents/Frameworks/X.framework/Versions/Current");
    assert_eq!(fs::read_link(&current).unwrap(), Path::new("A"));
    let exe_mode = crate::platform::fs::mode(&fs::metadata(app.join("Contents/MacOS/Wormhole")).unwrap());
    assert_eq!(exe_mode & 0o111, 0o111, "executable bit preserved");
    assert!(leftovers(dir.path()).is_empty(), "left behind: {:?}", leftovers(dir.path()));

    // Running the check again as the new version: nothing to do (no downgrade loop).
    let mut config = p.config.clone();
    config.current_version = Version::new(0, 2, 0);
    assert_eq!(check(&config).unwrap(), UpdateStatus::UpToDate);
}

#[test]
fn unstable_still_gets_stable_releases_when_its_own_feed_is_missing() {
    let key = TestKey::generate();
    let archive = fake_app_archive(BUNDLE_ID, "0.2.0");
    let mut config = publish(&key, &archive, archive.clone(), None).config;
    config.channel = UpdateChannel::Unstable;
    // Only stable/latest.json is served: unstable/latest.json is a 404.
    assert_eq!(available(&config).version, Version::new(0, 2, 0));
    config.current_version = Version::new(0, 2, 0);
    assert_eq!(check(&config).unwrap(), UpdateStatus::UpToDate, "an answer, not the 404");
}

#[test]
fn checksum_mismatch_is_rejected() {
    let key = TestKey::generate();
    let archive = fake_app_archive(BUNDLE_ID, "0.2.0");
    let p = publish_with(&key, &archive, archive.clone(), None, |feed| {
        feed["platforms"]["darwin-aarch64"]["sha256"] = "00".repeat(32).into();
    });
    let release = available(&p.config);
    let err = download(&p.config, &release, |_, _| {}).unwrap_err();
    assert!(
        matches!(err, UpdateError::Checksum { ref expected, .. } if *expected == "00".repeat(32)),
        "{err}"
    );
}

#[test]
fn signature_from_another_key_is_rejected() {
    let key = TestKey::generate();
    let archive = fake_app_archive(BUNDLE_ID, "0.2.0");
    let foreign = TestKey::generate().sign(&archive);
    let p = publish(&key, &archive, archive.clone(), Some(foreign));
    let release = available(&p.config);
    assert!(matches!(download(&p.config, &release, |_, _| {}), Err(UpdateError::Signature(_))));
}

#[test]
fn tampered_archive_with_matching_checksum_is_rejected() {
    // An attacker controlling feed + bucket can make sha256 match their bytes, but not the
    // signature: sign the genuine archive, serve (and hash) a modified one.
    let key = TestKey::generate();
    let genuine = fake_app_archive(BUNDLE_ID, "0.2.0");
    let evil = fake_app_archive(BUNDLE_ID, "0.2.0").into_iter().chain([0u8; 16]).collect::<Vec<_>>();
    let p = publish(&key, &evil, evil.clone(), Some(key.sign(&genuine)));
    let release = available(&p.config);
    let err = download(&p.config, &release, |_, _| {}).unwrap_err();
    assert!(matches!(err, UpdateError::Signature(_)), "{err}");
}

#[test]
fn an_archive_signed_for_another_release_is_not_accepted_as_this_one() {
    // The feed is unsigned. Whoever controls it can pair an older, validly signed archive with
    // a newer version number (or the other way round), and the bundle's own version, which the
    // install step checks, may not tell them apart (a prerelease keeps only x.y.z there). What
    // the signature does say is which release it was made for: its trusted comment.
    let key = TestKey::generate();
    let archive = fake_app_archive(BUNDLE_ID, "0.2.0");
    for comment in [
        "Wormhole 0.1.5 Wormhole-0.1.5-macos.app.tar.gz", // an older release
        "Wormhole 0.2.0-beta.1 Wormhole-0.2.0-beta.1-macos.app.tar.gz", // its prerelease
        "Wormhole 0.2.0 Wormhole-0.2.0-macos.dmg",        // another file of the release
        "timestamp:1700000000",                           // no release named at all
        "",
    ] {
        let p = publish(&key, &archive, archive.clone(), Some(key.sign_with_comment(&archive, comment)));
        let release = available(&p.config);
        let err = download(&p.config, &release, |_, _| {}).unwrap_err();
        assert!(matches!(err, UpdateError::Signature(ref m) if m.contains("0.2.0")), "{comment:?}: {err}");
    }
    // What package.sh writes is accepted.
    let p =
        publish(&key, &archive, archive.clone(), Some(key.sign_with_comment(&archive, TEST_TRUSTED_COMMENT)));
    assert!(download(&p.config, &available(&p.config), |_, _| {}).is_ok());
}

#[test]
fn size_limits_are_enforced() {
    let key = TestKey::generate();
    let archive = fake_app_archive(BUNDLE_ID, "0.2.0");

    let mut p = publish(&key, &archive, archive.clone(), None);
    let release = available(&p.config);
    p.config.max_download_size = 100;
    assert!(matches!(download(&p.config, &release, |_, _| {}), Err(UpdateError::TooLarge { limit: 100 })));

    // The server sends more than the feed promised.
    let bigger = archive.iter().copied().chain([0u8; 4096]).collect::<Vec<_>>();
    let p = publish(&key, &archive, bigger, None);
    let release = available(&p.config);
    assert!(matches!(download(&p.config, &release, |_, _| {}), Err(UpdateError::Network(_))));
}

#[test]
fn wrong_bundle_leaves_current_app_untouched() {
    let key = TestKey::generate();
    for (id, version) in [("com.evil.app", "0.2.0"), (BUNDLE_ID, "0.1.5")] {
        let archive = fake_app_archive(id, version);
        let p = publish(&key, &archive, archive.clone(), None);
        let update = download(&p.config, &available(&p.config), |_, _| {}).unwrap();
        let (dir, app) = temp_apps_dir();
        let err = install(&p.config, &update, &app).unwrap_err();
        assert!(matches!(err, UpdateError::Install(_)), "{err}");
        assert!(marker_of(&app).contains("old"));
        assert!(leftovers(dir.path()).is_empty());
    }
}

#[test]
fn an_update_under_the_identifier_the_app_moves_to_installs() {
    let key = TestKey::generate();
    let archive = fake_app_archive("dev.example.renamed", "0.2.0");
    let mut p = publish(&key, &archive, archive.clone(), None);
    p.config.also_accepted = vec!["dev.example.renamed".into()];
    let update = download(&p.config, &available(&p.config), |_, _| {}).unwrap();
    let (_dir, app) = temp_apps_dir();
    install(&p.config, &update, &app).unwrap();
    assert!(marker_of(&app).contains("new"));
}

#[test]
fn malicious_archive_is_rejected_through_install() {
    let key = TestKey::generate();
    let archive = raw_archive(&[
        RawEntry::Dir("Wormhole.app/"),
        RawEntry::File("Wormhole.app/../../escaped", b"pwned"),
    ]);
    let p = publish(&key, &archive, archive.clone(), None);
    let update = download(&p.config, &available(&p.config), |_, _| {}).unwrap();
    let (dir, app) = temp_apps_dir();
    let err = install(&p.config, &update, &app).unwrap_err();
    assert!(matches!(err, UpdateError::Install(ref m) if m.contains("unsafe path")), "{err}");
    assert!(marker_of(&app).contains("old"));
    assert!(!dir.path().join("escaped").exists() && !dir.path().parent().unwrap().join("escaped").exists());
    assert!(leftovers(dir.path()).is_empty());
}

fn extract_bytes(archive: &[u8]) -> (tempfile::TempDir, Result<std::path::PathBuf, UpdateError>) {
    let mut file = tempfile::tempfile().unwrap();
    file.write_all(archive).unwrap();
    let dest = tempfile::tempdir().unwrap();
    let result = extract(&file, &dest.path().join("out"));
    (dest, result)
}

#[test]
fn tar_traversal_and_odd_entries_are_rejected() {
    let cases: Vec<(&str, Vec<RawEntry<'_>>)> = vec![
        ("dotdot", vec![RawEntry::File("Wormhole.app/../x", b"x")]),
        ("absolute", vec![RawEntry::File("/tmp/wormhole-updater-test-abs", b"x")]),
        ("outside bundle", vec![RawEntry::File("evil.sh", b"x")]),
        ("two bundles", vec![RawEntry::Dir("A.app/"), RawEntry::Dir("B.app/")]),
        ("escaping symlink", vec![RawEntry::Dir("A.app/"), RawEntry::Symlink("A.app/l", "../../../etc")]),
        ("absolute symlink", vec![RawEntry::Dir("A.app/"), RawEntry::Symlink("A.app/l", "/etc")]),
        ("hard link", vec![RawEntry::File("A.app/f", b"x"), RawEntry::HardLink("A.app/h", "A.app/f")]),
        ("empty", vec![]),
    ];
    for (name, entries) in cases {
        let (_dest, result) = extract_bytes(&raw_archive(&entries));
        assert!(matches!(result, Err(UpdateError::Install(_))), "{name}: {result:?}");
    }
    assert!(!Path::new("/tmp/wormhole-updater-test-abs").exists());

    // Symlinks that stay inside the bundle and a leading "./" are fine.
    let ok = raw_archive(&[
        RawEntry::Dir("./A.app/"),
        RawEntry::Dir("A.app/Contents/"),
        RawEntry::Symlink("A.app/link", "Contents"),
        RawEntry::File("A.app/Contents/f", b"x"),
    ]);
    let (dest, result) = extract_bytes(&ok);
    assert_eq!(result.unwrap(), dest.path().join("out/A.app"));
}

#[test]
fn failed_swap_rolls_back() {
    let (dir, app) = temp_apps_dir();
    let backup = dir.path().join(".backup.app");
    let missing_new = dir.path().join("does-not-exist.app");
    let err = swap_into_place(&app, &missing_new, &backup).unwrap_err();
    assert!(matches!(err, UpdateError::Install(ref m) if m.contains("nothing was changed")), "{err}");
    assert!(marker_of(&app).contains("old"));
    assert!(!backup.exists());

    // And the happy path moves the old one to the backup location.
    let new = dir.path().join("new.app");
    write_fake_app(&new, BUNDLE_ID, "0.2.0", "new");
    swap_into_place(&app, &new, &backup).unwrap();
    assert!(marker_of(&app).contains("new"));
    assert!(marker_of(&backup).contains("old"));
}

#[test]
fn unwritable_location_is_reported() {
    let (dir, app) = temp_apps_dir();
    fs::set_permissions(dir.path(), crate::platform::fs::permissions(0o555)).unwrap();
    // root ignores directory permissions (cloud and container CI), so there is nothing to report.
    if fs::write(dir.path().join(".probe"), b"").is_ok() {
        fs::set_permissions(dir.path(), crate::platform::fs::permissions(0o755)).unwrap();
        return;
    }
    let result = preflight(&app);
    fs::set_permissions(dir.path(), crate::platform::fs::permissions(0o755)).unwrap();
    assert!(
        matches!(result, Err(UpdateError::NotWritable(ref p)) if p.ends_with(dir.path().file_name().unwrap()))
    );

    // A bundle directory we can't modify (like a root-owned install) is also refused.
    let contents = app.join("Contents");
    fs::set_permissions(&contents, crate::platform::fs::permissions(0o555)).unwrap();
    let result = preflight(&app);
    fs::set_permissions(&contents, crate::platform::fs::permissions(0o755)).unwrap();
    assert!(matches!(result, Err(UpdateError::NotWritable(_))), "{result:?}");
    assert!(leftovers(dir.path()).is_empty());
}

#[test]
fn translocated_and_unbundled_paths_are_refused() {
    let translocated = Path::new("/private/var/folders/x/T/AppTranslocation/ABC/d/Wormhole.app");
    let err = preflight(translocated).unwrap_err();
    assert!(matches!(err, UpdateError::Translocated(_)));
    assert!(err.to_string().contains("Applications folder"));

    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(preflight(dir.path()), Err(UpdateError::NotBundled)));
}

#[test]
fn a_symlink_made_through_another_symlink_cannot_point_out_of_the_bundle() {
    // `up -> ..` is harmless alone. But `up/esc` is then created one level higher than its path
    // says (at A.app/Contents/esc), so its target `../../../outside`, which the lexical check
    // reads as "inside", really leaves the extraction directory.
    let archive = raw_archive(&[
        RawEntry::Dir("A.app/"),
        RawEntry::Dir("A.app/Contents/"),
        RawEntry::Dir("A.app/Contents/MacOS/"),
        RawEntry::Symlink("A.app/Contents/MacOS/up", ".."),
        RawEntry::Symlink("A.app/Contents/MacOS/up/esc", "../../../outside"),
    ]);
    let (dest, result) = extract_bytes(&archive);
    assert!(
        matches!(result, Err(UpdateError::Install(ref m)) if m.contains("through another symlink")),
        "{result:?}"
    );
    assert!(fs::symlink_metadata(dest.path().join("out/A.app/Contents/esc")).is_err(), "it was created");
}

#[test]
fn setuid_bits_and_special_files_do_not_survive_extraction() {
    let archive = raw_archive(&[
        RawEntry::DirMode("A.app/", 0o2755),
        RawEntry::FileMode("A.app/suid", b"x", 0o4755),
        RawEntry::FileMode("A.app/sgid", b"x", 0o2755),
        RawEntry::DirMode("A.app/sticky/", 0o1777),
    ]);
    let (dest, result) = extract_bytes(&archive);
    let app = result.unwrap();
    assert_eq!(app, dest.path().join("out/A.app"));
    for path in ["", "suid", "sgid", "sticky"] {
        let mode = crate::platform::fs::mode(&fs::metadata(app.join(path)).unwrap());
        assert_eq!(mode & 0o7000, 0, "{path:?} kept special bits: {mode:o}");
    }
    assert_eq!(crate::platform::fs::mode(&fs::metadata(app.join("suid")).unwrap()) & 0o777, 0o755);

    for (name, kind) in
        [("fifo", tar::EntryType::Fifo), ("char", tar::EntryType::Char), ("block", tar::EntryType::Block)]
    {
        let archive = raw_archive(&[RawEntry::Dir("A.app/"), RawEntry::Special("A.app/dev", kind)]);
        let (_dest, result) = extract_bytes(&archive);
        assert!(
            matches!(result, Err(UpdateError::Install(ref m)) if m.contains("unsupported")),
            "{name}: {result:?}"
        );
    }
}
