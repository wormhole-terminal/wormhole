//! The intro window in a real window (GPUI's test platform; `view_tests` says how each test runs in
//! a child process with scratch data): its pages walked with the keys, Esc, ⇥ staying among its
//! buttons, a replay coming back to the welcome page, and the intro opened again later.

use std::time::Duration;

use gpui_kit::TestAppContext;
use wormhole_core::onboarding::Step;

use wormhole_core::features::FeatureFlag;

use super::FADE_OUT;
use crate::workspace::app::OpenWindow;
use crate::workspace::settings_page::update_settings;
use crate::workspace::view_tests::{draw, drive, isolate_in, press, read, start};

/// Press a key as the keyboard would, after a frame (the keys go where the last frame's focus is).
fn key(cx: &mut TestAppContext, window: &OpenWindow, keys: &str) {
    draw(cx, window);
    press(cx, window, keys);
}

fn step(cx: &mut TestAppContext, window: &OpenWindow) -> Option<Step> {
    read(cx, window, |ws| ws.first_run.intro.as_ref().map(|intro| intro.model.step()))
}

/// Whether the keyboard is on the page or one of its buttons.
fn in_page(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    draw(cx, window);
    cx.update(|cx| {
        window.handle.update(cx, |_, win, cx| {
            let workspace = window.workspace.upgrade()?;
            let workspace = workspace.read(cx);
            Some(workspace.first_run.intro.as_ref()?.focus.contains_focused(win, cx))
        })
    })
    .ok()
    .flatten()
    .unwrap_or(false)
}

#[gpui_kit::test]
fn the_arrows_and_return_walk_the_pages_and_esc_leaves(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_arrows_and_return_walk_the_pages_and_esc_leaves") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Welcome, window, cx));
    assert_eq!(step(cx, &window), Some(Step::Welcome));
    assert!(read(cx, &window, |ws| ws.first_run.is_open()));

    key(cx, &window, "right");
    assert_eq!(step(cx, &window), Some(Step::Folder), "→: the next page");
    key(cx, &window, "left");
    assert_eq!(step(cx, &window), Some(Step::Welcome), "←: back");
    key(cx, &window, "left");
    assert_eq!(step(cx, &window), Some(Step::Welcome), "← on the first page stays");
    key(cx, &window, "enter");
    assert_eq!(step(cx, &window), Some(Step::Folder), "⏎: the main button, Get started");

    // The window's own shortcuts wait while the intro is up: ⌘T opens no terminal under it.
    key(cx, &window, "cmd-t");
    read(cx, &window, |ws| assert_eq!(ws.sessions.len(), 1, "⌘T waits"));

    key(cx, &window, "escape");
    assert_eq!(step(cx, &window), None, "Esc skips the intro");
    read(cx, &window, |ws| assert!(!ws.first_run.is_open()));
}

#[gpui_kit::test]
fn tab_stays_among_the_pages_buttons(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "tab_stays_among_the_pages_buttons") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Welcome, window, cx));
    // Past the last of its buttons, in both directions, and the keys are still on the page: the
    // sidebar under it is out of reach.
    for _ in 0..8 {
        key(cx, &window, "tab");
        assert!(in_page(cx, &window), "⇥ left the page");
    }
    for _ in 0..8 {
        key(cx, &window, "shift-tab");
        assert!(in_page(cx, &window), "⇧⇥ left the page");
    }
}

#[gpui_kit::test]
fn a_replay_comes_back_to_the_welcome_page(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_replay_comes_back_to_the_welcome_page") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Look, window, cx));
    assert_eq!(step(cx, &window), Some(Step::Look));

    key(cx, &window, "r");
    // The window leaves first; the page changes once it is out of sight.
    assert_eq!(step(cx, &window), Some(Step::Look), "still the old page while the window leaves");
    cx.executor().advance_clock(FADE_OUT + Duration::from_millis(50));
    cx.run_until_parked();
    assert_eq!(step(cx, &window), Some(Step::Welcome), "R: back to the welcome page");
    assert!(in_page(cx, &window), "with the keyboard on it");
}

#[gpui_kit::test]
fn the_intro_can_be_shown_again_as_the_welcome_page_alone(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_intro_can_be_shown_again_as_the_welcome_page_alone") {
        return;
    }
    let window = start(cx, 1);
    // With the terminal welcome off (it is on by default in a development build), Help → Welcome
    // to Wormhole is the intro window, as ever.
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.feature_flags.insert(FeatureFlag::TerminalWelcome.key().to_string(), false);
        })
    });
    drive(cx, &window, |ws, window, cx| ws.show_intro(window, cx));
    // A window that is showing leaves first, as for a replay.
    cx.executor().advance_clock(FADE_OUT + Duration::from_millis(50));
    cx.run_until_parked();
    assert_eq!(step(cx, &window), Some(Step::Welcome));
    read(cx, &window, |ws| {
        let intro = ws.first_run.intro.as_ref().expect("open");
        assert!(!intro.first_run, "a replay, not the first run");
        assert!(intro.model.is_last(), "its one button ends it");
    });
    // → does not end it, ⏎ does; and afterwards the terminal has the keyboard.
    key(cx, &window, "right");
    assert_eq!(step(cx, &window), Some(Step::Welcome));
    key(cx, &window, "enter");
    assert_eq!(step(cx, &window), None);
    read(cx, &window, |ws| assert!(ws.first_run.agents_note == super::AgentsNote::Off));
}
