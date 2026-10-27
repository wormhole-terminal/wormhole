//! The terminal welcome's walk through how much the window shows, in a real window: `SetShown` changes
//! the settings it names, opens or closes that window's right panel, and nothing it was not asked.

use gpui_kit::TestAppContext;
use wormhole_core::config::feature_table::FeatureId;
use wormhole_core::config::presets::{Preset, Shown, TitleShown, ToolsShown};
use wormhole_core::ipc::control::{ErrorCode, ShownDone, WelcomeOp};
use wormhole_core::onboarding::intro::Handover;
use wormhole_core::view::layout::title_parts::{TitlePart, TitleRow};

use super::welcome_tests::{ask, code, state, state_of, welcome_terminal};
use crate::settings::{settings, update_settings};
use crate::window::test_harness::{draw, drive, isolate_in, read, start};

/// Each test runs in a child process of its own; the harness finds it by this module's path.
fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn set(from: Shown, shown: Shown) -> WelcomeOp {
    WelcomeOp::SetShown { from, shown }
}

/// What the window shows now: the settings and its panel.
fn shown_now(cx: &mut TestAppContext, window: &crate::app_state::OpenWindow) -> Shown {
    let panel = read(cx, window, |ws| ws.welcome_panel_open());
    cx.update(|cx| Shown::read(settings(cx), panel))
}

#[gpui_kit::test]
fn the_walks_answers_change_the_window_around_the_welcome_terminal(cx: &mut TestAppContext) {
    if isolate("the_walks_answers_change_the_window_around_the_welcome_terminal") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);

    // The state says what the window shows: the defaults, with the panel as it is.
    let first = state_of(&ask(cx, Some(tab), &token, state())).shown.expect("what the window shows");
    assert_eq!(first, Shown::read(&wormhole_core::config::Settings::default(), first.panel_open));

    // Not for anyone else, and nothing changes for a request that is turned away.
    for (from, token) in [(None, token.as_str()), (Some(1), token.as_str()), (Some(tab), "wrong")] {
        let reply = ask(cx, from, token, set(first.clone(), Shown::of(Preset::Everything)));
        assert_eq!(code(&reply), Some(ErrorCode::Refused), "{from:?}");
    }
    assert_eq!(shown_now(cx, &window), first);

    // Everything: the status bar, the minimap and the lines come on, and the panel opens.
    let reply = ask(cx, Some(tab), &token, set(first.clone(), Shown::of(Preset::Everything)));
    assert!(reply.ok, "{reply:?}");
    let done: ShownDone =
        serde_json::from_value(reply.result.expect("a result")).expect("where the panel is");
    assert!(done.panel_open, "and it says the panel is open");
    cx.run_until_parked();
    draw(cx, &window);
    let everything = shown_now(cx, &window);
    assert_eq!(everything.preset(), Some(Preset::Everything), "{everything:?}");
    assert!(read(cx, &window, |ws| ws.welcome_panel_open()), "the panel of that window is open");
    assert_eq!(state_of(&ask(cx, Some(tab), &token, state())).shown, Some(everything));

    // Zen: all of it goes, and the panel closes.
    let reply = ask(cx, Some(tab), &token, set(Shown::of(Preset::Everything), Shown::of(Preset::Zen)));
    assert!(reply.ok, "{reply:?}");
    cx.run_until_parked();
    draw(cx, &window);
    let (status_bar, minimap, dividers, parts) = cx.update(|cx| {
        let s = settings(cx);
        (s.status_bar, s.minimap, s.column_dividers, s.title_row().parts().to_vec())
    });
    assert!(!status_bar && !minimap && !dividers);
    assert_eq!(parts, [TitlePart::Terminal, TitlePart::Space, TitlePart::Splits]);
    assert!(!read(cx, &window, |ws| ws.welcome_panel_open()), "and the panel is closed");
    assert_eq!(shown_now(cx, &window).preset(), Some(Preset::Zen));
}

/// Only what the answer changed is written: a title row or a set of tools of the person's own, which the
/// walk carries along as it found it, is not overwritten by an answer about something else, and neither
/// is anything the person changed by hand while the walk was up.
#[gpui_kit::test]
fn an_answer_writes_only_what_it_changes(cx: &mut TestAppContext) {
    if isolate("an_answer_writes_only_what_it_changes") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);
    let mine = TitleRow::new([TitlePart::Splits, TitlePart::Space, TitlePart::Folder]);
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.title_row = Some(mine.clone());
            s.switch_feature(FeatureId::History, false);
        });
    });
    let arranged = shown_now(cx, &window);
    assert_eq!(arranged.title, TitleShown::Own(mine.clone()));
    assert!(matches!(arranged.tools, ToolsShown::Own(_)));

    // The walk read this, answers the minimap and sends the whole thing, its own arrangement as it was.
    let reply =
        ask(cx, Some(tab), &token, set(arranged.clone(), Shown { minimap: true, ..arranged.clone() }));
    assert!(reply.ok, "{reply:?}");
    let after = shown_now(cx, &window);
    assert!(after.minimap);
    assert_eq!(
        Shown { minimap: false, ..after.clone() },
        arranged,
        "the title row and the tools are as they were"
    );
    assert_eq!(cx.update(|cx| settings(cx).title_row().into_owned()), mine);

    // The person turns the status bar on by hand and closes the panel; the walk, which knew neither,
    // answers the dividers. Only the dividers change.
    cx.update(|cx| update_settings(cx, |s| s.status_bar = true));
    drive(cx, &window, |ws, _, cx| ws.set_welcome_panel(true, cx));
    drive(cx, &window, |ws, _, cx| ws.set_welcome_panel(false, cx));
    let by_hand = shown_now(cx, &window);
    assert!(by_hand.status_bar && !by_hand.panel_open);
    let walk_knew = Shown { status_bar: false, panel_open: true, ..after.clone() };
    let reply =
        ask(cx, Some(tab), &token, set(walk_knew.clone(), Shown { column_dividers: false, ..walk_knew }));
    assert!(reply.ok, "{reply:?}");
    let now = shown_now(cx, &window);
    assert_eq!(now, Shown { column_dividers: false, ..by_hand }, "{now:?}");

    // An answer that changes nothing writes nothing: the file is not written again.
    let file = crate::active_profile::paths().config_file();
    let written =
        || std::fs::metadata(&file).and_then(|meta| meta.modified()).expect("config.json was written");
    let before = written();
    let same = shown_now(cx, &window);
    assert!(ask(cx, Some(tab), &token, set(same.clone(), same)).ok);
    assert_eq!(written(), before);
}

#[gpui_kit::test]
fn a_walk_asking_for_more_than_there_is_is_refused_and_changes_nothing(cx: &mut TestAppContext) {
    if isolate("a_walk_asking_for_more_than_there_is_is_refused_and_changes_nothing") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);
    let before = shown_now(cx, &window);
    let flipped = Shown { minimap: !before.minimap, ..before.clone() };
    // A tool named twice.
    let twice =
        Shown { tools: ToolsShown::Own(vec![FeatureId::History, FeatureId::History]), ..flipped.clone() };
    let reply = ask(cx, Some(tab), &token, set(before.clone(), twice));
    assert_eq!(code(&reply), Some(ErrorCode::BadRequest));
    // An arrangement of one's own that the walk did not start with, which no answer offers to make.
    let tools = Shown { tools: ToolsShown::Own(vec![FeatureId::History]), ..flipped.clone() };
    let reply = ask(cx, Some(tab), &token, set(before.clone(), tools));
    assert_eq!(code(&reply), Some(ErrorCode::BadRequest), "tools");
    let row = TitleRow::new([TitlePart::Splits, TitlePart::Space]);
    let title = Shown { title: TitleShown::Own(row), ..flipped };
    let reply = ask(cx, Some(tab), &token, set(before.clone(), title));
    assert_eq!(code(&reply), Some(ErrorCode::BadRequest), "title row");
    assert_eq!(shown_now(cx, &window), before);
}
