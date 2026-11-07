use std::path::Path;
use std::sync::Once;

use crate::aux_backends::common::test_support::scratch;
use crate::aux_backends::{FirstRun, HookState, LookRecord, PlaceKindRecord};

/// Agents are looked for on `WORMHOLE_AGENT_PATH` when it is set: a folder with one fake `claude`, so what the
/// Mac running the tests has installed never shows. Once, before any test thread reads it.
fn fake_path() -> &'static Path {
    static ONCE: Once = Once::new();
    static DIR: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    let dir = DIR.get_or_init(|| tempfile::tempdir().expect("temp"));
    ONCE.call_once(|| {
        let exe = dir.path().join("claude");
        std::fs::write(&exe, "#!/bin/sh\n").expect("a fake agent");
        std::fs::set_permissions(&exe, std::os::unix::fs::PermissionsExt::from_mode(0o755)).expect("mode");
        // SAFETY: called once, from the first test that needs it; only `agents::installed` reads it.
        unsafe { std::env::set_var("WORMHOLE_AGENT_PATH", dir.path()) };
    });
    dir.path()
}

fn run(data: &Path, agent_home: &Path) -> std::sync::Arc<FirstRun> {
    fake_path();
    FirstRun::new(Some(agent_home.display().to_string()), Some(data.display().to_string()))
        .expect("a first run")
}

#[test]
fn the_first_step_offers_recent_folders_then_the_standard_places() {
    let (root, data) = scratch();
    let home = root.path().join("home");
    for dir in ["Desktop", "Developer", "work/api"] {
        std::fs::create_dir_all(home.join(dir)).expect("folder");
    }
    let run = run(&data, &root.path().join("agents"));
    let places = run.places(
        Some(home.display().to_string()),
        vec![home.join("work/api").display().to_string(), home.join("gone").display().to_string()],
    );
    let kinds: Vec<_> = places.iter().map(|p| p.kind).collect();
    assert_eq!(
        kinds,
        [
            PlaceKindRecord::Recent,
            PlaceKindRecord::Home,
            PlaceKindRecord::Desktop,
            PlaceKindRecord::Documents,
            PlaceKindRecord::Developer
        ],
        "a recent folder that is gone is not offered; Developer only when it is there"
    );
    assert_eq!((places[0].name.as_str(), places[0].detail.as_str()), ("api", "~/work"));
}

#[test]
fn agents_on_path_are_listed_and_their_hooks_written_where_told() {
    let (root, data) = scratch();
    let agents = root.path().join("agents");
    let run = run(&data, &agents);
    let scan = run.scan_agents();
    assert_eq!(scan.found.len(), 1, "only the fake claude is on the path: {:?}", scan.found);
    assert!(scan.others > 0);
    let claude = &scan.found[0];
    assert_eq!(
        (claude.id.as_str(), claude.name.as_str(), claude.hooks),
        ("claude", "Claude Code", HookState::NotInstalled)
    );

    let after = run.install_hooks("claude".into()).expect("installed");
    assert_eq!(after.hooks, HookState::Installed);
    assert!(agents.join(".claude/settings.json").is_file(), "under the agent home, not the real one");
    assert_eq!(run.scan_agents().found[0].hooks, HookState::Installed, "read back from disk");
    assert!(run.install_hooks("nobody".into()).is_err());
}

#[test]
fn the_first_run_is_remembered_once_finished_and_not_before() {
    let (root, data) = scratch();
    let run = run(&data, &root.path().join("agents"));
    assert!(run.should_show(0));
    assert!(!run.should_show(2), "a saved session means this is not a first launch");
    run.finish().expect("remembered");
    assert!(!run.should_show(0));
}

#[test]
fn the_look_steps_choices_are_written_through_core_settings() {
    let (root, data) = scratch();
    let run = run(&data, &root.path().join("agents"));
    run.write_look(LookRecord {
        primary: Some("#ff8800".into()),
        font_family: "Menlo".into(),
        font_size: 15.0,
        ligatures: false,
        notifications: false,
        notify_command_done: false,
    });
    let text =
        std::fs::read_to_string(data.join("config.json")).expect("config.json in the scratch data folder");
    let json: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    assert_eq!(json["font_family"], "Menlo");
    assert_eq!(json["font_size"], 15.0);
    assert_eq!(json["font_ligatures"], false);
    assert_eq!(json["notifications"], false);
    assert_eq!(json["notify_command_done"], false);
    assert_eq!(json["primary_color"], "#ff8800");
    assert_eq!(json["theme_primary"], false);
}
