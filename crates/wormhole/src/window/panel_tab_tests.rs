//! View tests of the right panel's optional tabs: Notes behind its feature flag, Usage while a
//! coding agent is detected on screen.

use gpui_kit::TestAppContext;
use wormhole_core::agents::AgentId;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::keys::keymap::ActionId;

use crate::features::notes::testing as notes;
use crate::settings::update_settings;
use crate::window::view_tests::{draw, drive, isolate_in, read, read_cx, start};
use wormhole_core::config::TabId;

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
            ws.show_tab(TabId::Notes, cx);
            (ws.panel_tab_list(cx).contains(&TabId::Notes), ws.right_tab, ws.menu_state(cx).hidden)
        })
    };
    let (listed, tab, hidden) = state(cx);
    assert!(listed && tab == TabId::Notes && !hidden.contains(&ActionId::NewNote), "on: the tab and keys");

    set_notes_flag(cx, false);
    let (listed, tab, hidden) = state(cx);
    assert!(!listed, "off: no Notes tab");
    assert!(tab != TabId::Notes, "off: the panel can't be put on Notes");
    for id in [ActionId::ShowNotes, ActionId::NewNote, ActionId::StashPrompt] {
        assert!(hidden.contains(&id), "{id:?} is hidden");
    }
    let count = |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| notes::note_count(ws, ws.active, cx));
    let before = count(cx);
    drive(cx, &window, |ws, window, cx| notes::new_note(ws, ws.active, None, window, cx));
    assert_eq!(count(cx), before, "no note is made");

    set_notes_flag(cx, true);
    let (listed, tab, hidden) = state(cx);
    assert!(listed && tab == TabId::Notes && !hidden.contains(&ActionId::NewNote), "on again: back");
}

#[gpui_kit::test]
fn the_usage_tab_follows_the_agent(cx: &mut TestAppContext) {
    if isolate("the_usage_tab_follows_the_agent") {
        return;
    }
    let window = start(cx, 2);
    draw(cx, &window);
    let usage = |cx: &mut TestAppContext| {
        drive(cx, &window, |ws, _, cx| {
            (
                ws.panel_tab_list(cx).contains(&TabId::Usage),
                ws.menu_state(cx).hidden.contains(&ActionId::ShowUsage),
            )
        })
    };
    assert_eq!(usage(cx), (false, true), "no agent: no tab, no key");
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::Usage, cx));
    assert!(read(cx, &window, |ws| ws.right_tab != TabId::Usage), "and it can't be shown");

    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].terminal().update(cx, |t, _| t.agent = Some(AgentId::Claude));
    });
    assert_eq!(usage(cx), (true, false), "an agent runs: the tab and its key are there");

    drive(cx, &window, |ws, _, cx| {
        ws.show_tab(TabId::History, cx);
        ws.show_tab(TabId::Usage, cx);
    });
    assert!(read(cx, &window, |ws| ws.right_tab == TabId::Usage));

    // Another terminal, without an agent: back to the tab before Usage.
    drive(cx, &window, |ws, window, cx| {
        ws.activate(1, window, cx);
        ws.leave_missing_tab(cx);
    });
    assert!(!usage(cx).0);
    assert!(read(cx, &window, |ws| ws.right_tab == TabId::History), "back to History");

    // The agent going away does the same, to Changes when nothing was open before.
    drive(cx, &window, |ws, window, cx| {
        ws.activate(0, window, cx);
        ws.show_tab(TabId::Changes, cx);
        ws.show_tab(TabId::Usage, cx);
        ws.sessions[0].terminal().update(cx, |t, _| t.agent = None);
        ws.leave_missing_tab(cx);
    });
    assert!(read(cx, &window, |ws| ws.right_tab == TabId::Changes));
}
