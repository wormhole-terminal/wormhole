use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::*;
use crate::platform::fs::{mode, set_mode};
use crate::probe::scan::scan_data;
use crate::probe::validate::Confirmation;
use crate::storage::paths::DataRoot;

const NOW: u64 = 1_800_000_000;

fn manifest(id: &str, permissions: &str, extra: &str) -> String {
    format!(
        "[probe]\nid = \"{id}\"\nname = \"Acme tools\"\ndescription = \"d\"\nauthor = \"a\"\n\
         version = \"0.3.0\"\nwormhole = \">=1.3\"\napi = 1\n\n[permissions]\n{permissions}\n{extra}"
    )
}

const DEPLOY: &str = "[[contributes.commands]]\nid = \"deploy\"\ntitle = \"Deploy\"\n\
    run = [\"./scripts/deploy.sh\"]\nin = \"new-terminal\"\ncwd = \"terminal\"\n";

fn install(root: &DataRoot, id: &str, permissions: &str) {
    let dir = root.probes_dir().join(id);
    fs::create_dir_all(dir.join("scripts")).expect("mkdir");
    fs::write(dir.join("probe.toml"), manifest(id, permissions, DEPLOY)).expect("manifest");
    fs::write(dir.join("scripts/deploy.sh"), "#!/bin/sh\necho deploy\n").expect("script");
    set_mode(&dir.join("scripts/deploy.sh"), 0o755).expect("chmod");
}

/// The scratch folder; its copies are read-only, so they are made deletable before it goes.
struct Scratch {
    approved: PathBuf,
    _tmp: TempDir,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = snapshot::remove_tree(&self.approved);
    }
}

fn setup() -> (Scratch, DataRoot) {
    let tmp = TempDir::new().expect("tempdir");
    let root = DataRoot::new(tmp.path().join("data"));
    fs::create_dir_all(root.base()).expect("data dir");
    (Scratch { approved: root.probes_approved_dir(), _tmp: tmp }, root)
}

fn request(root: &DataRoot, id: &str) -> Request {
    read_one(root, id).expect("a request")
}

fn state(root: &DataRoot, id: &str) -> ProbeState {
    list(root).expect("lists").into_iter().find(|e| e.folder == id).expect("listed").state
}

const ACME: &str = "process = [\"./scripts/deploy.sh\"]\nnotify = true\n";

fn approve_all(root: &DataRoot, id: &str) -> Request {
    let request = request(root, id);
    approve(root, &request, &request.initial_grants(), NOW).expect("approves");
    request
}

#[test]
fn a_valid_probe_is_off_until_approved_then_on() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    assert_eq!(state(&root, "acme-tools"), ProbeState::Off);
    approve_all(&root, "acme-tools");
    assert_eq!(state(&root, "acme-tools"), ProbeState::On);
}

#[test]
fn an_edit_after_approval_makes_it_changed_and_approving_again_makes_it_on() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    let script = root.probes_dir().join("acme-tools/scripts/deploy.sh");
    fs::write(&script, "#!/bin/sh\necho changed\n").expect("edit");
    assert_eq!(state(&root, "acme-tools"), ProbeState::Changed);
    assert!(!state(&root, "acme-tools").is_on(), "a changed Probe is off");

    approve_all(&root, "acme-tools");
    assert_eq!(state(&root, "acme-tools"), ProbeState::On);
}

#[test]
fn an_added_file_changes_it_too_and_a_finder_file_does_not() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    fs::write(root.probes_dir().join("acme-tools/.DS_Store"), "x").expect("finder");
    assert_eq!(state(&root, "acme-tools"), ProbeState::On);
    fs::write(root.probes_dir().join("acme-tools/more.txt"), "x").expect("add");
    assert_eq!(state(&root, "acme-tools"), ProbeState::Changed);
}

#[test]
fn what_cannot_be_enabled_says_why() {
    let (_tmp, root) = setup();
    let broken = root.probes_dir().join("broken");
    fs::create_dir_all(&broken).expect("mkdir");
    fs::write(broken.join("probe.toml"), "[probe\n").expect("write");
    let missing_script = root.probes_dir().join("no-script");
    fs::create_dir_all(&missing_script).expect("mkdir");
    fs::write(missing_script.join("probe.toml"), manifest("no-script", ACME, DEPLOY)).expect("write");
    let coded = root.probes_dir().join("coded");
    fs::create_dir_all(coded.join("src")).expect("mkdir");
    fs::write(
        coded.join("probe.toml"),
        manifest("coded", "", "").replace("api = 1\n", "api = 1\ncode_api = 0\nmain = \"src/index.ts\"\n"),
    )
    .expect("write");
    fs::write(coded.join("src/index.ts"), "export {}\n").expect("write");

    let rows = list(&root).expect("lists");
    let by = |folder: &str| rows.iter().find(|e| e.folder == folder).expect(folder);
    assert!(matches!(&by("broken").state, ProbeState::Invalid(reason) if reason.contains("probe.toml")));
    assert!(by("broken").probe.is_none());
    assert!(matches!(&by("no-script").state, ProbeState::Invalid(reason) if reason.contains("deploy.sh")));
    assert!(by("no-script").probe.is_some());
    assert_eq!(by("coded").state, ProbeState::Unsupported);
    assert!(matches!(read_one(&root, "coded"), Err(ReadOneError::NotApprovable(NotApprovable::NeedsCode))));
    assert!(matches!(
        read_one(&root, "no-script"),
        Err(ReadOneError::NotApprovable(NotApprovable::Invalid(_)))
    ));
}

#[test]
fn a_snapshot_is_a_read_only_copy_that_verifies() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    let store = ApprovalStore::load(&root);
    let snapshot = store.snapshot(&root, "acme-tools").expect("approved");
    assert_eq!(snapshot.dir(), root.probes_approved_dir().join("acme-tools"));
    snapshot.verify().expect("the copy is what was approved");

    let bits = |p: &str| mode(&fs::metadata(snapshot.dir().join(p)).expect(p)) & 0o777;
    assert_eq!(bits("probe.toml"), 0o400);
    assert_eq!(bits("scripts/deploy.sh"), 0o500, "a script that was executable stays so");
    assert_eq!(bits("scripts"), 0o500);
    assert_eq!(bits(""), 0o500);
    assert!(fs::write(snapshot.dir().join("probe.toml"), "x").is_err(), "read-only");
}

#[test]
fn the_copy_is_made_from_the_bytes_that_were_shown_not_from_the_disk() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    let request = request(&root, "acme-tools");
    // Edited while the sheet was open.
    fs::write(root.probes_dir().join("acme-tools/scripts/deploy.sh"), "#!/bin/sh\nevil\n").expect("edit");
    approve(&root, &request, &request.initial_grants(), NOW).expect("approves");

    let snapshot = ApprovalStore::load(&root).snapshot(&root, "acme-tools").expect("approved");
    snapshot.verify().expect("the copy matches what was shown");
    let copy = fs::read_to_string(snapshot.dir().join("scripts/deploy.sh")).expect("read");
    assert!(copy.contains("echo deploy"), "{copy}");
    assert_eq!(state(&root, "acme-tools"), ProbeState::Changed, "and the edit is seen as one");
}

#[test]
fn a_tampered_snapshot_does_not_verify() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    let snapshot = ApprovalStore::load(&root).snapshot(&root, "acme-tools").expect("approved");
    let script = snapshot.dir().join("scripts/deploy.sh");

    // Same user, so the modes can be undone; the hash still catches the edit.
    set_mode(&snapshot.dir().join("scripts"), 0o700).expect("chmod");
    set_mode(&script, 0o700).expect("chmod");
    fs::write(&script, "#!/bin/sh\nevil\n").expect("tamper");
    assert!(matches!(snapshot.verify(), Err(VerifyError::Differs)));
    assert_eq!(state(&root, "acme-tools"), ProbeState::On, "the live folder is still the approved one");

    fs::write(&script, "#!/bin/sh\necho deploy\n").expect("restore");
    snapshot.verify().expect("restored");
    set_mode(snapshot.dir(), 0o700).expect("chmod");
    fs::write(snapshot.dir().join("planted"), "x").expect("plant");
    assert!(matches!(snapshot.verify(), Err(VerifyError::Differs)), "an added file counts");
}

#[test]
fn a_missing_snapshot_does_not_verify() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    let snapshot = ApprovalStore::load(&root).snapshot(&root, "acme-tools").expect("approved");
    snapshot::remove(&root, "acme-tools").expect("removed");
    assert!(matches!(snapshot.verify(), Err(VerifyError::Missing)));
}

#[test]
fn turning_off_removes_the_approval_and_the_copy_and_only_that_probes() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    install(&root, "other-one", ACME);
    approve_all(&root, "acme-tools");
    approve_all(&root, "other-one");

    turn_off(&root, "acme-tools").expect("off");
    assert_eq!(state(&root, "acme-tools"), ProbeState::Off);
    assert!(!root.probes_approved_dir().join("acme-tools").exists());
    assert!(ApprovalStore::load(&root).get("acme-tools").is_none());
    assert_eq!(state(&root, "other-one"), ProbeState::On);
    ApprovalStore::load(&root).snapshot(&root, "other-one").expect("kept").verify().expect("intact");
    assert!(root.probes_dir().join("acme-tools/probe.toml").exists(), "the Probe's own folder stays");
    turn_off(&root, "acme-tools").expect("off twice is fine");
}

#[test]
fn removing_a_probe_removes_its_approval_and_copy_and_leftovers() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    install(&root, "other-one", ACME);
    approve_all(&root, "acme-tools");
    approve_all(&root, "other-one");
    // What an interrupted approval leaves.
    let leftover = root.probes_approved_dir().join(".new-ghost/deep");
    fs::create_dir_all(&leftover).expect("mkdir");
    fs::write(leftover.join("f"), "x").expect("write");
    set_mode(&leftover, 0o500).expect("chmod");

    fs::remove_dir_all(root.probes_dir().join("acme-tools")).expect("the user deletes the folder");
    let present: BTreeSet<String> = scan_data(&root).expect("scans").into_iter().map(|s| s.name).collect();
    sweep(&root, &present).expect("sweeps");

    let kept: Vec<_> = fs::read_dir(root.probes_approved_dir())
        .expect("dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(kept, ["other-one"]);
    assert!(ApprovalStore::load(&root).get("acme-tools").is_none());
    assert_eq!(state(&root, "other-one"), ProbeState::On);
}

#[test]
fn approving_again_replaces_the_copy_in_one_swap() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    fs::write(root.probes_dir().join("acme-tools/scripts/deploy.sh"), "#!/bin/sh\necho v2\n").expect("edit");
    approve_all(&root, "acme-tools");

    let names: Vec<_> = fs::read_dir(root.probes_approved_dir())
        .expect("dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["acme-tools"], "no staging or previous copy is left");
    let snapshot = ApprovalStore::load(&root).snapshot(&root, "acme-tools").expect("approved");
    snapshot.verify().expect("the new copy");
    assert!(fs::read_to_string(snapshot.dir().join("scripts/deploy.sh")).expect("read").contains("v2"));
}

#[test]
fn the_grants_are_kept_and_a_refused_permission_stays_refused() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    let first = request(&root, "acme-tools");
    let notify = first.summary().permissions.iter().find(|p| p.key == "notify").expect("notify").clone();
    let mut grants = first.initial_grants();
    assert!(grants.is_granted(&notify), "on by default");
    grants.set(&notify, false);
    approve(&root, &first, &grants, NOW).expect("approves");

    let store = ApprovalStore::load(&root);
    let approval = store.get("acme-tools").expect("approved");
    assert!(!approval.grants(&notify));
    let script = first.summary().permissions.iter().find(|p| p.key == "process").expect("process");
    assert!(approval.grants(script));
    assert_eq!(approval.approved_at, NOW);
    assert_eq!(approval.version, "0.3.0");
    assert_eq!(approval.declared.len(), 2);
    assert_eq!(approval.granted, ["process:./scripts/deploy.sh"]);

    // After a change, the sheet starts with the earlier refusal still in place.
    fs::write(root.probes_dir().join("acme-tools/scripts/deploy.sh"), "#!/bin/sh\necho v2\n").expect("edit");
    let again = request(&root, "acme-tools");
    assert!(!again.initial_grants().is_granted(&notify));
    assert!(again.changes().expect("was approved").files_only());
}

#[test]
fn what_changed_names_the_permissions_that_came_or_went() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    let manifest_path = root.probes_dir().join("acme-tools/probe.toml");
    fs::write(
        &manifest_path,
        manifest("acme-tools", "process = [\"./scripts/deploy.sh\"]\nopen = [\"acme.net\"]\n", DEPLOY),
    )
    .expect("edit");
    let request = request(&root, "acme-tools");
    let changes = request.changes().expect("was approved");
    assert!(!changes.files_only());
    assert_eq!(changes.added.len(), 1);
    assert_eq!(changes.added[0].key, "open");
    assert_eq!(changes.removed, ["notify:show notifications"]);
    // A new permission is on in the sheet, as every one is.
    assert!(request.initial_grants().is_granted(&changes.added[0]));
}

#[test]
fn a_click_is_enough_only_for_permissions_that_harm_little() {
    let (_tmp, root) = setup();
    let dir = root.probes_dir().join("quiet");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("probe.toml"), manifest("quiet", "notify = true\nopen = [\"acme.net\"]\n", ""))
        .expect("write");
    let quiet = request(&root, "quiet");
    assert_eq!(quiet.confirmation(&quiet.initial_grants()), Confirmation::Click);

    install(&root, "acme-tools", ACME);
    let acme = request(&root, "acme-tools");
    let mut grants = acme.initial_grants();
    assert_eq!(acme.confirmation(&grants), Confirmation::TouchId, "a program is Touch ID");
    let program = acme.summary().permissions.iter().find(|p| p.key == "process").expect("process");
    grants.set(program, false);
    assert_eq!(acme.confirmation(&grants), Confirmation::Click, "switched off, only notify is left");
}

#[test]
fn a_dangerous_combination_needs_touch_id_even_when_each_part_is_a_click() {
    let (_tmp, root) = setup();
    // `terminal` and `net` are E2; a secret handed to a program is the E1 combination.
    install(&root, "acme-tools", "process = [\"./scripts/deploy.sh\"]\nenv = [\"ACME_TOKEN\"]\n");
    let request = request(&root, "acme-tools");
    assert!(!request.summary().combinations.is_empty(), "env with process is a combination");
    let mut grants = request.initial_grants();
    for p in &request.summary().permissions {
        grants.set(p, false);
    }
    assert_eq!(request.confirmation(&grants), Confirmation::TouchId);
}

#[test]
fn a_store_that_is_too_big_or_corrupt_reads_as_nothing_approved() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    let file = root.probe_approvals_file();

    let big = ApprovalStore::load_capped(file.clone(), 16);
    assert!(big.is_empty() && big.is_damaged());

    fs::write(&file, "{ not json").expect("corrupt");
    let store = ApprovalStore::load(&root);
    assert!(store.is_empty() && store.is_damaged());
    assert_eq!(state(&root, "acme-tools"), ProbeState::Off, "so the Probe is off");

    // The broken file is kept aside by the next save, not lost.
    approve_all(&root, "acme-tools");
    assert_eq!(state(&root, "acme-tools"), ProbeState::On);
    let aside = file.with_file_name("probe-approvals.json.bad");
    assert_eq!(fs::read_to_string(aside).expect("kept"), "{ not json");
}

#[test]
fn a_store_of_another_version_or_with_odd_keys_is_not_trusted() {
    let (_tmp, root) = setup();
    let file = root.probe_approvals_file();
    fs::write(&file, r#"{"version": 2, "probes": {}}"#).expect("write");
    assert!(ApprovalStore::load(&root).is_damaged());
    let entry = r#"{"fingerprint":"f","version":"1","declared":[],"granted":[],"approved_at":1}"#;
    fs::write(&file, format!(r#"{{"version": 1, "probes": {{"../escape": {entry}, "fine-one": {entry}}}}}"#))
        .expect("write");
    let store = ApprovalStore::load(&root);
    assert_eq!(store.ids().collect::<Vec<_>>(), ["fine-one"], "a key is an id or it is dropped");
    assert!(store.snapshot(&root, "../escape").is_none());
}

#[test]
fn the_approvals_file_is_owner_only() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    let bits = |p: &Path| mode(&fs::metadata(p).expect("meta")) & 0o777;
    assert_eq!(bits(&root.probe_approvals_file()), 0o600);
    assert_eq!(bits(&root.probes_approved_dir()), 0o700);
}

#[test]
fn a_probe_id_that_is_no_id_names_no_folder() {
    let (_tmp, root) = setup();
    fs::create_dir_all(root.probes_approved_dir().join("victim")).expect("mkdir");
    turn_off(&root, "../probes-approved/victim").expect("nothing to do");
    turn_off(&root, "").expect("nothing to do");
    assert!(root.probes_approved_dir().join("victim").exists());
    assert!(matches!(read_one(&root, ".."), Err(ReadOneError::Unreadable(_))));
}
