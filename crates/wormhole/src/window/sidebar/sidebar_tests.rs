//! The terminal list pinned from outside, in a real window (GPUI's test platform; see `view_tests`
//! for how each test runs in a child process with scratch data): what a click, a drag, a key or the
//! modifier keys do to the list, whatever holds its state. The helpers at the top are the only place
//! a test names where the sidebar's state is kept.
use std::time::Duration;

use gpui_kit::{
    Bounds, Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px, size,
};
use wormhole_core::view::sidebar::Entry;

use crate::app_state::OpenWindow;
use crate::window::view_tests::{draw, drive, isolate_in, press, read, sidebar_of, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

// ---------------------------------------------------------------- where the state is kept

/// A window of `sessions` terminals, drawn twice at a size where the whole list shows.
fn open(cx: &mut TestAppContext, sessions: i64) -> OpenWindow {
    let window = start(cx, sessions);
    cx.simulate_window_resize(window.handle, size(px(1200.0), px(900.0)));
    draw(cx, &window);
    draw(cx, &window);
    window
}

fn pointer(cx: &mut TestAppContext, window: &OpenWindow) -> VisualTestContext {
    VisualTestContext::from_window(window.handle, cx)
}

/// Let the list's slide finish and draw the window where it lands.
fn settle(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.executor().advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    draw(cx, window);
    draw(cx, window);
}

/// The list's lines top to bottom: a group's head as `[name]`, a terminal as its id.
fn lines(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<String> {
    cx.update(|cx| {
        window.workspace.read_with(cx, |ws, cx| {
            let listing = ws.session_listing(cx);
            listing
                .entries
                .iter()
                .map(|entry| match *entry {
                    Entry::Band { band, .. } => format!("[{}]", listing.bands[band].label),
                    Entry::Row { session, .. } => ws.sessions[session].id.to_string(),
                })
                .collect()
        })
    })
    .expect("the window is open")
}

/// Where the list's line `at` was drawn.
fn line_bounds(cx: &mut TestAppContext, window: &OpenWindow, at: usize) -> Bounds<Pixels> {
    sidebar_of(cx, window, |sidebar| sidebar.scroll().bounds_for_item(at).expect("the line was drawn"))
}

fn selected(cx: &mut TestAppContext, window: &OpenWindow) -> i64 {
    read(cx, window, |ws| ws.sessions[ws.active].id)
}

fn click(cx: &mut TestAppContext, window: &OpenWindow, at: Point<Pixels>) {
    let mut pointer = pointer(cx, window);
    pointer.simulate_mouse_move(at, None, Modifiers::none());
    pointer.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    pointer.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    draw(cx, window);
}

/// The area that has the keyboard.
fn area(cx: &mut TestAppContext, window: &OpenWindow) -> Option<wormhole_core::view::focus::Area> {
    draw(cx, window);
    cx.update(|cx| {
        window.handle.update(cx, |_, win, cx| {
            let workspace = window.workspace.upgrade()?;
            workspace.read(cx).focus_area(win, cx)
        })
    })
    .ok()
    .flatten()
}

fn hints_showing(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.sidebar.read(cx).hints_showing(cx)))
        .expect("the window is open")
}

// ---------------------------------------------------------------- clicks

#[gpui_kit::test]
fn clicking_a_row_selects_its_terminal(cx: &mut TestAppContext) {
    if isolate("clicking_a_row_selects_its_terminal") {
        return;
    }
    let window = open(cx, 3);
    assert_eq!(selected(cx, &window), 1);
    let at = line_bounds(cx, &window, 2).center();
    click(cx, &window, at);
    assert_eq!(selected(cx, &window), 3);
    let at = line_bounds(cx, &window, 1).center();
    click(cx, &window, at);
    assert_eq!(selected(cx, &window), 2);
}

/// The × shows under the pointer, at the row's right end, and closes the terminal.
#[gpui_kit::test]
fn the_x_under_the_pointer_closes_its_row(cx: &mut TestAppContext) {
    if isolate("the_x_under_the_pointer_closes_its_row") {
        return;
    }
    let window = open(cx, 3);
    let row = line_bounds(cx, &window, 1);
    let x = row.origin.x + row.size.width - px(26.0);
    let y = row.origin.y + px(18.0);
    pointer(cx, &window).simulate_mouse_move(row.center(), None, Modifiers::none());
    draw(cx, &window);
    click(cx, &window, point(x, y));
    settle(cx, &window);
    let ids: Vec<i64> = read(cx, &window, |ws| ws.sessions.iter().map(|s| s.id).collect());
    assert_eq!(ids, [1, 3], "terminal 2 was closed");
}

#[gpui_kit::test]
fn a_groups_head_folds_and_opens_it_and_its_plus_adds_a_terminal_to_it(cx: &mut TestAppContext) {
    if isolate("a_groups_head_folds_and_opens_it_and_its_plus_adds_a_terminal_to_it") {
        return;
    }
    let window = open(cx, 3);
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[2, 3], cx));
    settle(cx, &window);
    assert_eq!(lines(cx, &window), ["1", "[Group 1]", "2", "3"]);
    let head = line_bounds(cx, &window, 1);
    click(cx, &window, point(head.origin.x + px(120.0), head.center().y));
    settle(cx, &window);
    assert_eq!(lines(cx, &window), ["1", "[Group 1]"], "folded: its terminals are out of the list");
    assert!(sidebar_of(cx, &window, |sidebar| sidebar.view().is_group_folded(group)));
    click(cx, &window, point(head.origin.x + px(120.0), head.center().y));
    settle(cx, &window);
    assert_eq!(lines(cx, &window), ["1", "[Group 1]", "2", "3"], "and open again");

    // The + at the head's right end, under the pointer, starts a terminal in the group.
    let head = line_bounds(cx, &window, 1);
    pointer(cx, &window).simulate_mouse_move(head.center(), None, Modifiers::none());
    draw(cx, &window);
    click(cx, &window, point(head.origin.x + head.size.width - px(50.0), head.center().y));
    settle(cx, &window);
    assert_eq!(read(cx, &window, |ws| ws.sessions.len()), 4, "a terminal was made");
    let made = read(cx, &window, |ws| ws.sessions.last().expect("a terminal").id);
    assert_eq!(sidebar_of(cx, &window, |sidebar| sidebar.view().groups.group_of(made)), Some(group));
}

// ---------------------------------------------------------------- drags

#[gpui_kit::test]
fn a_group_dragged_by_its_head_lands_after_the_other_group(cx: &mut TestAppContext) {
    if isolate("a_group_dragged_by_its_head_lands_after_the_other_group") {
        return;
    }
    let window = open(cx, 4);
    let (first, second) =
        drive(cx, &window, |ws, _, cx| (ws.group_terminals(&[1], cx), ws.group_terminals(&[2], cx)));
    settle(cx, &window);
    let before = lines(cx, &window);
    assert_eq!(before, ["[Group 1]", "1", "[Group 2]", "2", "3", "4"]);
    // Pick the first head up and drop it on the lower part of the second group's row.
    let from = line_bounds(cx, &window, 0).center();
    let over = line_bounds(cx, &window, 3);
    let to = point(over.center().x, over.origin.y + over.size.height - px(4.0));
    let mut pointer = pointer(cx, &window);
    pointer.simulate_mouse_move(from, None, Modifiers::none());
    pointer.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    for at in [point(from.x, from.y + px(30.0)), to] {
        draw(cx, &window);
        pointer.simulate_mouse_move(at, MouseButton::Left, Modifiers::none());
    }
    draw(cx, &window);
    assert!(read(cx, &window, |ws| ws.group_drag == Some(first)), "the group is in the air");
    pointer.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    settle(cx, &window);
    assert_eq!(read(cx, &window, |ws| ws.group_drag), None);
    assert_eq!(lines(cx, &window), ["[Group 2]", "2", "[Group 1]", "1", "3", "4"], "{second} is first now");
}

#[gpui_kit::test]
fn a_row_dropped_on_a_groups_head_joins_the_group(cx: &mut TestAppContext) {
    if isolate("a_row_dropped_on_a_groups_head_joins_the_group") {
        return;
    }
    let window = open(cx, 3);
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[2], cx));
    settle(cx, &window);
    assert_eq!(lines(cx, &window), ["1", "[Group 1]", "2", "3"]);
    let from = line_bounds(cx, &window, 3).center();
    let head = line_bounds(cx, &window, 1);
    let to = head.center();
    let mut pointer = pointer(cx, &window);
    pointer.simulate_mouse_move(from, None, Modifiers::none());
    pointer.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    for at in [point(from.x, from.y - px(30.0)), to] {
        draw(cx, &window);
        pointer.simulate_mouse_move(at, MouseButton::Left, Modifiers::none());
    }
    draw(cx, &window);
    pointer.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    settle(cx, &window);
    assert_eq!(sidebar_of(cx, &window, |sidebar| sidebar.view().groups.group_of(3)), Some(group));
}

// ---------------------------------------------------------------- the keycaps

/// ⌘ held on its own for a moment shows the ⌘1–9 keycaps; any other key, letting go and the window
/// losing the front hide them, and the setting turns them off.
#[gpui_kit::test]
fn holding_command_shows_the_keycaps_and_a_key_a_release_or_a_blur_hides_them(cx: &mut TestAppContext) {
    if isolate("holding_command_shows_the_keycaps_and_a_key_a_release_or_a_blur_hides_them") {
        return;
    }
    let window = open(cx, 2);
    let hold = |cx: &mut TestAppContext, modifiers: Modifiers| {
        pointer(cx, &window).simulate_modifiers_change(modifiers);
        cx.run_until_parked();
    };
    // The hold's clock is the real one.
    let wait = || std::thread::sleep(Duration::from_millis(300));
    assert!(!hints_showing(cx, &window));
    hold(cx, Modifiers::command());
    assert!(!hints_showing(cx, &window), "not at once: a quick ⌘C never flashes them");
    wait();
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    assert!(hints_showing(cx, &window), "after a moment");
    hold(cx, Modifiers::none());
    assert!(!hints_showing(cx, &window), "gone on release");

    // The keys are in the list, where a letter is no echo of the shell's.
    for _ in 0..wormhole_core::view::focus::Area::ALL.len() {
        if area(cx, &window) == Some(wormhole_core::view::focus::Area::Sidebar) {
            break;
        }
        press(cx, &window, "f6");
    }
    hold(cx, Modifiers::command());
    wait();
    assert!(hints_showing(cx, &window));
    press(cx, &window, "x");
    assert!(!hints_showing(cx, &window), "gone on any other key");
    hold(cx, Modifiers::none());

    hold(cx, Modifiers::command());
    wait();
    drive(cx, &window, |ws, _, cx| ws.hints_blur(cx));
    assert!(!hints_showing(cx, &window), "gone when the window loses the front");
    hold(cx, Modifiers::none());

    cx.update(|cx| crate::settings::update_settings(cx, |s| s.sidebar_cmd_hints = false));
    hold(cx, Modifiers::command());
    wait();
    assert!(!hints_showing(cx, &window), "off in Settings");
}

// ---------------------------------------------------------------- the keys

/// Letters typed in the list jump to the next name that starts with them.
#[gpui_kit::test]
fn letters_typed_in_the_list_jump_to_the_next_name_that_starts_with_them(cx: &mut TestAppContext) {
    use wormhole_core::view::focus::Area;

    use crate::window::sidebar::SidebarStop;
    if isolate("letters_typed_in_the_list_jump_to_the_next_name_that_starts_with_them") {
        return;
    }
    let window = open(cx, 3);
    drive(cx, &window, |ws, _, cx| {
        for (ix, name) in ["alpha", "beta", "bravo"].into_iter().enumerate() {
            ws.sessions[ix].custom_name = Some(name.to_string());
        }
        ws.mark_dirty(cx);
    });
    draw(cx, &window);
    // F6 until the list has the keys.
    for _ in 0..Area::ALL.len() {
        if area(cx, &window) == Some(Area::Sidebar) {
            break;
        }
        draw(cx, &window);
        press(cx, &window, "f6");
    }
    let at = |cx: &mut TestAppContext| sidebar_of(cx, &window, |sidebar| sidebar.keys_at().cloned());
    press(cx, &window, "b");
    assert_eq!(at(cx), Some(SidebarStop::Row(2)), "the first name starting with b");
    press(cx, &window, "r");
    assert_eq!(at(cx), Some(SidebarStop::Row(3)), "b r: bravo");
}

// ---------------------------------------------------------------- the tab bar

#[gpui_kit::test]
fn with_the_terminals_in_tabs_the_groups_are_stepped_and_numbered(cx: &mut TestAppContext) {
    use wormhole_core::view::sidebar::TerminalsIn;
    if isolate("with_the_terminals_in_tabs_the_groups_are_stepped_and_numbered") {
        return;
    }
    let window = open(cx, 4);
    drive(cx, &window, |ws, _, cx| {
        ws.group_terminals(&[1, 2], cx);
        ws.group_terminals(&[3], cx);
    });
    cx.update(|cx| crate::settings::update_settings(cx, |s| s.terminals_in = TerminalsIn::Tabs));
    settle(cx, &window);
    assert!(read(cx, &window, |ws| ws.tabs_on_top() && !ws.sidebar_shown()), "tabs, no sidebar");
    // The selected terminal (4) is in no group, the last set.
    drive(cx, &window, |ws, window, cx| ws.go_to_group(1, window, cx));
    assert!(matches!(selected(cx, &window), 1 | 2), "⌃⌥1: the first group");
    drive(cx, &window, |ws, window, cx| ws.step_group(true, window, cx));
    assert_eq!(selected(cx, &window), 3, "next: the second group");
    drive(cx, &window, |ws, window, cx| ws.step_group(true, window, cx));
    assert_eq!(selected(cx, &window), 4, "next: the terminals in no group");
    drive(cx, &window, |ws, window, cx| ws.step_group(false, window, cx));
    assert_eq!(selected(cx, &window), 3, "and back");
    // Groups come back to the sidebar with the setting.
    cx.update(|cx| crate::settings::update_settings(cx, |s| s.terminals_in = TerminalsIn::Sidebar));
    settle(cx, &window);
    assert!(read(cx, &window, |ws| !ws.tabs_on_top() && ws.sidebar_shown()));
}

// ---------------------------------------------------------------- the profile chip

#[gpui_kit::test]
fn the_profile_chip_is_drawn_only_while_there_are_two_profiles(cx: &mut TestAppContext) {
    use wormhole_core::config::profile::{NewProfile, StartFrom};

    use crate::app_state::AppState;
    if isolate("the_profile_chip_is_drawn_only_while_there_are_two_profiles") {
        return;
    }
    let window = open(cx, 1);
    assert!(
        !cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.profile_chip_shown(cx))).expect("open")
    );
    cx.update(|cx| {
        let new = NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults };
        AppState::create_profile(new, cx).expect("makes the profile");
    });
    assert!(
        cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.profile_chip_shown(cx))).expect("open")
    );
    draw(cx, &window);
    // Its menu opens and closes with the chip.
    assert!(sidebar_of(cx, &window, |sidebar| !sidebar.profile_menu_open()));
}

// ---------------------------------------------------------------- scrolling

#[gpui_kit::test]
fn the_wheel_scrolls_a_list_longer_than_the_window(cx: &mut TestAppContext) {
    if isolate("the_wheel_scrolls_a_list_longer_than_the_window") {
        return;
    }
    let window = start(cx, 30);
    cx.simulate_window_resize(window.handle, size(px(1200.0), px(500.0)));
    draw(cx, &window);
    draw(cx, &window);
    assert_eq!(sidebar_of(cx, &window, |sidebar| f32::from(sidebar.scroll().offset().y)), 0.0);
    let at = line_bounds(cx, &window, 0).center();
    pointer(cx, &window).simulate_event(gpui_kit::ScrollWheelEvent {
        position: at,
        delta: gpui_kit::ScrollDelta::Pixels(point(px(0.0), px(-200.0))),
        modifiers: Modifiers::none(),
        touch_phase: gpui_kit::TouchPhase::Moved,
    });
    cx.run_until_parked();
    draw(cx, &window);
    assert!(
        sidebar_of(cx, &window, |sidebar| f32::from(sidebar.scroll().offset().y)) < 0.0,
        "the list moved up"
    );
}

// ---------------------------------------------------------------- the frame

/// A window of twenty terminals in three groups (and four in none), drawn twice so what a window keeps
/// is built.
fn crowded(cx: &mut TestAppContext) -> OpenWindow {
    let window = open(cx, 20);
    drive(cx, &window, |ws, _, cx| {
        ws.group_terminals(&[1, 2, 3, 4, 5, 6], cx);
        ws.group_terminals(&[7, 8, 9, 10, 11, 12], cx);
        ws.group_terminals(&[13, 14, 15, 16], cx);
    });
    settle(cx, &window);
    window
}

/// Allocations a frame of the crowded window may make. Measured before the sidebar left the
/// `Workspace` it was 10,709 allocations (5.95 MB, the same on every run), and 10,713 after it became a
/// child view; a worktree lookup that hits its cache allocates nothing since (10,693). The budget is that
/// plus a little: a cost per row per frame shows at once.
const CROWDED_FRAME_ALLOCATIONS: u64 = 10_710;

#[gpui_kit::test]
fn a_frame_of_twenty_terminals_in_three_groups_stays_within_its_allocation_budget(cx: &mut TestAppContext) {
    use crate::perf::allocations::thread_counts;
    if isolate("a_frame_of_twenty_terminals_in_three_groups_stays_within_its_allocation_budget") {
        return;
    }
    let window = crowded(cx);
    draw(cx, &window);
    let before = thread_counts();
    draw(cx, &window);
    let drawn = thread_counts().since(before);
    report(&format!("crowded frame: {} allocations, {} bytes", drawn.allocations, drawn.bytes));
    assert!(
        drawn.allocations <= CROWDED_FRAME_ALLOCATIONS,
        "a frame of the crowded window made {} allocations (budget {CROWDED_FRAME_ALLOCATIONS})",
        drawn.allocations
    );
}

/// Where a measurement goes besides stderr (which an isolated test's parent keeps): `WORMHOLE_FRAME_REPORT`.
fn report(line: &str) {
    eprintln!("{line}");
    if let Some(path) = std::env::var_os("WORMHOLE_FRAME_REPORT") {
        use std::io::Write as _;
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }
}

/// The sidebar's part of a frame, by difference: the crowded window drawn with the sidebar and without
/// it, the median of several runs of many frames. Run by hand before and after a change to the sidebar
/// (`WORMHOLE_FRAME_REPORT=/tmp/frame.txt cargo test … the_sidebars_part_of_a_frame`); timings are not a gate.
#[gpui_kit::test]
fn the_sidebars_part_of_a_frame(cx: &mut TestAppContext) {
    use std::time::Instant;
    // A measurement, only when asked for with `WORMHOLE_FRAME_REPORT=<file>`.
    if std::env::var_os("WORMHOLE_FRAME_REPORT").is_none() || isolate("the_sidebars_part_of_a_frame") {
        return;
    }
    let window = crowded(cx);
    let frames = 300;
    let run = |cx: &mut TestAppContext| -> f64 {
        let started = Instant::now();
        for _ in 0..frames {
            drive(cx, &window, |ws, _, cx| ws.mark_dirty(cx));
            draw(cx, &window);
        }
        started.elapsed().as_secs_f64() * 1e6 / f64::from(frames)
    };
    let median = |mut runs: Vec<f64>| {
        runs.sort_by(f64::total_cmp);
        runs[runs.len() / 2]
    };
    let with: Vec<f64> = (0..7).map(|_| run(cx)).collect();
    drive(cx, &window, |ws, _, cx| ws.toggle_sidebar(cx));
    settle(cx, &window);
    let without: Vec<f64> = (0..7).map(|_| run(cx)).collect();
    let (with, without) = (median(with), median(without));
    report(&format!(
        "frame with the sidebar {with:.0} us, without {without:.0} us, the sidebar's part {:.0} us",
        with - without
    ));
}
