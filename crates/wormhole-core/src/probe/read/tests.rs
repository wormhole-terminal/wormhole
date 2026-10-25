use std::fs;

use tempfile::TempDir;

use super::*;
use crate::platform::fs::symlink;

const MANIFEST: &str = "[probe]\nid = \"tiny\"\nname = \"Tiny\"\ndescription = \"d\"\nauthor = \"a\"\n\
                        version = \"0.1.0\"\nwormhole = \">=1.3\"\napi = 1\n";

/// A scratch folder holding a Probe with these files besides a valid manifest.
fn probe(files: &[(&str, &[u8])]) -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    put(tmp.path(), MANIFEST_FILE, MANIFEST.as_bytes());
    for (path, bytes) in files {
        put(tmp.path(), path, bytes);
    }
    tmp
}

fn put(root: &Path, path: &str, bytes: &[u8]) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().expect("parent")).expect("mkdir");
    fs::write(target, bytes).expect("write");
}

fn read(tmp: &TempDir) -> Result<ProbeFolder, ReadError> {
    ProbeFolder::read(tmp.path())
}

#[test]
fn a_folder_is_its_manifest_and_every_file_sorted_by_path() {
    let tmp = probe(&[("scripts/run.sh", b"echo"), ("assets/a.svg", b"<svg/>"), ("zz", b"")]);
    let folder = read(&tmp).expect("reads");
    let paths: Vec<_> = folder.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["assets/a.svg", "probe.toml", "scripts/run.sh", "zz"]);
    assert_eq!(folder.manifest.probe.id, "tiny");
    assert_eq!(folder.file("scripts/run.sh").map(ProbeFile::size), Some(4));
    assert_eq!(folder.file("nope"), None);
}

#[test]
fn finder_s_ds_store_is_not_part_of_the_probe() {
    let plain = read(&probe(&[])).expect("reads");
    let with = read(&probe(&[(".DS_Store", b"junk"), ("assets/.DS_Store", b"junk")])).expect("reads");
    assert_eq!(plain.fingerprint(), with.fingerprint());
}

#[test]
fn the_reference_probe_reads_and_its_fingerprint_follows_its_files() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes/ok/acme-tools");
    let folder = ProbeFolder::read(&dir).expect("reads");
    assert_eq!(folder.warnings, []);
    assert_eq!(folder.files.len(), 3);
    assert_eq!(folder.fingerprint().len(), 64);
}

#[test]
fn no_manifest_a_bad_manifest_and_a_file_are_errors() {
    let tmp = TempDir::new().expect("tempdir");
    assert!(matches!(ProbeFolder::read(tmp.path()), Err(ReadError::NoManifest)));
    put(tmp.path(), MANIFEST_FILE, b"[probe]\n");
    assert!(matches!(ProbeFolder::read(tmp.path()), Err(ReadError::Manifest(_))));
    put(tmp.path(), MANIFEST_FILE, &[0xff, 0xfe]);
    assert!(matches!(ProbeFolder::read(tmp.path()), Err(ReadError::Manifest(_))), "not UTF-8");
    assert!(matches!(ProbeFolder::read(&tmp.path().join(MANIFEST_FILE)), Err(ReadError::NotAFolder(_))));
    assert!(matches!(ProbeFolder::read(&tmp.path().join("absent")), Err(ReadError::Io { .. })));
}

#[test]
fn at_most_200_files() {
    let tmp = probe(&[]);
    for ix in 0..MAX_FILES - 1 {
        put(tmp.path(), &format!("d/{ix}"), b"");
    }
    assert_eq!(read(&tmp).expect("200 files").files.len(), MAX_FILES);
    put(tmp.path(), "one-more", b"");
    assert!(matches!(read(&tmp), Err(ReadError::TooManyFiles)));
}

#[test]
fn folders_count_against_the_entries_cap_as_files_do() {
    let tmp = probe(&[]);
    // Empty folders are no files, so only the entries cap stops a folder made of them.
    for ix in 0..MAX_ENTRIES {
        fs::create_dir(tmp.path().join(format!("d{ix}"))).expect("mkdir");
    }
    assert!(matches!(read(&tmp), Err(ReadError::TooManyEntries)));
}

#[test]
fn at_most_2_mib_in_all_and_64_kib_of_manifest() {
    let half = vec![b'x'; (MAX_TOTAL_BYTES / 2) as usize];
    let tmp = probe(&[("a", &half), ("b", &half[..half.len() - MANIFEST.len()])]);
    assert!(read(&tmp).is_ok(), "exactly at the cap");
    put(tmp.path(), "c", b"x");
    assert!(matches!(read(&tmp), Err(ReadError::TooMuch)));

    let tmp = TempDir::new().expect("tempdir");
    let mut big = MANIFEST.to_string();
    big.push_str(&"#".repeat(MAX_MANIFEST_BYTES as usize));
    put(tmp.path(), MANIFEST_FILE, big.as_bytes());
    assert!(matches!(ProbeFolder::read(tmp.path()), Err(ReadError::Refused { .. })));
}

#[test]
fn folders_are_at_most_eight_deep() {
    let tmp = probe(&[("a/b/c/d/e/f/g/h/ok", b"")]);
    assert!(read(&tmp).is_ok());
    put(tmp.path(), "a/b/c/d/e/f/g/h/i/too-deep", b"");
    assert!(matches!(read(&tmp), Err(ReadError::TooDeep(_))));
}

#[test]
fn a_link_out_of_the_folder_is_an_error() {
    let outside = TempDir::new().expect("tempdir");
    put(outside.path(), "secret", b"hunter2");
    let tmp = probe(&[]);
    symlink(outside.path().join("secret"), tmp.path().join("peek")).expect("link");
    assert!(matches!(read(&tmp), Err(ReadError::LeavesFolder(path)) if path == "peek"));
}

#[test]
fn a_link_to_a_file_inside_is_read_and_a_link_to_a_folder_is_not_followed() {
    let tmp = probe(&[("real", b"data")]);
    symlink("real", tmp.path().join("alias")).expect("link");
    assert_eq!(read(&tmp).expect("reads").file("alias").map(ProbeFile::size), Some(4));

    fs::create_dir(tmp.path().join("dir")).expect("mkdir");
    symlink("dir", tmp.path().join("dir-alias")).expect("link");
    assert!(matches!(read(&tmp), Err(ReadError::FolderLink(path)) if path == "dir-alias"));
}

#[test]
fn a_broken_link_is_an_error_not_a_skipped_file() {
    let tmp = probe(&[]);
    symlink("nowhere", tmp.path().join("dangling")).expect("link");
    assert!(matches!(read(&tmp), Err(ReadError::Io { .. })));
}

#[test]
fn a_name_with_a_control_character_is_refused() {
    let tmp = probe(&[("a\nb", b"")]);
    assert!(matches!(read(&tmp), Err(ReadError::BadName(_))));
}

#[test]
fn the_errors_say_what_and_where() {
    let text = ReadError::LeavesFolder("peek".into()).to_string();
    assert!(text.contains("peek") && text.contains("out of"), "{text}");
    let text = ReadError::Manifest(ManifestError { message: "boom".into(), line: Some(2), column: Some(3) });
    assert_eq!(text.to_string(), "probe.toml:2:3: boom");
}
