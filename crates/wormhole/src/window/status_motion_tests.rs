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
use crate::style::motion::ANIMATION_FPS;
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
    // A long one: the wall clock, which the animation runs on, must not end it before the test has looked.
    choose(cx, StatusMotion::Wash);
    // The hook changes what the chrome shows too (the title row, the bar), so let that frame be drawn first.
    agent(cx, &window, "UserPromptSubmit", None);
    assert!(!playing(cx, &window).is_empty());
    let regions = |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| ws.regions.builds(cx));
    let list = |cx: &mut TestAppContext| sidebar_of(cx, &window, |sidebar| sidebar.builds());
    let chrome = regions(cx);
    // The tile's animation asked for its next frame while it was drawn, which notifies the view it is in:
    // each of those frames draws the list and nothing else.
    for _ in 0..5 {
        let before_frame = list(cx);
        frame_passes(cx, &window);
        draw_replaying(cx, &window);
        assert!(list(cx) > before_frame, "the frame the animation asked for draws the list");
    }
    assert_eq!(regions(cx), chrome, "a tile changing is the list's to draw, not the chrome's");
    // Once the change has played out the list asks for nothing more.
    settle(cx, &window);
    let rest = (regions(cx), list(cx));
    for _ in 0..5 {
        draw_replaying(cx, &window);
    }
    assert_eq!((regions(cx), list(cx)), rest, "and a list at rest is not drawn again");
}

/// A window with the status bar on and two terminals, drawn until it settles.
fn with_bar(cx: &mut TestAppContext) -> OpenWindow {
    let window = open(cx, 3);
    cx.update(|cx| crate::settings::update_settings(cx, |settings| settings.status_bar = true));
    cx.run_until_parked();
    for _ in 0..3 {
        draw(cx, &window);
    }
    window
}

/// The time between a 120 Hz display's frames.
const REFRESH: Duration = Duration::from_millis(8);

/// One frame of a 120 Hz display: the clock the executor keeps moves on by [`REFRESH`] and the window's frame
/// callbacks run. An animation with a frame cap notifies the view it is in on the display frame nearest its
/// due time, so this is the animation's own notification, not a test's.
fn display_frame(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.executor().advance_clock(REFRESH);
    cx.run_until_parked();
    cx.update(|cx| window.handle.update(cx, |_, window, cx| window.simulate_next_frame(cx)))
        .expect("the window is open");
    cx.run_until_parked();
}

/// A frame's time at `ANIMATION_FPS` goes by in display frames: an animation's next frame comes in it.
fn frame_passes(cx: &mut TestAppContext, window: &OpenWindow) {
    let interval = Duration::from_secs_f32(1.0 / ANIMATION_FPS) + Duration::from_millis(1);
    for _ in 0..interval.as_millis().div_ceil(REFRESH.as_millis()) {
        display_frame(cx, window);
    }
}

/// The status bar's region notified, as the tick does when what the bar says is not what it drew.
fn notify_bar(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.update(|cx| {
        window.workspace.update(cx, |ws, cx| ws.regions.notify_status_bar(cx)).ok();
    });
}

fn rolling(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    read_cx(cx, window, |ws, _| ws.regions.drawn.rolls.is_rolling())
}

/// Two terminals' hooks: the count of the working ones goes from none to one to two.
fn count_up(cx: &mut TestAppContext, window: &OpenWindow) {
    hook(cx, window, 2, "UserPromptSubmit", None);
    draw(cx, window);
    draw(cx, window);
    assert!(!rolling(cx, window), "an item that comes is not a count turning over");
    hook(cx, window, 3, "UserPromptSubmit", None);
    draw(cx, window);
    draw(cx, window);
}

#[gpui_kit::test]
fn a_count_in_the_status_bar_rolls_when_it_changes_and_the_bar_holds_nothing_at_rest(
    cx: &mut TestAppContext,
) {
    if isolate("a_count_in_the_status_bar_rolls_when_it_changes_and_the_bar_holds_nothing_at_rest") {
        return;
    }
    let window = with_bar(cx);
    assert!(!rolling(cx, &window), "a bar at rest holds no roll");
    count_up(cx, &window);
    assert!(rolling(cx, &window), "1 to 2 working turns over");
    // Some time on (the roll's clock is the executor's, which a test moves): the next build of the bar drops it.
    cx.executor().advance_clock(Duration::from_millis(250));
    notify_bar(cx, &window);
    draw(cx, &window);
    assert!(!rolling(cx, &window), "and it is over");
}

#[gpui_kit::test]
fn a_count_rolling_draws_the_status_bar_and_no_other_region(cx: &mut TestAppContext) {
    if isolate("a_count_rolling_draws_the_status_bar_and_no_other_region") {
        return;
    }
    let window = with_bar(cx);
    count_up(cx, &window);
    assert!(rolling(cx, &window));
    let builds = |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| ws.regions.builds(cx));
    let before = builds(cx);
    // The roll's animation asks for its next frame while it is drawn, which notifies the region it is
    // in. It runs on the wall clock (160 ms), so a slow machine may end it before the test has looked: each
    // frame it asked for builds the bar, a frame it did not ask for builds nothing, and no other region is
    // ever built. One display frame at a time, as a frame can hold one of the bar's builds at most.
    let mut asked = 0;
    for _ in 0..16 {
        let before_frame = builds(cx);
        display_frame(cx, &window);
        draw_replaying(cx, &window);
        let now = builds(cx);
        assert_eq!(
            now[..3],
            before_frame[..3],
            "the sidebar's header, the panel and the title row were not built"
        );
        assert!(now[3] - before_frame[3] <= 1, "the bar is built once for a frame");
        asked += now[3] - before_frame[3];
    }
    assert!(asked > 0, "the roll asked for frames");
    assert_eq!(builds(cx)[3], before[3] + asked);
}

#[gpui_kit::test]
fn counts_do_not_roll_under_reduce_motion_or_without_the_motion_feature(cx: &mut TestAppContext) {
    if isolate("counts_do_not_roll_under_reduce_motion_or_without_the_motion_feature") {
        return;
    }
    let window = with_bar(cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    count_up(cx, &window);
    assert!(!rolling(cx, &window), "Reduce Motion cuts the count");
    cx.update(|cx| {
        cx.set_reduce_motion(false);
        crate::settings::update_settings(cx, |settings| {
            settings.feature_flags.insert(FeatureFlag::Motion.key().to_string(), false);
        });
    });
    hook(cx, &window, 2, "Stop", None);
    draw(cx, &window);
    draw(cx, &window);
    assert!(!rolling(cx, &window), "nor does a build without the Motion feature");
}
