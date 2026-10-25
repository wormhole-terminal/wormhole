//! Probes from real folders: the reference Probe of `repo-docs/docs/probes.md` section 2 validates
//! clean, and every folder under `probes/errors/` is a Probe that has exactly the one finding its
//! name says (`probes/warnings/` the same, as a warning that leaves the Probe usable).
//!
//! The fixtures are only read. Anything these tests write goes to a temp dir.

use std::fs;
use std::path::{Path, PathBuf};

use wormhole_core::probe::scan::{ScanError, scan};
use wormhole_core::probe::validate::{Confirmation, ProgramClass};
use wormhole_core::probe::{ProbeFolder, Severity, validate};

fn fixtures(kind: &str) -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes").join(kind);
    let mut found: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("{}: {err}", dir.display()))
        .map(|entry| entry.expect("entry").path())
        .collect();
    found.sort();
    found
}

fn name(dir: &Path) -> &str {
    dir.file_name().and_then(|n| n.to_str()).expect("name")
}

fn read(dir: &Path) -> ProbeFolder {
    ProbeFolder::read(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()))
}

#[test]
fn the_reference_probe_validates_with_nothing_to_say() {
    let folder = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes/ok/acme-tools"));
    let report = validate(&folder);
    assert_eq!(report.findings, [], "{:#?}", report.findings);
    assert!(report.is_ok());
}

#[test]
fn the_reference_probe_s_summary_lists_what_it_would_be_allowed_to_do() {
    let folder = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes/ok/acme-tools"));
    let summary = validate(&folder).summary;
    let listed: Vec<_> =
        summary.permissions.iter().map(|p| (p.key, p.detail.as_str(), p.confirmation)).collect();
    assert_eq!(
        listed,
        [
            ("process", "git", Confirmation::TouchId),
            ("process", "./scripts/deploy.sh", Confirmation::TouchId),
            ("env", "ACME_TOKEN", Confirmation::TouchId),
            ("open", "acme.atlassian.net", Confirmation::Click),
            ("notify", "show notifications", Confirmation::Click),
        ]
    );
    let classes: Vec<_> = summary.programs.iter().map(|p| p.class).collect();
    assert_eq!(classes, [ProgramClass::ReadOnlyGit, ProgramClass::OwnScript]);
    assert_eq!(summary.combinations.len(), 1, "the token can reach the programs");
    assert!(!summary.has_code);
}

#[test]
fn the_reference_probe_s_fingerprint_is_stable() {
    // The files as committed. If this changes, a file of the fixture did, or the fingerprint's
    // encoding did, and every stored Probe approval would change with it.
    let folder = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes/ok/acme-tools"));
    assert_eq!(folder.fingerprint(), "a358fe3d2b018b4f83ab499cc9344eb7ddba7e80a68f9919e042ee28d1db8dfe");
}

#[test]
fn every_ok_fixture_has_no_findings() {
    for dir in fixtures("ok") {
        let report = validate(&read(&dir));
        assert_eq!(report.findings, [], "{}", name(&dir));
    }
}

#[test]
fn every_error_fixture_has_the_error_its_name_says_and_is_not_ok() {
    let all = fixtures("errors");
    assert!(all.len() >= 30, "the fixtures are there");
    for dir in all {
        let code = name(&dir);
        let report = validate(&read(&dir));
        let finding = report
            .findings
            .iter()
            .find(|f| f.code == code)
            .unwrap_or_else(|| panic!("{code}: found {:#?}", report.findings));
        assert_eq!(finding.severity, Severity::Error, "{code}");
        assert!(!report.is_ok(), "{code}");
        assert!(!finding.message.is_empty());
        // Findings read as one line for a person or a log.
        assert!(!finding.to_string().contains('\n'), "{code}: {finding}");
    }
}

#[test]
fn every_error_fixture_has_only_the_error_its_name_says() {
    // A fixture that trips an unrelated check would pass the test above by accident.
    for dir in fixtures("errors") {
        let code = name(&dir);
        let report = validate(&read(&dir));
        let others: Vec<_> = report.errors().filter(|f| f.code != code).map(|f| f.code).collect();
        assert_eq!(others, Vec::<&str>::new(), "{code}");
    }
}

#[test]
fn every_warning_fixture_warns_and_is_still_ok() {
    for dir in fixtures("warnings") {
        let code = name(&dir);
        let report = validate(&read(&dir));
        let finding = report
            .warnings()
            .find(|f| f.code == code)
            .unwrap_or_else(|| panic!("{code}: {:#?}", report.findings));
        assert_eq!(finding.severity, Severity::Warning);
        assert!(report.is_ok(), "{code}: {:#?}", report.findings);
    }
}

#[test]
fn scanning_a_folder_of_probes_lists_each_with_its_probe_or_its_reason() {
    let found = scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes/ok")).expect("scans");
    let names: Vec<_> = found.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["acme-tools", "minimal", "rules-only"]);
    assert!(found.iter().all(|s| s.probe.is_ok()));
    // The error fixtures are named by a code, not by the id inside: each is listed with that reason.
    let found = scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes/errors")).expect("scans");
    assert!(found.len() >= 30);
    for scanned in found {
        assert!(matches!(&scanned.probe, Err(ScanError::IdMismatch { .. })), "{}", scanned.name);
    }
}
