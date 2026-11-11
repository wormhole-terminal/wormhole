//! The handle over a temp data folder, with core's real behaviour: values, writes, refusals, resets,
//! the file followed, a project's overrides, search and the host lists. Nothing here reads the real
//! data folder, home or agents' configs.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use wormhole_core::storage::files::{LinkPolicy, write_atomic};
use wormhole_core::storage::paths::{DataRoot, ProfileId};

use super::{SettingsHandle, SettingsListener};

/// A file written the way the app writes one, for a test that edits by hand.
pub(crate) fn write_file(path: &Path, text: &str) {
    write_atomic(path, text.as_bytes(), LinkPolicy::Follow).expect("writes");
}

/// Counts the wakes and lets a test wait for the next one.
#[derive(Default)]
struct Wakes {
    count: AtomicUsize,
    seen: Mutex<usize>,
    bell: Condvar,
}

impl SettingsListener for Wakes {
    fn wake(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
        let _guard = self.seen.lock().expect("lock");
        self.bell.notify_all();
    }
}

impl Wakes {
    fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    /// Waits (up to a few seconds: the watch is about 50 ms, its fallback timer two seconds) for more
    /// wakes than `after`.
    fn wait_past(&self, after: usize) -> bool {
        let guard = self.seen.lock().expect("lock");
        let (_guard, timeout) = self
            .bell
            .wait_timeout_while(guard, Duration::from_secs(6), |_| self.count() <= after)
            .expect("wait");
        !timeout.timed_out()
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: DataRoot,
    wakes: Arc<Wakes>,
    handle: Arc<SettingsHandle>,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp data dir");
        let root = DataRoot::new(dir.path());
        let wakes = Arc::new(Wakes::default());
        let handle = SettingsHandle::open_at(&root, &ProfileId::default_profile(), wakes.clone());
        Self { _dir: dir, root, wakes, handle }
    }

    fn config_file(&self) -> std::path::PathBuf {
        self.root.default_profile().config_file()
    }

    fn values(&self) -> Value {
        serde_json::from_str(&self.handle.values()).expect("values are JSON")
    }

    fn set(&self, key: &str, value: Value) -> Result<(), super::SettingsError> {
        self.handle.set(key.into(), value.to_string())
    }
}

#[test]
fn a_fresh_profile_shows_the_defaults_and_nothing_changed() {
    let fx = Fixture::new();
    assert_eq!(fx.values(), serde_json::from_str::<Value>(&fx.handle.defaults()).expect("JSON"));
    assert!(fx.handle.changed_keys().is_empty());
    assert!(fx.handle.changed_rows().is_empty());
    assert!(fx.config_file().exists(), "core writes the file with its defaults");
    assert_eq!(fx.handle.health().error, None);
}

#[test]
fn a_write_sticks_clamped_reaches_the_file_and_resets() {
    let fx = Fixture::new();
    let before = fx.handle.revision();
    fx.set("opacity", json!(0.5)).expect("sets");
    assert_eq!(fx.values()["opacity"], json!(0.5));
    assert!(fx.handle.revision() > before);
    let text = std::fs::read_to_string(fx.config_file()).expect("read");
    assert!(text.contains("\"opacity\": 0.5"), "{text}");
    assert_eq!(fx.handle.changed_keys(), ["opacity"]);
    let rows = fx.handle.changed_rows();
    assert_eq!(rows.len(), 1, "{rows:?}");

    fx.set("opacity", json!(7.0)).expect("sets");
    assert_eq!(fx.values()["opacity"], json!(1.0), "reads back as it was clamped");

    fx.set("opacity", json!(0.4)).expect("sets");
    fx.set("diff_wrap", json!(true)).expect("sets");
    assert_eq!(fx.handle.reset_all(), 2);
    assert!(fx.handle.changed_keys().is_empty());
    assert_eq!(fx.values(), serde_json::from_str::<Value>(&fx.handle.defaults()).expect("JSON"));
}

#[test]
fn copy_as_json_is_core_s_snippet_for_a_row_and_for_the_changed_page() {
    let fx = Fixture::new();
    fx.set("opacity", json!(0.5)).expect("sets");
    fx.set("motion.insert", json!("reveal")).expect("sets");
    // a row's keys, in core's shape: a key inside an object as that object, a key holding nothing left out
    assert_eq!(fx.handle.copy_json(vec!["opacity".into()]), "{\"opacity\": 0.5}");
    assert_eq!(fx.handle.copy_json(vec!["motion.insert".into()]), "{\"motion\": {\"insert\": \"reveal\"}}");
    assert_eq!(fx.handle.copy_json(vec!["no_such_key".into()]), "{}");
    // the Changed page copies the keys of the rows it lists, and only those
    let page = fx.handle.copy_changed_json();
    assert!(page.contains("\"opacity\": 0.5") && page.contains("\"insert\": \"reveal\""), "{page}");
    fx.handle.reset_all();
    assert_eq!(fx.handle.copy_changed_json(), "{}");
}

#[test]
fn what_core_refuses_changes_nothing_and_says_why() {
    let fx = Fixture::new();
    let (revision, file) = (fx.handle.revision(), std::fs::read(fx.config_file()).expect("read"));

    let wrong_type = fx.set("opacity", json!("dim")).expect_err("a string is no opacity").to_string();
    assert!(wrong_type.starts_with("`opacity`: "), "{wrong_type}");
    let unknown = fx.set("no_such_setting", json!(1)).expect_err("no such key").to_string();
    assert_eq!(unknown, "unknown setting `no_such_setting`");
    assert!(fx.handle.set("opacity".into(), "{not json".into()).is_err());
    assert!(fx.handle.set_many("[1]".into()).is_err(), "an object is wanted");

    // All or nothing: the good key before the bad one is not kept.
    let both = json!({ "opacity": 0.5, "no_such_setting": 1 }).to_string();
    assert!(fx.handle.set_many(both).is_err());

    assert_eq!(fx.handle.revision(), revision);
    assert_eq!(std::fs::read(fx.config_file()).expect("read"), file);
    assert!(fx.handle.changed_keys().is_empty(), "nothing was kept");

    fx.handle.set_many(json!({ "opacity": 0.5, "diff_wrap": true }).to_string()).expect("both read");
    assert_eq!((fx.values()["opacity"].clone(), fx.values()["diff_wrap"].clone()), (json!(0.5), json!(true)));
}

#[test]
fn the_file_is_followed_and_our_own_writes_do_not_wake() {
    let fx = Fixture::new();
    fx.set("opacity", json!(0.6)).expect("sets");
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(fx.wakes.count(), 0, "a write we made is not news to us");

    // A hand edit, as an editor saves it: the whole file replaced.
    write_file(&fx.config_file(), "{ \"opacity\": 0.4, \"diff_wrap\": true }\n");
    assert!(fx.wakes.wait_past(0), "the edit woke the window");
    let revision = fx.handle.drain();
    assert_eq!(fx.values()["opacity"], json!(0.4));
    assert_eq!(fx.values()["diff_wrap"], json!(true));
    assert_eq!(revision, fx.handle.revision());

    // Armed again by the drain: the next edit wakes again.
    let woken = fx.wakes.count();
    write_file(&fx.config_file(), "{ \"opacity\": 0.3 }\n");
    assert!(fx.wakes.wait_past(woken));
    fx.handle.drain();
    assert_eq!(fx.values()["opacity"], json!(0.3));
}

#[test]
fn a_file_that_does_not_read_is_never_written_over() {
    let fx = Fixture::new();
    let broken = "{ \"opacity\": \"dim\" }\n";
    write_file(&fx.config_file(), broken);
    assert!(fx.wakes.wait_past(0));
    fx.handle.drain();
    let health = fx.handle.health();
    assert!(health.unreadable);
    assert!(health.error.is_some_and(|error| error.contains("aren't saved until it's fixed")));

    fx.set("diff_wrap", json!(true)).expect("kept in memory");
    assert_eq!(fx.values()["diff_wrap"], json!(true));
    assert_eq!(
        std::fs::read_to_string(fx.config_file()).expect("read"),
        broken,
        "the file is left as it was"
    );
}

#[test]
fn a_projects_overrides_show_and_are_marked_but_are_not_the_persons_own() {
    let fx = Fixture::new();
    let wrap_row = wormhole_core::config::settings_index::entries()
        .find(|(_, _, entry)| entry.keys.contains(&"diff_wrap"))
        .map(|(_, _, entry)| entry.id)
        .expect("a row for diff_wrap");

    // `hotkey` is the app's, not a project's: dropped.
    let overrides = json!({ "diff_wrap": true, "hotkey": "ctrl-x" }).to_string();
    let before = fx.handle.revision();
    fx.handle.set_project_overrides(Some(overrides)).expect("an object");
    assert!(fx.handle.revision() > before);
    assert_eq!(fx.values()["diff_wrap"], json!(true));
    assert_ne!(fx.values()["hotkey"], json!("ctrl-x"));
    assert_eq!(fx.handle.overridden_entries(), [wrap_row]);
    assert!(fx.handle.changed_keys().is_empty(), "the project's value is not the person's change");

    fx.handle.set_project_overrides(None).expect("none");
    assert_eq!(fx.values()["diff_wrap"], json!(false));
    assert!(fx.handle.overridden_entries().is_empty());
    assert!(fx.handle.set_project_overrides(Some("[]".into())).is_err());
}

#[test]
fn search_finds_rows_by_core_rules() {
    let fx = Fixture::new();
    assert!(fx.handle.search("   ".into()).is_empty());
    let hits = fx.handle.search("line height".into());
    let first = hits.first().expect("a hit");
    assert_eq!(first.entry, "line-height");
    assert!(first.label.is_some(), "the match is marked in the label");
    assert!(fx.handle.search("zzzznothing".into()).is_empty());
}

#[test]
fn the_host_lists_come_from_core() {
    let fx = Fixture::new();
    let shells = fx.handle.shell_options();
    assert_eq!(shells[0].value, "", "the login shell is the empty value");
    let themes = fx.handle.icon_theme_options();
    assert_eq!(themes.first().map(|t| t.value.as_str()), Some("Wormhole Icons"));
    assert_eq!(themes.last().map(|t| t.value.as_str()), Some("None"));
    let paths = fx.handle.paths();
    assert!(paths.config_file.ends_with("config.json"));
    assert_eq!(paths.data_dir, fx.root.base().to_string_lossy());
}

#[test]
fn the_schema_the_rows_come_from_is_served() {
    let schema: Value = serde_json::from_str(&super::settings_schema_json()).expect("JSON");
    assert_eq!(schema["format"], json!(1));
    assert!(schema["sections"].as_array().is_some_and(|sections| !sections.is_empty()));
}

#[cfg(test)]
mod theme_files {
    use std::sync::Arc;

    use wormhole_core::storage::paths::{DataRoot, ProfileId};

    use crate::settings::handle::{SettingsHandle, SettingsListener};

    struct Quiet;
    impl SettingsListener for Quiet {
        fn wake(&self) {}
    }

    fn handle(data: &std::path::Path) -> Arc<SettingsHandle> {
        SettingsHandle::open_at(&DataRoot::new(data), &ProfileId::default_profile(), Arc::new(Quiet))
    }

    /// An imported theme is among the profile's themes and picked; moving it to the Trash is planned with the pick
    /// that names it, which goes back to the default first and comes back when the move fails.
    #[test]
    fn an_imported_theme_is_picked_and_its_trashing_resets_the_pick_and_can_put_it_back() {
        crate::isolate_for_tests();
        let data = tempfile::tempdir().expect("data dir");
        let scratch = tempfile::tempdir().expect("scratch dir");
        let file = scratch.path().join("Mine.json");
        super::write_file(
            &file,
            "{ \"name\": \"Mine\", \"extends\": \"Nord\", \"ui\": { \"primary\": \"#ff6600\" } }",
        );
        let handle = handle(data.path());
        handle.import_theme(file.to_string_lossy().into_owned()).expect("imported");
        assert!(data.path().join("themes").join("Mine.json").is_file(), "copied into the profile folder");
        let plan = handle.theme_trash_plan("Mine".into()).expect("a user theme");
        assert!(plan.message.contains("Mine") && plan.dark && !plan.light, "{plan:?}");
        assert!(plan.detail.contains("Wormhole Dark"), "{}", plan.detail);
        let before = handle.reset_theme_picks("Mine".into());
        assert!(handle.values().contains("\"theme\":\"Wormhole Dark\""), "{}", handle.values());
        handle.restore_theme_picks(before);
        assert!(!handle.values().contains("\"theme\":\"Wormhole Dark\""), "{}", handle.values());
        assert!(handle.theme_trash_plan("Nord".into()).is_none(), "a built-in has no file");
        let refused = handle.import_theme(scratch.path().join("nothing.json").to_string_lossy().into_owned());
        assert!(refused.is_err());
    }
}
