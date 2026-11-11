//! The desk over a temp data folder with core's real rules: a Probe is off, then approved from exactly what its
//! sheet showed, on, and off again. The keys are the development build's files in the folder; the owner's
//! authentication is never asked (a click-level Probe), and a desk that is asked fails the test.

use std::fs;
use std::sync::Arc;

use wormhole_core::storage::paths::DataRoot;

use super::{ProbeApprovalOutcome, ProbesDesk};
use crate::files_panel::{OwnerAnswer, OwnerAuthenticator};
use crate::settings::tests::write_file;

struct NeverAsked;
impl OwnerAuthenticator for NeverAsked {
    fn authenticate(&self, _reason: String) -> OwnerAnswer {
        panic!("a test must not raise the authentication");
    }
}

fn desk() -> (tempfile::TempDir, Arc<ProbesDesk>) {
    let tmp = tempfile::tempdir().expect("temp dir");
    let root = DataRoot::new(tmp.path().join("Wormhole-swift-test"));
    fs::create_dir_all(root.probes_dir().join("acme-tools")).expect("probe folder");
    write_file(
        &root.probes_dir().join("acme-tools/probe.toml"),
        "[probe]\nid = \"acme-tools\"\nname = \"Acme tools\"\ndescription = \"Does things\"\nauthor = \"a\"\n\
         version = \"0.3.0\"\nwormhole = \">=1.3\"\napi = 1\n\n[permissions]\nnotify = true\n",
    );
    let desk = ProbesDesk::at(root, Arc::new(NeverAsked));
    (tmp, desk)
}

#[test]
fn a_probe_is_off_then_approved_from_its_sheet_then_on_then_off_again() {
    let (_tmp, desk) = desk();
    let state =
        |desk: &ProbesDesk| desk.list().rows.into_iter().find(|r| r.id == "acme-tools").map(|r| r.state);
    assert_eq!(state(&desk).as_deref(), Some("off"));

    assert_eq!(
        desk.approve("acme-tools".into(), vec![0]),
        ProbeApprovalOutcome::Cancelled,
        "no sheet was read yet"
    );
    let sheet = desk.sheet("acme-tools".into()).expect("a sheet");
    assert_eq!(sheet.permissions.len(), 1);
    assert!(!desk.needs_touch_id("acme-tools".into(), vec![0]), "a notification is a click");
    assert_eq!(desk.approve("acme-tools".into(), vec![0]), ProbeApprovalOutcome::Approved);
    assert_eq!(state(&desk).as_deref(), Some("on"));

    desk.turn_off("acme-tools".into()).expect("turns off");
    assert_eq!(state(&desk).as_deref(), Some("off"));
}

#[test]
fn a_folder_that_is_not_a_probe_is_listed_as_invalid_and_has_no_sheet() {
    let (_tmp, desk) = desk();
    let probes = desk.root.probes_dir();
    fs::create_dir_all(probes.join("broken")).expect("folder");
    write_file(&probes.join("broken/probe.toml"), "not a manifest");
    let row = desk.list().rows.into_iter().find(|r| r.id == "broken").expect("listed");
    assert_eq!((row.state.as_str(), row.can_approve), ("invalid", false));
    assert!(desk.sheet("broken".into()).is_err());
}
