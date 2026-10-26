//! The terminal list's motion in a real window (see `test_harness` for how each test runs in a child
//! process with scratch data): a terminal that opens or closes and a group that folds or opens end at the
//! layout the list has whatever the person chose, the rows that leave are not live while they are drawn,
//! and Reduce Motion cuts it all. GPUI's animations run on the wall clock, which a test cannot advance,
//! so these check what the list holds (the lines it draws, the rows it keeps, whether it is busy), and
//! its timers, which the test clock moves.
use std::time::Duration;

use gpui_kit::accesskit::Role;
use gpui_kit::{TestAppContext, px, size};
use wormhole_core::view::motion::{FoldMotion, InsertMotion, MotionChoice, MotionChoices};
use wormhole_core::view::sidebar::Entry;

use crate::app_state::OpenWindow;
use crate::window::test_harness::{draw, drive, isolate_in, read, sidebar_of, simulate_tree, start, tree};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A window of `sessions` terminals with motion on (the harness starts with Reduce Motion), drawn at rest.
fn open(cx: &mut TestAppContext, sessions: i64) -> OpenWindow {
    let window = start(cx, sessions);
    cx.update(|cx| cx.set_reduce_motion(false));
    cx.simulate_window_resize(window.handle, size(px(1200.0), px(900.0)));
    draw(cx, &window);
    draw(cx, &window);
    window
}

fn choose(cx: &mut TestAppContext, insert: InsertMotion, fold: FoldMotion) {
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.motion = MotionChoices { insert, fold, ..settings.motion }
        });
    });
}

/// Run what is left of every motion on the clock the timers keep, and draw where it lands.
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

/// How many lines the list has drawn, the rows that are leaving among them, and the line under them.
fn children(cx: &mut TestAppContext, window: &OpenWindow) -> usize {
    sidebar_of(cx, window, |sidebar| sidebar.scroll().children_count())
}

fn busy(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    sidebar_of(cx, window, |sidebar| sidebar.motion_busy())
}

fn leaving(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<i64> {
    sidebar_of(cx, window, |sidebar| sidebar.leaving_ids())
}

fn close(cx: &mut TestAppContext, window: &OpenWindow, id: i64) {
    drive(cx, window, |ws, window, cx| {
        let ix = ws.find_session(id).expect("a row");
        ws.close_session(ix, window, cx);
    });
    draw(cx, window);
    draw(cx, window);
}

fn open_one(cx: &mut TestAppContext, window: &OpenWindow) {
    drive(cx, window, |ws, window, cx| ws.new_session(window, cx));
    draw(cx, window);
    draw(cx, window);
}

#[gpui_kit::test]
fn a_terminal_opens_into_the_list_with_each_variant_and_the_list_rests_after(cx: &mut TestAppContext) {
    if isolate("a_terminal_opens_into_the_list_with_each_variant_and_the_list_rests_after") {
        return;
    }
    let window = open(cx, 2);
    for &variant in InsertMotion::ALL {
        choose(cx, variant, FoldMotion::Classic);
        let (before, drawn) = (lines(cx, &window), children(cx, &window));
        open_one(cx, &window);
        let after = lines(cx, &window);
        assert_eq!(after.len(), before.len() + 1, "{variant:?}");
        assert_eq!(children(cx, &window), drawn + 1, "{variant:?}: the new row is one more line");
        assert!(busy(cx, &window), "{variant:?}: the row comes in");
        assert!(leaving(cx, &window).is_empty(), "{variant:?}: nothing leaves");
        settle(cx, &window);
        assert_eq!(lines(cx, &window), after, "{variant:?}: it ends where the listing has it");
        assert!(!busy(cx, &window), "{variant:?}: and nothing is left going");
        assert_eq!(children(cx, &window), drawn + 1);
    }
}

#[gpui_kit::test]
fn a_terminal_that_closes_stays_drawn_while_it_goes_and_is_gone_after(cx: &mut TestAppContext) {
    if isolate("a_terminal_that_closes_stays_drawn_while_it_goes_and_is_gone_after") {
        return;
    }
    let window = open(cx, 3);
    for &variant in InsertMotion::ALL {
        choose(cx, variant, FoldMotion::Classic);
        let drawn = children(cx, &window);
        let victim = lines(cx, &window)[1].parse().expect("a terminal's id");
        close(cx, &window, victim);
        let cut = variant == InsertMotion::Classic;
        assert_eq!(leaving(cx, &window), if cut { vec![] } else { vec![victim] }, "{variant:?}");
        assert_eq!(
            children(cx, &window),
            if cut { drawn - 1 } else { drawn },
            "{variant:?}: a row that goes is drawn until it is over, a cut one is not"
        );
        assert!(!lines(cx, &window).contains(&victim.to_string()), "{variant:?}: the listing has it no more");
        assert!(busy(cx, &window), "{variant:?}");
        settle(cx, &window);
        assert!(leaving(cx, &window).is_empty(), "{variant:?}: no stale row after the exit");
        assert_eq!(children(cx, &window), drawn - 1, "{variant:?}");
        assert!(!busy(cx, &window), "{variant:?}");
        // Back to three for the next.
        open_one(cx, &window);
        settle(cx, &window);
    }
}

#[gpui_kit::test]
fn a_row_that_is_leaving_is_not_a_terminal_to_voiceover(cx: &mut TestAppContext) {
    if isolate("a_row_that_is_leaving_is_not_a_terminal_to_voiceover") {
        return;
    }
    let window = open(cx, 3);
    choose(cx, InsertMotion::FromSide, FoldMotion::Classic);
    simulate_tree(cx, &window, true);
    draw(cx, &window);
    assert_eq!(tree(cx, &window).count(Role::TreeItem), 3);
    close(cx, &window, 2);
    assert_eq!(leaving(cx, &window), [2], "the row is still drawn");
    let during = tree(cx, &window);
    assert!(during.whole(), "{:?}", during.problems);
    assert_eq!(during.count(Role::TreeItem), 2, "but only the two that are in the list are tree items");
    settle(cx, &window);
    assert_eq!(tree(cx, &window).count(Role::TreeItem), 2);
}

#[gpui_kit::test]
fn a_group_folds_and_opens_with_each_variant_and_ends_as_the_listing_says(cx: &mut TestAppContext) {
    if isolate("a_group_folds_and_opens_with_each_variant_and_ends_as_the_listing_says") {
        return;
    }
    let window = open(cx, 3);
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[1, 2], cx));
    draw(cx, &window);
    settle(cx, &window);
    let open_lines = lines(cx, &window);
    assert_eq!(open_lines.len(), 4, "the head, its two rows and the terminal under it");
    for &variant in FoldMotion::ALL {
        choose(cx, InsertMotion::Classic, variant);
        let drawn = children(cx, &window);
        drive(cx, &window, |ws, _, cx| ws.toggle_group_band(group, cx));
        draw(cx, &window);
        draw(cx, &window);
        let cut = variant == FoldMotion::Classic;
        assert_eq!(
            lines(cx, &window).len(),
            2,
            "{variant:?}: folded, the list holds the head and the terminal"
        );
        assert_eq!(
            leaving(cx, &window),
            if cut { vec![] } else { vec![1, 2] },
            "{variant:?}: the rows tuck or close before they go"
        );
        assert_eq!(children(cx, &window), if cut { drawn - 2 } else { drawn }, "{variant:?}");
        assert!(busy(cx, &window), "{variant:?}");
        settle(cx, &window);
        assert!(leaving(cx, &window).is_empty() && !busy(cx, &window), "{variant:?}: folded and still");
        assert_eq!(children(cx, &window), drawn - 2, "{variant:?}");

        drive(cx, &window, |ws, _, cx| ws.toggle_group_band(group, cx));
        draw(cx, &window);
        draw(cx, &window);
        assert_eq!(lines(cx, &window), open_lines, "{variant:?}: open again");
        assert!(busy(cx, &window), "{variant:?}: the rows come in");
        assert!(leaving(cx, &window).is_empty(), "{variant:?}");
        settle(cx, &window);
        assert!(!busy(cx, &window), "{variant:?}: and rest");
        assert_eq!(children(cx, &window), drawn);
    }
}

#[gpui_kit::test]
fn reduce_motion_cuts_a_terminal_closing_opening_and_a_group_folding(cx: &mut TestAppContext) {
    if isolate("reduce_motion_cuts_a_terminal_closing_opening_and_a_group_folding") {
        return;
    }
    let window = open(cx, 3);
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[1, 2], cx));
    settle(cx, &window);
    choose(cx, InsertMotion::FromSide, FoldMotion::Tuck);
    cx.update(|cx| cx.set_reduce_motion(true));
    close(cx, &window, 3);
    assert!(leaving(cx, &window).is_empty() && !busy(cx, &window), "a closed row is gone at once");
    open_one(cx, &window);
    assert!(!busy(cx, &window), "an opened row is there at once");
    drive(cx, &window, |ws, _, cx| ws.toggle_group_band(group, cx));
    draw(cx, &window);
    draw(cx, &window);
    assert!(leaving(cx, &window).is_empty() && !busy(cx, &window), "a folded group's rows are gone at once");
    drive(cx, &window, |ws, _, cx| ws.toggle_group_band(group, cx));
    draw(cx, &window);
    draw(cx, &window);
    assert!(!busy(cx, &window));
    assert_eq!(read(cx, &window, |ws| ws.sessions.len()), 3);
}

#[gpui_kit::test]
fn a_list_at_rest_does_not_draw_again_and_stops_when_the_motion_ends(cx: &mut TestAppContext) {
    if isolate("a_list_at_rest_does_not_draw_again_and_stops_when_the_motion_ends") {
        return;
    }
    let window = open(cx, 3);
    choose(cx, InsertMotion::Reveal, FoldMotion::Classic);
    settle(cx, &window);
    let builds = |cx: &mut TestAppContext| sidebar_of(cx, &window, |sidebar| sidebar.builds());
    let at_rest = builds(cx);
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    assert_eq!(builds(cx), at_rest, "nothing draws the list again while it rests");
    close(cx, &window, 2);
    settle(cx, &window);
    let after = builds(cx);
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    assert_eq!(builds(cx), after, "and it rests again when the motion has ended");
}
