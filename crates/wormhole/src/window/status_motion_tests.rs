//! A terminal's status changing in a real window (see `test_harness` for how each test runs in a child
//! process with scratch data), and the status bar's counts turning over: with each variant a change is
//! played and the row stands plain after it, Classic, Reduce Motion and a build without the Motion
//! feature change at once, a change draws the list and no region of the chrome, and a count that changes
//! rolls inside the bar alone. GPUI's animations run on the wall clock, which a test cannot advance, so
//! these check what the list holds and its timers, which the test clock moves.
use std::time::Duration;

use gpui_kit::{TestAppContext, px, size};
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::motion::pose::status;
use wormhole_core::view::motion::{MotionChoice, MotionChoices, StatusMotion};
use wormhole_core::view::sidebar::Urgency;

use crate::app_state::OpenWindow;
use crate::window::test_harness::{draw, draw_replaying, hook, isolate_in, read_cx, sidebar_of, start};

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

fn choose(cx: &mut TestAppContext, status: StatusMotion) {
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.motion = MotionChoices { status, ..MotionChoices::CLASSIC }
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

fn playing(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<(i64, StatusMotion)> {
    sidebar_of(cx, window, |sidebar| sidebar.statuses_playing())
}

/// An agent's hook for terminal 2, and the frames the window draws for it.
fn agent(cx: &mut TestAppContext, window: &OpenWindow, event: &str, message: Option<&str>) {
    hook(cx, window, 2, event, message);
    draw(cx, window);
    draw(cx, window);
}

/// What terminal 2 goes through, as the hooks say it: working, asking for input, finished, working again.
/// (Waiting out a change moves the clock the terminals' own timers keep, which settles a terminal that
/// has heard nothing for a while to a quiet one, so each change is judged by what the list showed before
/// it and shows after it.)
const STEPS: [(&str, Option<&str>); 4] = [
    ("UserPromptSubmit", None),
    ("Notification", Some("Allow Bash?")),
    ("Stop", None),
    ("UserPromptSubmit", None),
];

/// What terminal 2's tile says now.
fn shown(cx: &mut TestAppContext, window: &OpenWindow) -> Urgency {
    sidebar_of(cx, window, |sidebar| sidebar.shown_urgency(2)).expect("the terminal is listed")
}

#[gpui_kit::test]
fn a_status_change_plays_with_each_variant_the_row_has_it_for_and_stands_plain_after(
    cx: &mut TestAppContext,
) {
    if isolate("a_status_change_plays_with_each_variant_the_row_has_it_for_and_stands_plain_after") {
        return;
    }
    let window = open(cx, 2);
    for &variant in StatusMotion::ALL {
        choose(cx, variant);
        for (event, message) in STEPS {
            let from = shown(cx, &window);
            agent(cx, &window, event, message);
            let to = shown(cx, &window);
            let plays = !status::duration(variant, from, to).is_zero();
            assert_eq!(
                playing(cx, &window),
                if plays { vec![(2, variant)] } else { vec![] },
                "{variant:?}: {from:?} to {to:?}"
            );
            assert_eq!(sidebar_of(cx, &window, |s| s.statuses_busy()), plays, "{variant:?}");
            settle(cx, &window);
            assert!(playing(cx, &window).is_empty(), "{variant:?}: {to:?} stands plain once it is over");
        }
    }
}

#[gpui_kit::test]
fn a_change_that_comes_again_before_the_first_is_over_starts_over_and_ends_plain(cx: &mut TestAppContext) {
    if isolate("a_change_that_comes_again_before_the_first_is_over_starts_over_and_ends_plain") {
        return;
    }
    let window = open(cx, 2);
    choose(cx, StatusMotion::Crossfade);
    agent(cx, &window, "UserPromptSubmit", None);
    assert_eq!(playing(cx, &window), [(2, StatusMotion::Crossfade)]);
    // Before its 160 ms are up: the next status.
    cx.executor().advance_clock(Duration::from_millis(60));
    agent(cx, &window, "Notification", Some("Allow Bash?"));
    assert_eq!(playing(cx, &window), [(2, StatusMotion::Crossfade)], "one run for the row, the latest");
    // The first change's timer fires while the second is playing and leaves it alone.
    cx.executor().advance_clock(Duration::from_millis(120));
    cx.run_until_parked();
    assert_eq!(playing(cx, &window), [(2, StatusMotion::Crossfade)], "the second change is still playing");
    settle(cx, &window);
    assert!(playing(cx, &window).is_empty());
}

#[gpui_kit::test]
fn classic_the_motion_feature_off_and_reduce_motion_change_a_status_at_once(cx: &mut TestAppContext) {
    if isolate("classic_the_motion_feature_off_and_reduce_motion_change_a_status_at_once") {
        return;
    }
    let window = open(cx, 2);
    choose(cx, StatusMotion::Classic);
    agent(cx, &window, "UserPromptSubmit", None);
    assert!(playing(cx, &window).is_empty(), "Classic is a cut");

    choose(cx, StatusMotion::BadgePop);
    cx.update(|cx| cx.set_reduce_motion(true));
    agent(cx, &window, "Stop", None);
    assert!(playing(cx, &window).is_empty(), "Reduce Motion cuts it");
    cx.update(|cx| cx.set_reduce_motion(false));

    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.feature_flags.insert(FeatureFlag::Motion.key().to_string(), false);
        });
    });
    agent(cx, &window, "Notification", Some("Allow Bash?"));
    assert!(playing(cx, &window).is_empty(), "without the Motion feature the choice is not in force");
    assert!(!sidebar_of(cx, &window, |s| s.statuses_busy()));
}

#[gpui_kit::test]
fn a_setting_that_changes_every_tile_is_not_a_status_change(cx: &mut TestAppContext) {
    if isolate("a_setting_that_changes_every_tile_is_not_a_status_change") {
        return;
    }
    let window = open(cx, 2);
    choose(cx, StatusMotion::Crossfade);
    agent(cx, &window, "UserPromptSubmit", None);
    settle(cx, &window);
    // The sidebar's status dot off draws every tile as quiet: the same terminals, the same states.
    cx.update(|cx| crate::settings::update_settings(cx, |settings| settings.sidebar_status_dot = false));
    draw(cx, &window);
    draw(cx, &window);
    assert!(playing(cx, &window).is_empty(), "the setting did not change any terminal's status");
}

#[gpui_kit::test]
fn a_status_change_draws_the_list_and_no_region_and_stops_when_it_ends(cx: &mut TestAppContext) {
    if isolate("a_status_change_draws_the_list_and_no_region_and_stops_when_it_ends") {
        return;
    }
    let window = open(cx, 3);
    choose(cx, StatusMotion::Crossfade);
    // The hook changes what the chrome shows too (the title row, the bar), so let that frame be drawn first.
    agent(cx, &window, "UserPromptSubmit", None);
    assert!(!playing(cx, &window).is_empty());
    let regions = |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| ws.regions.builds(cx));
    let list = |cx: &mut TestAppContext| sidebar_of(cx, &window, |sidebar| sidebar.builds());
    let (chrome, drawn) = (regions(cx), list(cx));
    // An animation asks for its next frame by notifying the view it is in, here the list (GPUI's test
    // platform runs no such request itself): each of those frames draws the list and nothing else.
    for _ in 0..5 {
        cx.update(|cx| {
            let sidebar = window.workspace.read_with(cx, |ws, _| ws.sidebar.clone()).expect("open");
            sidebar.update(cx, |_, cx| cx.notify());
        });
        draw_replaying(cx, &window);
    }
    assert_eq!(regions(cx), chrome, "a tile changing is the list's to draw, not the chrome's");
    assert_eq!(list(cx), drawn + 5, "the list is drawn again on each");
    // Once the change has played out the list asks for nothing more.
    settle(cx, &window);
    let rest = (regions(cx), list(cx));
    for _ in 0..5 {
        draw_replaying(cx, &window);
    }
    assert_eq!((regions(cx), list(cx)), rest, "and a list at rest is not drawn again");
}
