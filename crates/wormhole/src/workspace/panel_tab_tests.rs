//! View tests of the right panel's optional tabs: Notes behind its feature flag.

use gpui_kit::TestAppContext;
use wormhole_core::features::FeatureFlag;
use wormhole_core::keymap::ActionId;

use super::RightTab;
use super::settings_page::update_settings;
use super::view_tests::{draw, drive, isolate_in, read, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn set_notes_flag(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.feature_flags.insert(FeatureFlag::NotesAndTasks.key().to_string(), on);
        })
    });
}

#[gpui_kit::test]
fn notes_and_their_actions_are_gone_while_the_flag_is_off(cx: &mut TestAppContext) {
    if isolate("notes_and_their_actions_are_gone_while_the_flag_is_off") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let state = |cx: &mut TestAppContext| {
        drive(cx, &window, |ws, _, cx| {
            ws.show_tab(RightTab::Notes, cx);
            (ws.panel_tab_list(cx).contains(&RightTab::Notes), ws.right_tab, ws.menu_state(cx).hidden)
        })
    };
    let (listed, tab, hidden) = state(cx);
    assert!(listed && tab == RightTab::Notes && !hidden.contains(&ActionId::NewNote), "on: the tab and keys");

    set_notes_flag(cx, false);
    let (listed, tab, hidden) = state(cx);
    assert!(!listed, "off: no Notes tab");
    assert!(tab != RightTab::Notes, "off: the panel can't be put on Notes");
    for id in [ActionId::ShowNotes, ActionId::NewNote, ActionId::StashPrompt] {
        assert!(hidden.contains(&id), "{id:?} is hidden");
    }
    let before = read(cx, &window, |ws| ws.active_session().notes.len());
    drive(cx, &window, |ws, window, cx| ws.new_note(None, window, cx));
    assert_eq!(read(cx, &window, |ws| ws.active_session().notes.len()), before, "no note is made");

    set_notes_flag(cx, true);
    let (listed, tab, hidden) = state(cx);
    assert!(listed && tab == RightTab::Notes && !hidden.contains(&ActionId::NewNote), "on again: back");
}
