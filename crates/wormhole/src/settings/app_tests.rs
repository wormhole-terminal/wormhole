//! The store as a GPUI global, in a headless app (needs `devctl`'s test support).

use gpui_kit::TestAppContext;
use gpui_kit::component::Theme as KitTheme;

use super::{Changed, SettingsStore};
use crate::theme_bridge::token;

/// A store with its theme handed to the Kit, as at launch.
fn setup_with_kit(cx: &mut gpui_kit::App) {
    gpui_kit::init(cx);
    cx.set_global(SettingsStore::for_test());
    SettingsStore::publish(Changed { theme: true, ..Changed::default() }, cx);
}

#[gpui_kit::test]
fn a_theme_change_reaches_the_kit_before_any_tick(cx: &mut TestAppContext) {
    cx.update(|cx| {
        setup_with_kit(cx);
        let dark_primary = KitTheme::global(cx).colors.primary;

        SettingsStore::update(cx, |s| s.theme = "GitHub Light".into());

        let store = cx.global::<SettingsStore>();
        let primary = token(&store.tokens, "primary");
        assert_ne!(primary, dark_primary, "the two themes should differ");
        assert_eq!(KitTheme::global(cx).colors.primary, primary);
        // Both copies of the color, and the Base layer scrollbars and selection read.
        assert_eq!(KitTheme::global(cx).tokens.primary.color, primary);
        assert_eq!(gpui_kit::base::Theme::global(cx).tokens.colors.primary, primary);
        assert!(!KitTheme::global(cx).is_dark());
    });
}

#[gpui_kit::test]
fn the_syntax_theme_follows_too(cx: &mut TestAppContext) {
    cx.update(|cx| {
        setup_with_kit(cx);
        let highlight_before = KitTheme::global(cx).highlight_theme.clone();

        SettingsStore::update(cx, |s| s.theme = "GitHub Light".into());

        let store = cx.global::<SettingsStore>();
        assert!(std::sync::Arc::ptr_eq(&KitTheme::global(cx).highlight_theme, &store.highlight));
        assert!(!std::sync::Arc::ptr_eq(&highlight_before, &store.highlight));
    });
}

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
