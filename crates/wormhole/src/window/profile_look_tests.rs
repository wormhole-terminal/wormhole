//! An odd window (a window in a profile Wormhole is not in) draws with its own profile's look,
//! while the windows of the shown profile keep drawing with the shown one (see `view_tests` for
//! how each test runs in a child process with scratch data).
use gpui_kit::TestAppContext;
use wormhole_core::config::ThemeMode;
use wormhole_core::config::profile::{NewProfile, StartFrom};
use wormhole_core::storage::paths::ProfileId;

use crate::app_state::{AppState, app};
use crate::settings::{ParkedSettings, SettingsStore, settings, settings_in, store_in, ui, ui_in};
use crate::window::Workspace;
use crate::window::view_tests::{draw, isolate_in, read, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

#[gpui_kit::test]
fn an_odd_window_reads_its_profiles_theme_and_fonts(cx: &mut TestAppContext) {
    if isolate("an_odd_window_reads_its_profiles_theme_and_fonts") {
        return;
    }
    let shown = start(cx, 1);
    let work = cx
        .update(|cx| {
            let new =
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults };
            let id = AppState::create_profile(new, cx)?;
            AppState::ensure_open(&id, cx);
            Ok::<ProfileId, String>(id)
        })
        .expect("makes the profile");
    cx.update(|cx| {
        // The shown profile: dark, one terminal font. Work: light, another.
        crate::settings::update_settings(cx, |s| {
            s.theme_mode = ThemeMode::Dark;
            s.font_family = "Menlo".into();
        });
        cx.global_mut::<ParkedSettings>().get_mut(&work).expect("Work is parked").restyle(|s| {
            s.theme_mode = ThemeMode::Light;
            s.font_family = "Monaco".into();
        });
    });

    cx.update(|cx| AppState::open_window_in_profile(&work, cx));
    cx.run_until_parked();
    let odd = cx
        .update(|cx| app(cx).windows.iter().find(|w| w.profile == work).cloned())
        .expect("the odd window opened");

    cx.update(|cx| {
        let work_ui = ui_in(&work, cx);
        assert_ne!(work_ui.background, ui(cx).background, "the odd window's colours are its own");
        assert_eq!(ui_in(&ProfileId::default_profile(), cx).background, ui(cx).background);
        assert_eq!(settings_in(&work, cx).font_family, "Monaco");
        assert_eq!(settings(cx).font_family, "Menlo");
        assert_ne!(store_in(&work, cx).tokens.appearance, cx.global::<SettingsStore>().tokens.appearance);
    });

    // Its terminal knows the window's profile, so it draws with that profile's palette and fonts.
    let terminal_profile = read(cx, &odd, |ws: &Workspace| ws.sessions[0].terminal().clone());
    cx.update(|cx| assert_eq!(terminal_profile.read(cx).profile(), &work));
    let shown_terminal = read(cx, &shown, |ws: &Workspace| ws.sessions[0].terminal().clone());
    cx.update(|cx| assert!(shown_terminal.read(cx).profile().is_default()));
    draw(cx, &odd);
    draw(cx, &shown);
}
