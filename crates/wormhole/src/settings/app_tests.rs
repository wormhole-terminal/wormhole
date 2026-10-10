//! The store as a GPUI global, in a headless app (needs `devctl`'s test support).

use gpui_kit::TestAppContext;

use super::SettingsStore;

#[gpui_kit::test]
fn a_tick_that_finds_nothing_leaves_a_pages_problem_alone(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_global(SettingsStore::for_test());
        // A config that was never loaded resolves its theme on the first tick (that tick does
        // find something); the tick under test is the idle one after it.
        SettingsStore::poll(cx);
        // A settings page reports a failed import this way, outside any settings change.
        cx.global_mut::<SettingsStore>().error = Some("import failed: nope".into());
        let revision = cx.global::<SettingsStore>().revision;

        // The app polls every tick; nothing changed on disk (there is no file at all).
        assert!(!SettingsStore::poll(cx));

        let store = cx.global::<SettingsStore>();
        assert_eq!(store.error.as_deref(), Some("import failed: nope"));
        assert_eq!(store.revision, revision);
    });
}

#[gpui_kit::test]
fn a_project_changes_how_its_sessions_look_not_the_app(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_global(SettingsStore::for_test());
        let (hotkey, auto_update, diff_wrap) = {
            let s = &cx.global::<SettingsStore>().settings;
            (s.hotkey.clone(), s.auto_update, s.diff_wrap)
        };
        let overlay =
            serde_json::json!({ "diff_wrap": !diff_wrap, "hotkey": "ctrl-`", "auto_update": !auto_update });

        SettingsStore::set_project_overrides(cx, overlay.as_object().cloned());

        let s = &cx.global::<SettingsStore>().settings;
        assert_eq!(s.diff_wrap, !diff_wrap, "a project may set how diffs wrap");
        assert_eq!(s.hotkey, hotkey, "but not the app's global hotkey");
        assert_eq!(s.auto_update, auto_update, "nor whether it updates itself");
    });
}
