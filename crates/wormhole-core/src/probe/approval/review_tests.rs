//! What the review of E1.0c asked for: a link where a folder should be, a scan that did not see everything, the one
//! gate before a Probe runs, an approval that is all or nothing, and the confirmation core asks for.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use super::tests::{ACME, NOW, Scratch, approve_all, install, request, setup, state};
use super::*;
use crate::platform::fs::{mode, set_mode};
use crate::probe::scan::Coverage;
use crate::storage::paths::DataRoot;

/// A folder of the user's own, next to the data folder, that no Probe call may touch.
fn precious(scratch: &Scratch) -> PathBuf {
    let dir = scratch._tmp.path().join("precious");
    fs::create_dir_all(dir.join("acme-tools/inner")).expect("mkdir");
    fs::write(dir.join("keep.txt"), "keep").expect("write");
    fs::write(dir.join("acme-tools/inner/keep.txt"), "keep").expect("write");
    dir
}

#[test]
fn a_linked_probes_approved_folder_is_never_followed_or_emptied() {
    let (scratch, root) = setup();
    let dir = precious(&scratch);
    crate::platform::fs::symlink(&dir, root.probes_approved_dir()).expect("link");
    let before = mode(&fs::metadata(dir.join("acme-tools")).expect("meta"));

    assert!(sweep(&root, Some(&BTreeSet::new())).is_err(), "sweep refuses a link");
    assert!(sweep(&root, None).is_err());
    assert!(turn_off(&root, "acme-tools").is_err(), "so does turning a Probe off");
    assert_eq!(mode(&fs::metadata(dir.join("acme-tools")).expect("meta")), before, "no chmod through it");
    assert_eq!(fs::read_to_string(dir.join("keep.txt")).expect("kept"), "keep");
    assert_eq!(fs::read_to_string(dir.join("acme-tools/inner/keep.txt")).expect("kept"), "keep");

    install(&root, "acme-tools", ACME);
    let request = request(&root, "acme-tools");
    let confirmed = request.confirm(&request.initial_grants(), Proof::TouchId).expect("a yes");
    assert!(approve(&root, &request, &confirmed, NOW).is_err(), "nor is a copy made through it");
    assert!(!dir.join(".new-acme-tools").exists());
    let _ = fs::remove_file(root.probes_approved_dir());
}

#[test]
fn a_link_inside_a_copy_is_removed_not_followed() {
    let (scratch, root) = setup();
    let dir = precious(&scratch);
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    // Same user, so the copy can be tampered with: a link to a folder of the user's own.
    let copy = root.probes_approved_dir().join("acme-tools");
    set_mode(&copy, 0o700).expect("chmod");
    crate::platform::fs::symlink(&dir, copy.join("elsewhere")).expect("link");
    turn_off(&root, "acme-tools").expect("off");
    assert!(!copy.exists());
    assert_eq!(fs::read_to_string(dir.join("keep.txt")).expect("kept"), "keep");
    assert_eq!(fs::read_to_string(dir.join("acme-tools/inner/keep.txt")).expect("kept"), "keep");
}

#[test]
fn a_folder_that_cannot_be_read_forgets_nothing() {
    let (_scratch, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    let listing = list(&root).expect("lists");
    assert_eq!(listing.coverage, Coverage::Whole);
    assert!(listing.present().is_some());

    set_mode(&root.probes_dir(), 0o000).expect("chmod");
    let listing = list(&root).expect("lists");
    set_mode(&root.probes_dir(), 0o700).expect("chmod back");
    assert_eq!(listing.coverage, Coverage::Partial, "unreadable is not empty");
    assert!(listing.present().is_none(), "so it names nobody as gone");
    sweep(&root, listing.present().as_ref()).expect("sweeps leftovers only");

    assert!(ApprovalStore::load(&root).get("acme-tools").is_some());
    assert!(root.probes_approved_dir().join("acme-tools").exists());
    assert_eq!(state(&root, "acme-tools"), ProbeState::On);
}

#[test]
fn a_missing_probes_folder_forgets_no_approval_but_tidies_leftovers() {
    let (_scratch, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    fs::create_dir_all(root.probes_approved_dir().join(".new-ghost")).expect("leftover");
    fs::create_dir_all(root.probes_approved_dir().join("orphan")).expect("a copy nothing owns");
    fs::remove_dir_all(root.probes_dir()).expect("the user moves the folder away");

    let listing = list(&root).expect("lists");
    assert_eq!(listing.coverage, Coverage::Missing);
    assert!(listing.entries.is_empty());
    sweep(&root, listing.present().as_ref()).expect("sweeps");

    assert!(ApprovalStore::load(&root).get("acme-tools").is_some(), "the approval stays");
    assert!(root.probes_approved_dir().join("acme-tools").exists(), "and its copy");
    assert!(!root.probes_approved_dir().join(".new-ghost").exists());
    assert!(!root.probes_approved_dir().join("orphan").exists());
}

#[test]
fn only_runnable_hands_out_a_copy_and_only_for_a_probe_that_is_on() {
    let (_scratch, root) = setup();
    install(&root, "acme-tools", ACME);
    assert_eq!(runnable(&root, "acme-tools").map(|_| ()), Err(NotRunnable::NotApproved));
    approve_all(&root, "acme-tools");
    let copy = runnable(&root, "acme-tools").expect("on");
    assert_eq!(copy.dir(), root.probes_approved_dir().join("acme-tools"));

    // Edited after approval: the copy still verifies on its own, but the Probe may not run.
    fs::write(root.probes_dir().join("acme-tools/scripts/deploy.sh"), "#!/bin/sh\nevil\n").expect("edit");
    copy.verify().expect("the copy is still what was approved");
    assert_eq!(runnable(&root, "acme-tools").map(|_| ()), Err(NotRunnable::Changed));

    // Invalid now, and a name that is no id.
    fs::write(root.probes_dir().join("acme-tools/probe.toml"), "[probe\n").expect("break");
    assert!(matches!(runnable(&root, "acme-tools"), Err(NotRunnable::Invalid(_))));
    assert_eq!(runnable(&root, "../x").map(|_| ()), Err(NotRunnable::NotApproved));
}

#[test]
fn a_probe_is_not_on_without_its_copy_and_not_runnable_with_a_tampered_one() {
    let (_scratch, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    let copy = root.probes_approved_dir().join("acme-tools");
    set_mode(&copy, 0o700).expect("chmod");
    set_mode(&copy.join("probe.toml"), 0o600).expect("chmod");
    fs::write(copy.join("probe.toml"), "tampered").expect("tamper");
    assert!(matches!(runnable(&root, "acme-tools"), Err(NotRunnable::Copy(_))));

    snapshot::remove(&root, "acme-tools").expect("the copy is gone");
    assert_eq!(state(&root, "acme-tools"), ProbeState::Changed, "approved, but there is nothing to run");
    assert_eq!(runnable(&root, "acme-tools").map(|_| ()), Err(NotRunnable::Changed));
}

#[test]
fn a_failed_approval_leaves_the_old_copy_and_the_old_approval() {
    let (scratch, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    let before = ApprovalStore::load(&root).get("acme-tools").cloned().expect("approved");

    fs::write(root.probes_dir().join("acme-tools/scripts/deploy.sh"), "#!/bin/sh\necho v2\n").expect("edit");
    let request = request(&root, "acme-tools");
    let confirmed = request.confirm(&request.initial_grants(), Proof::TouchId).expect("a yes");
    // The approvals file cannot be written: the data folder takes no new file.
    set_mode(&scratch.data, 0o500).expect("chmod");
    let result = approve(&root, &request, &confirmed, NOW + 1);
    set_mode(&scratch.data, 0o700).expect("chmod back");

    assert!(matches!(result, Err(ApproveError::Store(_))), "{result:?}");
    let store = ApprovalStore::load(&root);
    assert_eq!(store.get("acme-tools"), Some(&before), "the old approval stays");
    store.snapshot(&root, "acme-tools").expect("approved").verify().expect("and the old copy still verifies");
    let names: Vec<_> = fs::read_dir(root.probes_approved_dir())
        .expect("dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["acme-tools"], "no staging or previous copy is left");
}

#[test]
fn a_first_approval_that_cannot_be_saved_leaves_no_copy() {
    let (scratch, root) = setup();
    install(&root, "acme-tools", ACME);
    let request = request(&root, "acme-tools");
    let confirmed = request.confirm(&request.initial_grants(), Proof::TouchId).expect("a yes");
    set_mode(&scratch.data, 0o500).expect("chmod");
    // `probes-approved/` has to be made first, which the read-only folder refuses too.
    fs::create_dir_all(root.probes_approved_dir()).ok();
    let result = approve(&root, &request, &confirmed, NOW);
    set_mode(&scratch.data, 0o700).expect("chmod back");
    assert!(result.is_err());
    assert!(!root.probes_approved_dir().join("acme-tools").exists());
}

#[test]
fn a_copy_an_interrupted_approval_set_aside_is_put_back() {
    let (_scratch, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    // The crash happened between the two renames: `<id>` is gone, `.old-<id>` has the approved copy.
    let parent = root.probes_approved_dir();
    fs::rename(parent.join("acme-tools"), parent.join(".old-acme-tools")).expect("rename");
    fs::create_dir_all(parent.join(".new-acme-tools")).expect("staging");

    sweep(&root, None).expect("sweeps");
    assert!(!parent.join(".old-acme-tools").exists() && !parent.join(".new-acme-tools").exists());
    runnable(&root, "acme-tools").expect("the approved copy is back and runs");
}

#[test]
fn a_click_is_not_a_yes_to_what_needs_touch_id_and_a_yes_is_for_one_request() {
    let (_scratch, root) = setup();
    install(&root, "acme-tools", ACME);
    let acme = request(&root, "acme-tools");
    assert_eq!(acme.confirm(&acme.initial_grants(), Proof::Click).map(|_| ()), Err(NeedsTouchId));
    let yes = acme.confirm(&acme.initial_grants(), Proof::TouchId).expect("Touch ID is a yes");

    // A yes for one request does not approve another (another Probe, or the same one after an edit).
    install(&root, "other-one", ACME);
    let other = request(&root, "other-one");
    assert!(matches!(approve(&root, &other, &yes, NOW), Err(ApproveError::Unconfirmed)));
    fs::write(root.probes_dir().join("acme-tools/scripts/deploy.sh"), "#!/bin/sh\nevil\n").expect("edit");
    let edited = request(&root, "acme-tools");
    assert!(matches!(approve(&root, &edited, &yes, NOW), Err(ApproveError::Unconfirmed)));
    assert!(ApprovalStore::load(&root).is_empty());

    // Switched off, a click is enough where only a click was asked for.
    let mut grants = acme.initial_grants();
    let program = acme.summary().permissions.iter().find(|p| p.key == "process").expect("process").clone();
    grants.set(&program, false);
    acme.confirm(&grants, Proof::Click).expect("only notify is left on");
}

#[test]
fn the_grants_are_the_ones_frozen_in_the_confirmation() {
    let (_scratch, root) = setup();
    install(&root, "acme-tools", ACME);
    let request = request(&root, "acme-tools");
    let mut grants = request.initial_grants();
    let confirmed = request.confirm(&grants, Proof::TouchId).expect("a yes");
    // Changed after the yes: not what was approved.
    let notify = request.summary().permissions.iter().find(|p| p.key == "notify").expect("notify").clone();
    grants.set(&notify, false);
    approve(&root, &request, &confirmed, NOW).expect("approves");
    assert!(ApprovalStore::load(&root).get("acme-tools").expect("approved").grants(&notify));
}

#[test]
fn an_approved_probe_can_be_forgotten_in_any_state_but_on() {
    let (_scratch, root) = setup();
    install(&root, "acme-tools", ACME);
    approve_all(&root, "acme-tools");
    let row = |root: &DataRoot| {
        list(root).expect("lists").entries.into_iter().find(|e| e.folder == "acme-tools").expect("row")
    };
    assert!(!row(&root).can_forget(), "on: the switch turns it off");

    fs::write(root.probes_dir().join("acme-tools/probe.toml"), "[probe\n").expect("break");
    let broken = row(&root);
    assert!(matches!(broken.state, ProbeState::Invalid(_)));
    assert!(broken.can_forget(), "an approval of a Probe that is invalid now can be forgotten");

    turn_off(&root, "acme-tools").expect("forgets");
    assert!(!row(&root).can_forget());
    assert!(ApprovalStore::load(&root).get("acme-tools").is_none());
    assert!(!root.probes_approved_dir().join("acme-tools").exists());
}
