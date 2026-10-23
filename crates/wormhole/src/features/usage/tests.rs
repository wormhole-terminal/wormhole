//! What the Usage tab does through the window: when it is offered, what it says, how it keeps its
//! reports and when it redraws.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{TestAppContext, VisualTestContext};
use wormhole_core::agents::AgentId;
use wormhole_core::agents::usage::{self, Usage, Window};
use wormhole_core::config::profile::{NewProfile, StartFrom};
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::storage::store::Store;

use crate::app_state::{AppState, OpenWindow, app, app_mut};
use crate::window::test_harness::{draw, drive, isolate_in, read, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// Far enough ahead that a window never counts as reset while a test runs.
const FAR: i64 = 4_000_000_000;

pub(crate) fn claude(used: f32, updated: i64) -> Usage {
    Usage {
        agent: AgentId::Claude,
        five_hour: Some(Window { used, resets_at: FAR }),
        seven_day: None,
        limits: Vec::new(),
        response_tokens: None,
        updated,
    }
}

/// An agent reports its limits, as the hook socket's message does.
fn report(cx: &mut TestAppContext, usage: Usage) {
    cx.update(|cx| super::record(usage, cx));
    cx.run_until_parked();
}

fn reports(cx: &mut TestAppContext) -> Vec<Usage> {
    cx.update(|cx| super::reports(cx).to_vec())
}

/// What the profile's database remembers of them.
fn remembered(cx: &mut TestAppContext) -> Vec<Usage> {
    cx.update(|cx| {
        let json = app(cx).store.as_ref().and_then(|store| store.meta(usage::STORE_KEY).ok().flatten());
        json.map(|json| usage::from_json(&json)).unwrap_or_default()
    })
}

/// Gives the app the database `name` in the test's scratch dir, which `report` then writes to: a
/// profile's, as a switch to it would.
fn with_database(cx: &mut TestAppContext, name: &str) {
    // The scratch data dir the test runs in, which the harness removes afterwards.
    let data = std::env::var_os("WORMHOLE_DATA_DIR").expect("a scratch data dir");
    let dir = std::path::PathBuf::from(data).join(format!("usage-{name}"));
    std::fs::create_dir_all(&dir).expect("makes the scratch dir");
    let store = Store::open_at(&dir).expect("opens a database");
    cx.update(|cx| app_mut(cx).store = Some(store));
}

/// Whether a frame drew the element `name` (a debug selector of the tab).
pub(crate) fn drawn(cx: &mut TestAppContext, window: &OpenWindow, name: &str) -> bool {
    draw(cx, window);
    // A selector is a `&'static str`; a test may leak its few.
    let name: &'static str = Box::leak(name.to_string().into_boxed_str());
    VisualTestContext::from_window(window.handle, cx).debug_bounds(name).is_some()
}

/// A window on its Usage tab: an agent runs in the terminal on screen.
pub(crate) fn on_usage(cx: &mut TestAppContext) -> OpenWindow {
    let window = start(cx, 1);
    drive(cx, &window, |ws, _, cx| {
        ws.sessions()[0].terminal().update(cx, |t, cx| {
            t.agent = Some(AgentId::Claude);
            cx.notify();
        });
        ws.sync_summaries(cx);
        ws.show_tab(TabId::Usage, cx);
    });
    assert_eq!(read(cx, &window, |ws| ws.right_tab()), TabId::Usage);
    window
}

#[gpui_kit::test]
fn the_tab_says_how_to_get_reports_until_there_are_some(cx: &mut TestAppContext) {
    if isolate("the_tab_says_how_to_get_reports_until_there_are_some") {
        return;
    }
    let window = on_usage(cx);
    assert!(drawn(cx, &window, "usage-empty"), "no report: the note about integrations");
    assert!(!drawn(cx, &window, "usage-card-Claude"));

    report(cx, claude(25.0, 100));
    assert!(drawn(cx, &window, "usage-card-Claude"), "a report: its card");
    assert!(!drawn(cx, &window, "usage-empty"), "and no note");
    assert!(!drawn(cx, &window, "usage-card-Codex"), "only agents that reported");
}

#[gpui_kit::test]
fn the_latest_report_of_an_agent_wins_and_is_remembered(cx: &mut TestAppContext) {
    if isolate("the_latest_report_of_an_agent_wins_and_is_remembered") {
        return;
    }
    let _window = start(cx, 1);
    with_database(cx, "one");
    report(cx, claude(25.0, 100));
    report(cx, Usage { agent: AgentId::Codex, ..claude(10.0, 110) });
    report(cx, claude(40.0, 120));
    let kept = reports(cx);
    assert_eq!(kept.len(), 2, "one report per agent");
    let newest = kept.iter().find(|u| u.agent == AgentId::Claude).expect("Claude's");
    assert_eq!(newest.five_hour.map(|w| w.used), Some(40.0), "the newest");
    assert_eq!(remembered(cx), kept, "kept across restarts");
}

#[gpui_kit::test]
fn the_tab_redraws_on_the_tick_only_while_it_shows(cx: &mut TestAppContext) {
    if isolate("the_tab_redraws_on_the_tick_only_while_it_shows") {
        return;
    }
    let window = on_usage(cx);
    let redraws = Rc::new(Cell::new(0u32));
    let counted = redraws.clone();
    cx.update(|cx| {
        let workspace = window.workspace.upgrade().expect("the window is open");
        cx.observe(&workspace, move |_, _| counted.set(counted.get() + 1)).detach();
    });
    // The redraws of the window's tick number `at`. What else a tick notifies for is the same whatever
    // the tab, so a difference between the tabs is the Usage tab's own.
    let tick_at = |cx: &mut TestAppContext, tab: TabId, at: u64| {
        drive(cx, &window, |ws, _, cx| {
            // The tick looks for the agent in the terminal's process tree, where `cat` runs and none
            // does, so it is told again before each.
            ws.sessions()[0].terminal().update(cx, |t, cx| {
                t.agent = Some(AgentId::Claude);
                cx.notify();
            });
            ws.sync_summaries(cx);
            ws.show_tab(tab, cx);
        });
        redraws.set(0);
        drive(cx, &window, |ws, window, cx| {
            ws.set_ticks(at - 1);
            ws.tick(window, cx);
        });
        redraws.get()
    };
    let period = 25;
    let ticks = |cx: &mut TestAppContext, tab| {
        [tick_at(cx, tab, period), tick_at(cx, tab, period + 1), tick_at(cx, tab, 2 * period)]
    };
    let on = ticks(cx, TabId::Usage);
    assert!(on[0] > 0 && on[2] > 0, "every {period} ticks while it shows");

    let off = ticks(cx, TabId::Changes);
    assert!(on[1] == off[1], "between those ticks, the same as another tab");
    assert!(on[0] > off[0] && on[2] > off[2], "another tab: the tick leaves the window alone");
}

#[gpui_kit::test]
fn each_profile_has_its_own_reports(cx: &mut TestAppContext) {
    if isolate("each_profile_has_its_own_reports") {
        return;
    }
    let _window = start(cx, 1);
    with_database(cx, "home");
    report(cx, claude(25.0, 100));
    let home = reports(cx);
    assert_eq!(home.len(), 1);

    // Another profile is shown, through the switch the app itself does.
    let work = cx
        .update(|cx| {
            let new =
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults };
            AppState::create_profile(new, cx)
        })
        .expect("makes the profile");
    // The switch re-reads the custom fonts, which the app has loaded by now.
    cx.update(|cx| cx.set_global(crate::fonts::LoadedFonts::default()));
    cx.update(|cx| AppState::switch_profile(&work, cx)).expect("switches");
    cx.run_until_parked();
    assert!(reports(cx).is_empty(), "the other profile's reports, which are none");
    report(cx, Usage { agent: AgentId::Codex, ..claude(5.0, 200) });
    assert_eq!(reports(cx).iter().map(|u| u.agent).collect::<Vec<_>>(), [AgentId::Codex]);

    // Back: the first profile's, as they were left, in memory and in its database; the other's report
    // went to the other's database only.
    cx.update(|cx| AppState::switch_profile(&ProfileId::default_profile(), cx)).expect("switches back");
    cx.run_until_parked();
    assert_eq!(reports(cx), home);
    assert_eq!(remembered(cx), home);
    let in_work = cx.update(|cx| {
        let store = app(cx).store_of(&work).expect("Work's database is parked");
        let json = store.meta(usage::STORE_KEY).ok().flatten();
        json.map(|json| usage::from_json(&json)).unwrap_or_default()
    });
    assert_eq!(in_work.iter().map(|u| u.agent).collect::<Vec<_>>(), [AgentId::Codex]);

    // Profile ids are slugs, so deleting "Work" and making "Work" again gives the same id: the new
    // profile starts with no reports, and the first it gets is the only one in its database.
    cx.update(|cx| AppState::delete_profile(&work, cx)).expect("deletes the profile");
    let again = cx
        .update(|cx| {
            let new =
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults };
            AppState::create_profile(new, cx)
        })
        .expect("makes the profile again");
    assert_eq!(again, work, "the same id");
    cx.update(|cx| AppState::switch_profile(&again, cx)).expect("switches");
    cx.run_until_parked();
    assert!(reports(cx).is_empty(), "nothing of the deleted profile's");
    report(cx, claude(60.0, 300));
    assert_eq!(reports(cx).iter().map(|u| u.agent).collect::<Vec<_>>(), [AgentId::Claude]);
    assert_eq!(remembered(cx), reports(cx), "and its database has only what it reported itself");
}

#[gpui_kit::test]
fn a_report_repaints_every_window_that_has_the_tab(cx: &mut TestAppContext) {
    if isolate("a_report_repaints_every_window_that_has_the_tab") {
        return;
    }
    let window = start(cx, 1);
    let redraws = Rc::new(Cell::new(0u32));
    let counted = redraws.clone();
    cx.update(|cx| {
        let workspace = window.workspace.upgrade().expect("the window is open");
        cx.observe(&workspace, move |_, _| counted.set(counted.get() + 1)).detach();
    });
    report(cx, claude(25.0, 100));
    assert!(redraws.get() > 0, "the window is told, whichever tab it shows");
}

#[gpui_kit::test]
fn the_entry_offers_the_tab_and_its_menu_line_while_an_agent_is_on_screen(cx: &mut TestAppContext) {
    if isolate("the_entry_offers_the_tab_and_its_menu_line_while_an_agent_is_on_screen") {
        return;
    }
    let window = start(cx, 1);
    let asked = |cx: &mut TestAppContext| {
        drive(cx, &window, |ws, _, cx| {
            let at =
                ws.feature_defs().iter().position(|def| def.id == FeatureId::Usage).expect("in the table");
            let (def, entity) =
                (&ws.feature_defs()[at], ws.feature_entity(at).expect("made with the window"));
            let (tab, facts) = (&def.tabs[0], ws.facts(cx));
            let mut state = Default::default();
            (def.menu.expect("has a menu hook"))(entity, &facts, &mut state, cx);
            let words = (tab.spoken)(entity, &facts, cx);
            ((tab.shown)(&facts, cx), state.hidden.contains(&ActionId::ShowUsage), words)
        })
    };
    assert_eq!(asked(cx), (false, true, "Usage".to_string()), "no agent");
    drive(cx, &window, |ws, _, cx| {
        ws.sessions()[0].terminal().update(cx, |t, cx| {
            t.agent = Some(AgentId::Codex);
            cx.notify();
        });
    });
    assert_eq!(asked(cx), (true, false, "Usage".to_string()), "an agent");
}
