//! Settings → Extensions through the real window: the page lists the Probes of the data folder, a click
//! approval flips a row to On, an edit makes it Changed, and Touch ID that is cancelled changes nothing.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.
//! Touch ID is never reached: under test `platform::auth` answers "cancelled" itself, and the approval that
//! follows a yes is driven through `finish_approval`.

use std::fs;
use std::path::PathBuf;

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{Entity, TestAppContext};
use wormhole_core::probe::approval::{self, ApprovalStore, ProbeState};
use wormhole_core::storage::paths::DataRoot;

use super::{Section, SettingsWindow, current_window, open_settings};
use crate::platform::auth::AuthError;
use crate::window::test_harness::{isolate_in, start};

const COMMAND: &str = "[[contributes.commands]]\nid = \"go\"\ntitle = \"Go\"\nrun = [\"git\", \"log\", \"-1\"]\n\
     in = \"background\"\ncwd = \"terminal\"\n";

fn manifest(id: &str, permissions: &str, extra: &str) -> String {
    format!(
        "[probe]\nid = \"{id}\"\nname = \"{id} tools\"\ndescription = \"does things\"\nauthor = \"a\"\n\
         version = \"0.3.0\"\nwormhole = \">=1.3\"\napi = 1\n\n[permissions]\n{permissions}\n{extra}"
    )
}

/// A Probe that needs Touch ID (it runs a program) and one that needs only a click.
fn install(root: &DataRoot) {
    for (id, permissions, extra) in [
        ("acme-tools", "process = [\"git\"]\nnotify = true\n", COMMAND),
        ("quiet-one", "notify = true\n", ""),
    ] {
        let dir = root.probes_dir().join(id);
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("probe.toml"), manifest(id, permissions, extra)).expect("manifest");
    }
    let broken = root.probes_dir().join("broken");
    fs::create_dir_all(&broken).expect("mkdir");
    fs::write(broken.join("probe.toml"), "[probe\n").expect("manifest");
}

/// Makes the read-only copies deletable again, so the harness can remove the scratch folder.
struct Tidy(DataRoot);

impl Drop for Tidy {
    fn drop(&mut self) {
        let _ = approval::sweep(&self.0, &Default::default());
    }
}

fn open(cx: &mut TestAppContext) -> Entity<SettingsWindow> {
    cx.update(|cx| open_settings(Some(Section::Extensions), cx));
    cx.run_until_parked();
    cx.update(|cx| current_window(cx)).expect("the settings window opened").1
}

fn state(cx: &mut TestAppContext, page: &Entity<SettingsWindow>, folder: &str) -> Option<ProbeState> {
    page.read_with(cx, |page, _| page.extensions.state_of(folder).cloned())
}

fn scan(cx: &mut TestAppContext, page: &Entity<SettingsWindow>) {
    cx.update(|cx| page.update(cx, |page, cx| page.scan_probes(cx)));
    cx.run_until_parked();
}

fn sheet_open(cx: &mut TestAppContext, page: &Entity<SettingsWindow>) -> bool {
    page.read_with(cx, |page, _| page.extensions.sheet.is_some())
}

fn open_sheet(cx: &mut TestAppContext, page: &Entity<SettingsWindow>, id: &str) {
    let id = id.to_string();
    cx.update(|cx| page.update(cx, |page, cx| page.open_probe_sheet(id, cx)));
    cx.run_until_parked();
}

fn approvals_file() -> PathBuf {
    DataRoot::current().probe_approvals_file()
}

#[gpui_kit::test]
fn the_page_lists_the_probes_with_their_states(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_page_lists_the_probes_with_their_states") {
        return;
    }
    start(cx, 1);
    let root = DataRoot::current();
    let _tidy = Tidy(root.clone());
    install(&root);
    let page = open(cx);

    assert_eq!(state(cx, &page, "acme-tools"), Some(ProbeState::Off));
    assert_eq!(state(cx, &page, "quiet-one"), Some(ProbeState::Off));
    assert!(matches!(state(cx, &page, "broken"), Some(ProbeState::Invalid(_))));
    // And it draws, with the rows in it.
    let (handle, _) = cx.update(|cx| current_window(cx)).expect("open");
    cx.update(|cx| handle.update(cx, |_, window, cx| window.render_frame(cx))).expect("draws");
}

#[gpui_kit::test]
fn an_empty_folder_draws_the_empty_page(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "an_empty_folder_draws_the_empty_page") {
        return;
    }
    start(cx, 1);
    let page = open(cx);
    assert_eq!(page.read_with(cx, |page, _| page.extensions.listed()), Some(0));
    let (handle, _) = cx.update(|cx| current_window(cx)).expect("open");
    cx.update(|cx| handle.update(cx, |_, window, cx| window.render_frame(cx))).expect("draws");
}

#[gpui_kit::test]
fn a_click_approval_turns_a_probe_on_and_off_again(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_click_approval_turns_a_probe_on_and_off_again") {
        return;
    }
    start(cx, 1);
    let root = DataRoot::current();
    let _tidy = Tidy(root.clone());
    install(&root);
    let page = open(cx);

    open_sheet(cx, &page, "quiet-one");
    assert!(sheet_open(cx, &page), "switching on asks first");
    assert_eq!(state(cx, &page, "quiet-one"), Some(ProbeState::Off), "and nothing is on yet");
    cx.update(|cx| page.update(cx, |page, cx| page.approve_probe(cx)));
    cx.run_until_parked();

    assert!(!sheet_open(cx, &page));
    assert_eq!(state(cx, &page, "quiet-one"), Some(ProbeState::On));
    let store = ApprovalStore::load(&root);
    store.snapshot(&root, "quiet-one").expect("approved").verify().expect("the copy is what was approved");

    // Off needs no question, and takes the copy with it.
    cx.update(|cx| page.update(cx, |page, cx| page.turn_probe_off("quiet-one".into(), "Quiet".into(), cx)));
    cx.run_until_parked();
    assert_eq!(state(cx, &page, "quiet-one"), Some(ProbeState::Off));
    assert!(!root.probes_approved_dir().join("quiet-one").exists());
    assert!(ApprovalStore::load(&root).get("quiet-one").is_none());
}

#[gpui_kit::test]
fn cancelled_touch_id_changes_nothing_and_a_yes_records_the_approval(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "cancelled_touch_id_changes_nothing_and_a_yes_records_the_approval") {
        return;
    }
    start(cx, 1);
    let root = DataRoot::current();
    let _tidy = Tidy(root.clone());
    install(&root);
    let page = open(cx);

    open_sheet(cx, &page, "acme-tools");
    // Approve asks the system, which under test answers "cancelled" at once.
    cx.update(|cx| page.update(cx, |page, cx| page.approve_probe(cx)));
    cx.run_until_parked();
    assert!(sheet_open(cx, &page), "the sheet stays for another look");
    assert_eq!(state(cx, &page, "acme-tools"), Some(ProbeState::Off));
    assert!(ApprovalStore::load(&root).get("acme-tools").is_none());
    assert!(!root.probes_approved_dir().join("acme-tools").exists());
    assert!(!approvals_file().exists(), "nothing was written");

    // A failed one says so and changes nothing either.
    cx.update(|cx| page.update(cx, |page, cx| page.finish_approval(Err(AuthError::Failed("no".into())), cx)));
    cx.run_until_parked();
    assert!(sheet_open(cx, &page));
    assert_eq!(state(cx, &page, "acme-tools"), Some(ProbeState::Off));

    // A yes records exactly what the sheet showed.
    cx.update(|cx| page.update(cx, |page, cx| page.finish_approval(Ok(()), cx)));
    cx.run_until_parked();
    assert!(!sheet_open(cx, &page));
    assert_eq!(state(cx, &page, "acme-tools"), Some(ProbeState::On));
}

#[gpui_kit::test]
fn a_permission_switched_off_in_the_sheet_is_stored_as_not_granted(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_permission_switched_off_in_the_sheet_is_stored_as_not_granted") {
        return;
    }
    start(cx, 1);
    let root = DataRoot::current();
    let _tidy = Tidy(root.clone());
    install(&root);
    let page = open(cx);

    open_sheet(cx, &page, "acme-tools");
    // Leave the program off: what is left is `notify`, a click.
    cx.update(|cx| {
        page.update(cx, |page, _| {
            let sheet = page.extensions.sheet.as_mut().expect("open");
            let program =
                sheet.request().summary().permissions.iter().find(|p| p.key == "process").expect("process");
            let program = program.clone();
            sheet.grants_mut().set(&program, false);
        })
    });
    cx.update(|cx| page.update(cx, |page, cx| page.approve_probe(cx)));
    cx.run_until_parked();

    assert_eq!(state(cx, &page, "acme-tools"), Some(ProbeState::On), "a click was enough");
    let store = ApprovalStore::load(&root);
    let approval = store.get("acme-tools").expect("approved");
    assert_eq!(approval.granted, ["notify:show notifications"]);
    assert_eq!(approval.declared.len(), 2);
}

#[gpui_kit::test]
fn an_edit_makes_a_probe_changed_and_the_sheet_says_what_changed(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "an_edit_makes_a_probe_changed_and_the_sheet_says_what_changed") {
        return;
    }
    start(cx, 1);
    let root = DataRoot::current();
    let _tidy = Tidy(root.clone());
    install(&root);
    let page = open(cx);
    open_sheet(cx, &page, "quiet-one");
    cx.update(|cx| page.update(cx, |page, cx| page.approve_probe(cx)));
    cx.run_until_parked();
    assert_eq!(state(cx, &page, "quiet-one"), Some(ProbeState::On));

    fs::write(root.probes_dir().join("quiet-one/notes.txt"), "an edit").expect("edit");
    scan(cx, &page);
    assert_eq!(state(cx, &page, "quiet-one"), Some(ProbeState::Changed));

    // Switching it on again opens the sheet, which says the files changed.
    open_sheet(cx, &page, "quiet-one");
    let files_only = page.read_with(cx, |page, _| {
        page.extensions
            .sheet
            .as_ref()
            .and_then(|sheet| sheet.request().changes())
            .is_some_and(|changes| changes.files_only())
    });
    assert!(files_only);
    let (handle, _) = cx.update(|cx| current_window(cx)).expect("open");
    cx.update(|cx| handle.update(cx, |_, window, cx| window.render_frame(cx))).expect("draws the sheet");

    cx.update(|cx| page.update(cx, |page, cx| page.approve_probe(cx)));
    cx.run_until_parked();
    assert_eq!(state(cx, &page, "quiet-one"), Some(ProbeState::On));
}

#[gpui_kit::test]
fn a_probe_that_cannot_be_approved_opens_no_sheet(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_probe_that_cannot_be_approved_opens_no_sheet") {
        return;
    }
    start(cx, 1);
    let root = DataRoot::current();
    let _tidy = Tidy(root.clone());
    install(&root);
    let page = open(cx);
    open_sheet(cx, &page, "broken");
    assert!(!sheet_open(cx, &page));
    assert!(page.read_with(cx, |page, _| page.notice.is_some()), "it says why");
}

#[gpui_kit::test]
fn removing_a_probe_forgets_its_approval(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "removing_a_probe_forgets_its_approval") {
        return;
    }
    start(cx, 1);
    let root = DataRoot::current();
    let _tidy = Tidy(root.clone());
    install(&root);
    let page = open(cx);
    open_sheet(cx, &page, "quiet-one");
    cx.update(|cx| page.update(cx, |page, cx| page.approve_probe(cx)));
    cx.run_until_parked();

    fs::remove_dir_all(root.probes_dir().join("quiet-one")).expect("the user deletes it");
    scan(cx, &page);
    assert_eq!(state(cx, &page, "quiet-one"), None);
    assert!(ApprovalStore::load(&root).get("quiet-one").is_none());
    assert!(!root.probes_approved_dir().join("quiet-one").exists());
}
