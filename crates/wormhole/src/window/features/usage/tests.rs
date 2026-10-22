//! What the Usage tab does through the window: when it is offered, what it says, how it keeps its
//! reports and when it redraws.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{TestAppContext, VisualTestContext};
use wormhole_core::agents::AgentId;
use wormhole_core::agents::usage::{self, Usage, Window};
use wormhole_core::config::TabId;
use wormhole_core::storage::store::Store;

use crate::app_state::{OpenWindow, app, app_mut};
use crate::window::view_tests::{draw, drive, isolate_in, read, start};

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
        ws.sessions[0].terminal().update(cx, |t, _| t.agent = Some(AgentId::Claude));
        ws.show_tab(TabId::Usage, cx);
    });
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::Usage);
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
            ws.sessions[0].terminal().update(cx, |t, _| t.agent = Some(AgentId::Claude));
            ws.show_tab(tab, cx);
        });
        redraws.set(0);
        drive(cx, &window, |ws, window, cx| {
            ws.ticks = at - 1;
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
    with_database(cx, "work");
    report(cx, claude(25.0, 100));
    let work = reports(cx);
    assert_eq!(work.len(), 1);

    // Another profile is shown: its database, which has none yet.
    with_database(cx, "home");
    cx.update(super::profile_shown);
    assert!(reports(cx).is_empty(), "the other profile's reports, which are none");
    report(cx, Usage { agent: AgentId::Codex, ..claude(5.0, 200) });

    // Back: the first profile's, as they were left, and the second's are kept for its return.
    with_database(cx, "work");
    cx.update(super::profile_shown);
    assert_eq!(reports(cx), work);
    with_database(cx, "home");
    cx.update(super::profile_shown);
    assert_eq!(reports(cx).iter().map(|u| u.agent).collect::<Vec<_>>(), [AgentId::Codex]);
}
