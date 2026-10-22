//! View tests of restarting to update: the question before it, and "Restart When Idle" settled by
//! the tick (see `view_tests` for how each test runs in a child process with scratch data). A test
//! process is not an app bundle, so a restart that goes ahead only clears the request.
use std::time::{Duration, Instant};

use gpui_kit::TestAppContext;
use wormhole_core::storage::store::{SessionRecord, WindowRecord};

use crate::workspace::app::{OpenWindow, app, app_mut, open_workspace_window};
use crate::workspace::update_restart::{request_restart, tick_at};
use crate::workspace::updates::UpdatePhase;
use crate::workspace::view_tests::{hook, isolate_in, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// An update is installed, as Settings → Updates shows the Restart button.
fn install_update(cx: &mut TestAppContext) {
    cx.update(|cx| app_mut(cx).updates.phase = UpdatePhase::Installed("0.3.1".into()));
}

/// Restart, as the button in Settings does, with the sheet on `window`.
fn press_restart(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.update(|cx| window.handle.update(cx, |_, window, cx| request_restart(window, cx)))
        .expect("the window is open");
    cx.run_until_parked();
}

fn pending(cx: &mut TestAppContext) -> bool {
    cx.update(|cx| app(cx).updates.restart_pending())
}

/// Let the tick look at `at`, `secs` seconds after `start`.
fn look(cx: &mut TestAppContext, start: Instant, secs: u64) {
    cx.update(|cx| tick_at(start + Duration::from_secs(secs), cx));
}

#[gpui_kit::test]
fn restart_asks_only_while_something_runs_and_says_what(cx: &mut TestAppContext) {
    if isolate("restart_asks_only_while_something_runs_and_says_what") {
        return;
    }
    let window = start(cx, 2);
    install_update(cx);
    press_restart(cx, &window);
    assert!(!cx.has_pending_prompt(), "nothing runs: it restarts without asking");

    hook(cx, &window, 1, "UserPromptSubmit", None);
    press_restart(cx, &window);
    let (title, detail) = cx.pending_prompt().expect("an agent at work: a question first");
    assert_eq!(title, "Restart to Update?");
    // The terminal is named after its agent, so the line says it once.
    assert_eq!(
        detail,
        "Claude is working\n\nYour terminals come back after the restart, but what runs in them ends."
    );
    // Later does nothing beyond closing the sheet.
    cx.simulate_prompt_answer("Later");
    cx.run_until_parked();
    assert!(!pending(cx));
    assert!(!cx.has_pending_prompt());
}

#[gpui_kit::test]
fn a_terminal_in_another_window_counts(cx: &mut TestAppContext) {
    if isolate("a_terminal_in_another_window_counts") {
        return;
    }
    let first = start(cx, 1);
    cx.update(|cx| {
        let record =
            WindowRecord { id: 2, sessions: vec![SessionRecord::new(9, None, None)], ..Default::default() };
        open_workspace_window(Some(record), None, cx);
    });
    cx.run_until_parked();
    let second = cx.update(|cx| app(cx).windows.iter().find(|w| w.id == 2).cloned()).expect("the second");
    install_update(cx);
    hook(cx, &second, 9, "UserPromptSubmit", None);
    // Asked from the first window's sheet, it still names the terminal running in the second.
    press_restart(cx, &first);
    assert!(cx.pending_prompt().is_some_and(|(_, detail)| detail.contains("is working")));
    cx.simulate_prompt_answer("Later");
}

#[gpui_kit::test]
fn restart_when_idle_waits_for_a_quiet_stretch(cx: &mut TestAppContext) {
    if isolate("restart_when_idle_waits_for_a_quiet_stretch") {
        return;
    }
    let window = start(cx, 1);
    install_update(cx);
    hook(cx, &window, 1, "UserPromptSubmit", None);
    press_restart(cx, &window);
    cx.simulate_prompt_answer("Restart When Idle");
    cx.run_until_parked();
    assert!(pending(cx), "remembered");

    let t0 = Instant::now();
    look(cx, t0, 0);
    look(cx, t0, 3600);
    assert!(pending(cx), "an agent at work holds it for as long as it works");

    // Done: the wait starts, and is not over before 30 seconds.
    hook(cx, &window, 1, "Stop", None);
    look(cx, t0, 3601);
    look(cx, t0, 3620);
    assert!(pending(cx));
    // Between two steps: busy for a moment, and the wait starts over.
    hook(cx, &window, 1, "UserPromptSubmit", None);
    look(cx, t0, 3625);
    hook(cx, &window, 1, "Stop", None);
    look(cx, t0, 3626);
    look(cx, t0, 3655);
    assert!(pending(cx), "29 seconds since the last busy look");
    // 30 seconds idle in a row: it restarts (here: the request is answered).
    look(cx, t0, 3656);
    assert!(!pending(cx));
}

#[gpui_kit::test]
fn a_request_does_not_outlive_the_installed_update(cx: &mut TestAppContext) {
    if isolate("a_request_does_not_outlive_the_installed_update") {
        return;
    }
    let window = start(cx, 1);
    install_update(cx);
    hook(cx, &window, 1, "UserPromptSubmit", None);
    press_restart(cx, &window);
    cx.simulate_prompt_answer("Restart When Idle");
    cx.run_until_parked();
    assert!(pending(cx));
    // A new check changed what Settings shows: the line is gone, so the request must be too.
    cx.update(|cx| app_mut(cx).updates.phase = UpdatePhase::Checking);
    look(cx, Instant::now(), 0);
    assert!(!pending(cx));
}
