//! The chrome around the terminal (`regions`) is drawn again when a fact it shows changes, and left
//! alone by a frame that only a terminal's output asks for. The harness is `test_harness`.

use gpui_kit::{Entity, TestAppContext};
use wormhole_core::config::TabId;
use wormhole_term::PtyEvent;

use crate::app_state::{OpenWindow, app_mut};
use crate::settings::SettingsStore;
use crate::terminal_view::TerminalView;
use crate::window::reported_tests::terminal;
use crate::window::test_harness::{draw_replaying as draw, drive, hook, isolate_in, read_cx, start};

const SIDEBAR: usize = 0;
const PANEL: usize = 1;
const TITLE_ROW: usize = 2;
const STATUS_BAR: usize = 3;

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// How many times each region has been built so far: sidebar, panel, title row, status bar.
fn builds(cx: &mut TestAppContext, window: &OpenWindow) -> [usize; 4] {
    read_cx(cx, window, |ws, cx| ws.regions.builds(cx))
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

#[gpui_kit::test]
fn a_terminal_that_needs_you_draws_the_sidebar_and_the_status_bar_again(cx: &mut TestAppContext) {
    if isolate("a_terminal_that_needs_you_draws_the_sidebar_and_the_status_bar_again") {
        return;
    }
    let window = settled_window(cx);
    let before = builds(cx, &window);
    hook(cx, &window, 2, "PermissionRequest:Bash", Some("Approve cargo test?"));
    draw(cx, &window);
    let after = builds(cx, &window);
    assert!(after[SIDEBAR] > before[SIDEBAR], "its row says so");
    assert!(after[STATUS_BAR] > before[STATUS_BAR], "and the bar counts it");
}

#[gpui_kit::test]
fn a_terminal_renamed_draws_the_title_row_and_the_list_again(cx: &mut TestAppContext) {
    if isolate("a_terminal_renamed_draws_the_title_row_and_the_list_again") {
        return;
    }
    let window = settled_window(cx);
    let before = builds(cx, &window);
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].custom_name = Some("renamed".into());
        ws.mark_dirty(cx);
    });
    draw(cx, &window);
    let after = builds(cx, &window);
    assert!(after[TITLE_ROW] > before[TITLE_ROW], "the row names the selected terminal");
    assert!(after[SIDEBAR] > before[SIDEBAR], "and the list too");
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

#[gpui_kit::test]
fn the_apps_shared_state_changing_draws_every_region_again(cx: &mut TestAppContext) {
    if isolate("the_apps_shared_state_changing_draws_every_region_again") {
        return;
    }
    let window = settled_window(cx);
    let before = builds(cx, &window);
    cx.update(|cx| {
        app_mut(cx);
    });
    cx.run_until_parked();
    draw(cx, &window);
    let after = builds(cx, &window);
    for (region, (before, after)) in before.iter().zip(after).enumerate() {
        assert!(after > *before, "region {region} was not drawn again");
    }
}

/// Allocations a frame of a two-terminal window with the status bar on may make when only a terminal asked
/// for it and the chrome is replayed. Measured: 533 allocations (203 KB), the same on every run. The
/// budget is that plus about a hundredth: what is left is the root's own rebuild (the window's actions,
/// the overlay and menu layers, the pane probe) and the terminal.
const REPLAYED_FRAME_ALLOCATIONS: u64 = 539;

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
