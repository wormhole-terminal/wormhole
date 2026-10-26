//! The chrome around the terminal (`regions`) is drawn again when a fact it shows changes, and left
//! alone by a frame that only a terminal's output asks for. The harness is `test_harness`.

use gpui_kit::accesskit::Role;
use gpui_kit::{Entity, TestAppContext};
use wormhole_core::config::TabId;
use wormhole_term::PtyEvent;

use crate::app_state::{OpenWindow, app, app_mut};
use crate::settings::SettingsStore;
use crate::terminal_view::TerminalView;
use crate::window::reported_tests::terminal;
use crate::window::test_harness::{
    activate, draw_replaying as draw, drive, hook, isolate_in, read, read_cx, rebuild, simulate_tree, start,
    tree,
};

const PANEL: usize = 1;
const TITLE_ROW: usize = 2;
const STATUS_BAR: usize = 3;

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// How many times each region has been built so far: sidebar header, panel, title row, status bar.
fn builds(cx: &mut TestAppContext, window: &OpenWindow) -> [usize; 4] {
    read_cx(cx, window, |ws, cx| ws.regions.builds(cx))
}

/// How many times the sidebar's list has been built so far (it is a view of its own, not a region).
fn list_builds(cx: &mut TestAppContext, window: &OpenWindow) -> usize {
    read_cx(cx, window, |ws, cx| ws.sidebar.read(cx).builds())
}

/// Output as the PTY's reader delivers it, to a terminal on screen.
fn output(cx: &mut TestAppContext, terminal: &Entity<TerminalView>) {
    cx.update(|cx| terminal.update(cx, |t, cx| t.event_for_test(PtyEvent::Output, cx)));
    cx.run_until_parked();
}

/// A window with two terminals and the status bar on, drawn until it settles.
fn settled_window(cx: &mut TestAppContext) -> OpenWindow {
    let window = start(cx, 2);
    cx.update(|cx| SettingsStore::update(cx, |settings| settings.status_bar = true));
    cx.run_until_parked();
    for _ in 0..3 {
        draw(cx, &window);
    }
    window
}

#[gpui_kit::test]
fn a_frame_only_a_terminal_asks_for_builds_none_of_the_chrome(cx: &mut TestAppContext) {
    if isolate("a_frame_only_a_terminal_asks_for_builds_none_of_the_chrome") {
        return;
    }
    let window = settled_window(cx);
    let shown = terminal(cx, &window, 0);
    // The first output turns the terminal busy, which the chrome shows.
    output(cx, &shown);
    for _ in 0..3 {
        draw(cx, &window);
    }
    let before = builds(cx, &window);
    assert!(before.iter().all(|&n| n > 0), "every region was drawn once: {before:?}");
    for _ in 0..20 {
        output(cx, &shown);
        draw(cx, &window);
    }
    assert_eq!(builds(cx, &window), before, "twenty frames of output left the chrome as it was");
}

/// GPUI fills the accessibility tree during every frame and, in the patched `gpui-pre`, replays a reused
/// view's nodes with the rest of it: with the tree built a frame only the terminal asked for still builds
/// none of the chrome, and the tree is the one a full rebuild gives (the sidebar's rows, the panel's tabs, the
/// status bar, with the same ids, links, labels and focus). The test platform has no assistive app, so the
/// test says one connected.
#[gpui_kit::test]
fn with_the_accessibility_tree_built_the_chrome_is_replayed_and_stays_in_the_tree(cx: &mut TestAppContext) {
    if isolate("with_the_accessibility_tree_built_the_chrome_is_replayed_and_stays_in_the_tree") {
        return;
    }
    let window = settled_window(cx);
    let shown = terminal(cx, &window, 0);
    output(cx, &shown);
    for _ in 0..3 {
        draw(cx, &window);
    }
    assert!(!read_cx(cx, &window, |ws, _| ws.regions.tree_seen()), "not before an app asks");
    simulate_tree(cx, &window, true);
    // A view prepainted before the tree was built has no nodes to replay: the first frames draw the
    // regions, the later ones replay them.
    for _ in 0..3 {
        draw(cx, &window);
    }
    assert!(read_cx(cx, &window, |ws, _| ws.regions.tree_seen()), "noted once the tree is built");
    // The reference: everything drawn afresh.
    rebuild(cx, &window);
    let reference = tree(cx, &window);
    assert!(reference.whole(), "{:?}", reference.problems);
    assert_eq!(reference.count(Role::TreeItem), 2, "the sidebar's rows: {:?}", reference.nodes);
    assert_eq!(reference.count(Role::TabList), 1, "the panel's tabs: {:?}", reference.nodes);
    assert_eq!(reference.count(Role::Toolbar), 1, "the status bar: {:?}", reference.nodes);
    let before = builds(cx, &window);
    for round in 0..5 {
        output(cx, &shown);
        draw(cx, &window);
        assert_eq!(tree(cx, &window), reference, "frame {round} of output");
    }
    assert_eq!(builds(cx, &window), before, "five frames of output left the chrome as it was");
}

/// A popover opens its content through `deferred`, which GPUI prepaints after the cached view that holds the
/// popover: the content stays in the tree, and answers, on the frames that replay the view. The profile
/// chip's menu is one (two profiles make the chip).
#[gpui_kit::test]
fn an_open_menu_stays_in_the_tree_on_frames_that_replay_the_chrome(cx: &mut TestAppContext) {
    use wormhole_core::config::profile::{NewProfile, StartFrom};

    use crate::app_state::AppState;
    use crate::sidebar::profile_chip::ProfilePage;
    if isolate("an_open_menu_stays_in_the_tree_on_frames_that_replay_the_chrome") {
        return;
    }
    let window = settled_window(cx);
    cx.update(|cx| {
        let new = NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults };
        AppState::create_profile(new, cx).expect("makes the profile");
    });
    simulate_tree(cx, &window, true);
    for _ in 0..3 {
        draw(cx, &window);
    }
    let sidebar = read_cx(cx, &window, |ws, _| ws.sidebar.clone());
    cx.update(|cx| sidebar.update(cx, |sidebar, cx| sidebar.open_profile_menu_for_test(cx)));
    // The popover shows its content from the frame after it has seen where its trigger is.
    for _ in 0..3 {
        draw(cx, &window);
    }
    rebuild(cx, &window);
    let reference = tree(cx, &window);
    assert!(reference.whole(), "{:?}", reference.problems);
    let open_in = reference.one(Role::Button, "Open a window in");
    assert!(!reference.labelled(Role::Button, "Settings").is_empty(), "the menu's last row is in the tree");

    // Frames only the terminal asked for replay the sidebar's region, and the menu with it.
    let shown = terminal(cx, &window, 0);
    let before = builds(cx, &window);
    for round in 0..4 {
        output(cx, &shown);
        draw(cx, &window);
        assert_eq!(tree(cx, &window), reference, "frame {round} of output with the menu open");
    }
    assert_eq!(builds(cx, &window), before, "the chrome was replayed");

    // And a row answers on such a frame: it opens the menu's second page.
    activate(cx, &window, open_in);
    assert_eq!(sidebar_page(cx, &sidebar), Some(ProfilePage::OpenIn));
    draw(cx, &window);
    draw(cx, &window);
    let second = tree(cx, &window);
    assert!(second.whole(), "{:?}", second.problems);
    assert!(!second.labelled(Role::Button, "Back").is_empty(), "its back row is there");
    assert!(second.labelled(Role::Button, "Settings").is_empty(), "and the first page's rows are gone");
}

fn sidebar_page(
    cx: &mut TestAppContext,
    sidebar: &Entity<crate::sidebar::Sidebar>,
) -> Option<crate::sidebar::profile_chip::ProfilePage> {
    cx.update(|cx| sidebar.read(cx).profile_page())
}

#[gpui_kit::test]
fn a_terminal_that_needs_you_draws_the_sidebar_and_the_status_bar_again(cx: &mut TestAppContext) {
    if isolate("a_terminal_that_needs_you_draws_the_sidebar_and_the_status_bar_again") {
        return;
    }
    let window = settled_window(cx);
    let before = builds(cx, &window);
    let list_before = list_builds(cx, &window);
    hook(cx, &window, 2, "PermissionRequest:Bash", Some("Approve cargo test?"));
    draw(cx, &window);
    let after = builds(cx, &window);
    assert!(list_builds(cx, &window) > list_before, "its row says so");
    assert!(after[STATUS_BAR] > before[STATUS_BAR], "and the bar counts it");
}

#[gpui_kit::test]
fn a_terminal_renamed_draws_the_title_row_and_the_list_again(cx: &mut TestAppContext) {
    if isolate("a_terminal_renamed_draws_the_title_row_and_the_list_again") {
        return;
    }
    let window = settled_window(cx);
    let before = builds(cx, &window);
    let list_before = list_builds(cx, &window);
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].custom_name = Some("renamed".into());
        ws.mark_dirty(cx);
    });
    draw(cx, &window);
    let after = builds(cx, &window);
    assert!(after[TITLE_ROW] > before[TITLE_ROW], "the row names the selected terminal");
    assert!(list_builds(cx, &window) > list_before, "and the list too");
}

#[gpui_kit::test]
fn another_tab_draws_the_panel_again(cx: &mut TestAppContext) {
    if isolate("another_tab_draws_the_panel_again") {
        return;
    }
    let window = settled_window(cx);
    let before = builds(cx, &window);
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::History, cx));
    draw(cx, &window);
    assert!(builds(cx, &window)[PANEL] > before[PANEL]);
}

/// A global the chrome reads changes without the window being told: the regions are.
#[gpui_kit::test]
fn a_setting_that_changes_draws_every_region_again(cx: &mut TestAppContext) {
    if isolate("a_setting_that_changes_draws_every_region_again") {
        return;
    }
    let window = settled_window(cx);
    let before = builds(cx, &window);
    // Not `update_settings`: that also refreshes the window, which would draw everything anyway.
    cx.update(|cx| SettingsStore::update(cx, |settings| settings.split_gaps = !settings.split_gaps));
    cx.run_until_parked();
    draw(cx, &window);
    let after = builds(cx, &window);
    for (region, (before, after)) in before.iter().zip(after).enumerate() {
        assert!(after > *before, "region {region} was not drawn again");
    }
}

/// Every write to the shared state marks it changed, whatever the write: callers write it every tick.
#[gpui_kit::test]
fn writing_the_apps_shared_state_without_changing_a_fact_the_chrome_draws_builds_no_region(
    cx: &mut TestAppContext,
) {
    if isolate("writing_the_apps_shared_state_without_changing_a_fact_the_chrome_draws_builds_no_region") {
        return;
    }
    let window = settled_window(cx);
    let before = builds(cx, &window);
    cx.update(|cx| {
        app_mut(cx).remember_project("/a/project".into());
    });
    cx.run_until_parked();
    // The tick's callers write it again each time: the project already leads.
    for _ in 0..3 {
        cx.update(|cx| {
            if !app(cx).leads_recent_projects(std::path::Path::new("/a/project")) {
                app_mut(cx).remember_project("/a/project".into());
            }
            app_mut(cx);
        });
        cx.run_until_parked();
        draw(cx, &window);
    }
    assert_eq!(builds(cx, &window), before, "nothing the chrome shows moved");
}

#[gpui_kit::test]
fn a_fact_of_the_apps_shared_state_that_changes_draws_every_region_again(cx: &mut TestAppContext) {
    if isolate("a_fact_of_the_apps_shared_state_that_changes_draws_every_region_again") {
        return;
    }
    let window = settled_window(cx);
    let before = builds(cx, &window);
    // The note after a first run without agents is closed for good: the sidebar reads it.
    cx.update(|cx| app_mut(cx).dismiss_tip(wormhole_core::onboarding::Tip::NoAgents));
    cx.run_until_parked();
    draw(cx, &window);
    let after = builds(cx, &window);
    for (region, (before, after)) in before.iter().zip(after).enumerate() {
        assert!(after > *before, "region {region} was not drawn again");
    }
}

/// The list notifies for its own drawing, an agent's spinner at 30 frames a second among it. GPUI marks a
/// notified view's ancestors dirty, so the list must not be a child of any cached region: its header is its
/// sibling.
#[gpui_kit::test]
fn the_list_drawing_itself_builds_none_of_the_cached_regions(cx: &mut TestAppContext) {
    if isolate("the_list_drawing_itself_builds_none_of_the_cached_regions") {
        return;
    }
    let window = settled_window(cx);
    let before = builds(cx, &window);
    let list_before = list_builds(cx, &window);
    for _ in 0..10 {
        cx.update(|cx| {
            let sidebar = window.workspace.read_with(cx, |ws, _| ws.sidebar.clone()).expect("open");
            sidebar.update(cx, |_, cx| cx.notify());
        });
        cx.run_until_parked();
        draw(cx, &window);
    }
    assert_eq!(
        builds(cx, &window),
        before,
        "a hover or a spinner's turn is the list's: no region (the sidebar's header among them) was built"
    );
    assert_eq!(list_builds(cx, &window), list_before + 10, "and the list itself was drawn each time");
}

/// What the list allocates when it draws itself with nothing moving, for a window of the terminals and the
/// group `list_at_rest_window` makes. The list's motion (springs, rows that come and go) adds to a frame
/// only while something moves, and at rest the comparison of its keys, which is no allocation more than the
/// list made before it had any motion. The budget is what it made then (543, the same before and after the
/// motion work, measured on the commit that preceded it) plus about a hundredth.
const LIST_AT_REST_ALLOCATIONS: u64 = 549;

/// Six terminals, two of them in a group, drawn until they settle, with motion on.
fn list_at_rest_window(cx: &mut TestAppContext) -> OpenWindow {
    let window = start(cx, 6);
    cx.update(|cx| cx.set_reduce_motion(false));
    drive(cx, &window, |ws, _, cx| ws.group_terminals(&[2, 3], cx));
    for _ in 0..4 {
        draw(cx, &window);
    }
    window
}

#[gpui_kit::test]
fn the_list_at_rest_stays_within_its_allocation_budget(cx: &mut TestAppContext) {
    if isolate("the_list_at_rest_stays_within_its_allocation_budget") {
        return;
    }
    use crate::perf::allocations::thread_counts;
    let window = list_at_rest_window(cx);
    let sidebar = window.workspace.read_with(cx, |ws, _| ws.sidebar.clone()).expect("open");
    // The list asks to be drawn, and the draw is only the list (the chrome replays).
    let mut made = Vec::new();
    for _ in 0..3 {
        cx.update(|cx| sidebar.update(cx, |_, cx| cx.notify()));
        let before = thread_counts();
        draw(cx, &window);
        made.push(thread_counts().since(before).allocations);
    }
    let rest = made[2];
    eprintln!("list at rest: {made:?} allocations");
    assert_eq!(made[1], made[2], "a list that does not change allocates the same each frame");
    assert!(
        rest <= LIST_AT_REST_ALLOCATIONS,
        "the list at rest made {rest} allocations (budget {LIST_AT_REST_ALLOCATIONS})"
    );
}

/// With the terminals in tabs the root reads the list to build the bar, which is therefore drawn for the list's
/// every notification (GPUI draws the root again), and the regions are left alone.
#[gpui_kit::test]
fn with_tabs_on_the_list_drawing_itself_draws_the_tab_bar_and_builds_no_region(cx: &mut TestAppContext) {
    if isolate("with_tabs_on_the_list_drawing_itself_draws_the_tab_bar_and_builds_no_region") {
        return;
    }
    use wormhole_core::view::sidebar::TerminalsIn;
    let window = settled_window(cx);
    cx.update(|cx| crate::settings::update_settings(cx, |s| s.terminals_in = TerminalsIn::Tabs));
    cx.run_until_parked();
    for _ in 0..3 {
        draw(cx, &window);
    }
    assert!(read(cx, &window, |ws| ws.tabs_on_top()), "the terminals are in tabs");
    let sidebar = cx.update(|cx| window.workspace.read_with(cx, |ws, _| ws.sidebar.clone()).expect("open"));
    let before = builds(cx, &window);
    for _ in 0..3 {
        let bar = |cx: &mut TestAppContext| cx.update(|cx| sidebar.read(cx).tab_bar_builds());
        let was = bar(cx);
        cx.update(|cx| sidebar.update(cx, |_, cx| cx.notify()));
        // The test platform draws a window that is dirty when the effects have run.
        cx.run_until_parked();
        assert_eq!(bar(cx), was + 1, "the list's notification drew the tab bar");
    }
    assert_eq!(builds(cx, &window), before, "and none of the regions");
}

#[gpui_kit::test]
fn the_colour_of_the_selected_terminals_group_changing_draws_the_title_row_again(cx: &mut TestAppContext) {
    if isolate("the_colour_of_the_selected_terminals_group_changing_draws_the_title_row_again") {
        return;
    }
    let window = settled_window(cx);
    drive(cx, &window, |ws, _, cx| {
        let id = ws.sessions[ws.active].id;
        ws.group_terminals(&[id], cx);
    });
    draw(cx, &window);
    let before = builds(cx, &window);
    // The list's own change, as a band's menu makes it: the list notifies, the window hears of what it reads.
    cx.update(|cx| {
        let sidebar = window.workspace.read_with(cx, |ws, _| ws.sidebar.clone()).expect("open");
        sidebar.update(cx, |sidebar, cx| {
            use wormhole_core::view::sidebar::GroupColor;
            let group = &sidebar.view().groups.0[0];
            let now = group.color();
            let other = if now == Some(GroupColor::Red) { GroupColor::Green } else { GroupColor::Red };
            let id = group.id;
            sidebar.view_mut().groups.set_color(id, Some(other));
            cx.notify();
        });
    });
    cx.run_until_parked();
    draw(cx, &window);
    assert!(builds(cx, &window)[TITLE_ROW] > before[TITLE_ROW], "the row wears the group's colour");
}

/// Allocations a frame of a two-terminal window with the status bar on may make when only a terminal asked
/// for it and the chrome is replayed. Measured: 566 allocations, the same on every run (it was 546 before
/// the toggles' strip fills and the Animate icons setting). The budget is that plus about a hundredth:
/// what is left is the root's own rebuild (the window's actions,
/// the overlay and menu layers, the pane probe, the sidebar's column with its header and list as the
/// replayed views it holds side by side) and the terminal.
const REPLAYED_FRAME_ALLOCATIONS: u64 = 572;

/// Allocations of a draw of the window.
fn allocations_of_draw(cx: &mut TestAppContext, window: &OpenWindow) -> u64 {
    use crate::perf::allocations::thread_counts;
    let before = thread_counts();
    draw(cx, window);
    thread_counts().since(before).allocations
}

/// Allocations of the frame the window draws when it is notified, for the same window: the cost the
/// replay saves. (GPUI draws a dirty window at the end of the update that dirtied it.)
fn allocations_of_notified_frame(cx: &mut TestAppContext, window: &OpenWindow) -> u64 {
    use crate::perf::allocations::thread_counts;
    let before = thread_counts();
    cx.update(|cx| crate::app_state::AppState::with_workspace(window, cx, |_, _, cx| cx.notify()))
        .expect("the window is open");
    thread_counts().since(before).allocations
}

#[gpui_kit::test]
fn a_frame_only_a_terminal_asks_for_stays_within_its_allocation_budget(cx: &mut TestAppContext) {
    if isolate("a_frame_only_a_terminal_asks_for_stays_within_its_allocation_budget") {
        return;
    }
    let window = settled_window(cx);
    let shown = terminal(cx, &window, 0);
    output(cx, &shown);
    for _ in 0..3 {
        draw(cx, &window);
    }
    let replayed = allocations_of_draw(cx, &window);
    let rebuilt = allocations_of_notified_frame(cx, &window);
    eprintln!("window frame, chrome replayed: {replayed} allocations; rebuilt: {rebuilt}");
    assert!(
        replayed <= REPLAYED_FRAME_ALLOCATIONS,
        "a frame with the chrome replayed made {replayed} allocations (budget {REPLAYED_FRAME_ALLOCATIONS})"
    );
    assert!(
        replayed * 3 < rebuilt,
        "replaying the chrome saves most of a frame's allocations ({replayed} of {rebuilt})"
    );
}

/// Words that move with the clock ("working · 4m") are the tick's, up to a tick late: the check compares what
/// they are made of, so a minute that turns between the tick and a frame is not a missing notify.
#[gpui_kit::test]
fn a_minute_turning_between_ticks_is_not_a_missing_notify(cx: &mut TestAppContext) {
    if isolate("a_minute_turning_between_ticks_is_not_a_missing_notify") {
        return;
    }
    let window = settled_window(cx);
    drive(cx, &window, |ws, window, cx| ws.tick(window, cx));
    hook(cx, &window, 1, "UserPromptSubmit", None);
    let working = terminal(cx, &window, 0);
    for status_bar in [true, false] {
        cx.update(|cx| SettingsStore::update(cx, |settings| settings.status_bar = status_bar));
        cx.update(|cx| {
            working.update(cx, |t, cx| {
                // Just short of the minute, which a few moments later is reached.
                t.backdate_status_for_test(std::time::Duration::from_millis(59_900));
                cx.notify();
            })
        });
        cx.run_until_parked();
        for _ in 0..3 {
            draw(cx, &window);
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
        // The words now read "1m" where the regions drew "<1m", and nothing has ticked: this draws the
        // regions as they were, and the check must let it pass.
        draw(cx, &window);
    }
}

/// What the check reports when `change` is made to the window without a notify and a frame is drawn.
fn report_of_a_missed_notify(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    change: impl FnOnce(&mut crate::window::Workspace) + 'static,
) -> String {
    // `drive` draws too (the effects it sets off end in a frame), so the change is made, and the frame
    // drawn, in one update.
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        cx.update(|cx| crate::app_state::AppState::with_workspace(window, cx, |ws, _, _| change(ws)))
            .expect("the window is open");
        draw(cx, window);
    }));
    match caught {
        Ok(()) => panic!("the check missed a change that was not told"),
        Err(payload) => payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(ToString::to_string))
            .unwrap_or_default(),
    }
}

/// The check itself: a fact a region shows that changes without notifying the workspace is found in the
/// next frame, by name; the same change with the notify is not.
#[gpui_kit::test]
fn a_fact_changed_without_notifying_is_found_by_the_check(cx: &mut TestAppContext) {
    if isolate("a_fact_changed_without_notifying_is_found_by_the_check") {
        return;
    }
    let window = settled_window(cx);
    let report = report_of_a_missed_notify(cx, &window, |ws| ws.right_tab = TabId::History);
    assert!(
        report.contains("Panel") && report.contains("tab:"),
        "the report names the region and the fact: {report}"
    );
}

/// The window's own state that a part draws besides the shared state's facts is checked too: the title
/// row's menu and the keys' places, the panel's scroll. A panic ends the window (GPUI leaves its update
/// poisoned), so each is a test of its own.
macro_rules! a_missed_notify_is_found {
    ($name:ident, $region:literal, $fact:literal, $change:expr) => {
        #[gpui_kit::test]
        fn $name(cx: &mut TestAppContext) {
            if isolate(stringify!($name)) {
                return;
            }
            let window = settled_window(cx);
            let report = report_of_a_missed_notify(cx, &window, $change);
            assert!(report.contains($region) && report.contains(concat!($fact, ":")), "{report}");
        }
    };
}

a_missed_notify_is_found!(a_ports_menu_opened_unannounced_is_found, "TitleRow", "ports menu", |ws| {
    ws.ports_menu = true
});
a_missed_notify_is_found!(a_title_button_moved_unannounced_is_found, "TitleRow", "button", |ws| {
    ws.areas.title = 1
});
a_missed_notify_is_found!(a_status_item_moved_unannounced_is_found, "StatusBar", "item", |ws| {
    ws.areas.status = 1
});
a_missed_notify_is_found!(a_panel_scrolled_unannounced_is_found, "Panel", "scroll", |ws| {
    ws.panel_scroll.set_offset(gpui_kit::point(gpui_kit::px(0.0), gpui_kit::px(-40.0)))
});
