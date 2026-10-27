//! Windows and splits asked for from the control line, in real windows whose terminals run `/bin/cat`:
//! `new-window`, `split`, `focus`, `rename` and `close` with the ids they answer, the events they cause, what
//! they leave alone (the selection, the front window), and that `close` only ever asks. The harness is
//! `test_harness`.

use std::sync::Arc;
use std::time::Duration;

use gpui_kit::{Bounds, TestAppContext, point, px, size};
use serde::de::DeserializeOwned;
use wormhole_core::ipc::bus::{Bus, Grant, Next, Published, Subscription};
use wormhole_core::ipc::control::event::{Body, EventsSpec};
use wormhole_core::ipc::control::structure::NAME_MAX_CHARS;
use wormhole_core::ipc::control::{
    Action, CloseAsked, CloseSpec, ErrorCode, FocusSpec, Focused, Listing, NewWindowSpec, Reason, RenameSpec,
    Renamed, Reply, Request, Side, SplitMade, SplitSpec, Target, WindowOpened,
};
use wormhole_core::ipc::{Answer, Peer};

use crate::app::control;
use crate::app_state::{AppState, OpenWindow, app};
use crate::settings::update_settings;
use crate::window::test_harness::{draw, hook, isolate_in, read, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn ask(cx: &mut TestAppContext, action: Action, from: Option<&str>) -> Reply {
    ask_in(cx, None, action, from)
}

/// [`ask`] with the window's panes area as a frame would have measured it, set in the same breath: any
/// frame between would measure the test window's own.
fn ask_in(
    cx: &mut TestAppContext,
    area: Option<(&OpenWindow, f32, f32)>,
    action: Action,
    from: Option<&str>,
) -> Reply {
    let peer = Peer::new(900_002, vec![1]);
    let request = Request::new(action, from.map(str::to_string));
    let answer = cx.update(|cx| {
        if let Some((window, width, height)) = area {
            AppState::with_workspace(window, cx, |ws, _, _| {
                ws.pane_area = Bounds::new(point(px(0.0), px(0.0)), size(px(width), px(height)));
            });
        }
        control::answer(request, &peer, cx)
    });
    match answer {
        Answer::Reply(reply) => reply,
        _ => panic!("these verbs answer with one line"),
    }
}

/// The result of a reply that worked, read as `T`.
fn result<T: DeserializeOwned>(reply: &Reply) -> T {
    let value = reply.result.clone().unwrap_or_else(|| panic!("{reply:?}"));
    serde_json::from_value(value).expect("the result of this verb")
}

fn code(reply: &Reply) -> Option<ErrorCode> {
    reply.err().map(|error| error.code)
}

fn reason(reply: &Reply) -> Option<Reason> {
    reply.err().and_then(|error| error.reason)
}

fn allow(cx: &mut TestAppContext, write: bool) {
    cx.update(|cx| update_settings(cx, |settings| settings.control_write = write));
}

fn split_of(target: i64, side: Side) -> SplitSpec {
    SplitSpec {
        target: Target::Terminal(target),
        direction: side,
        size: None,
        folder: None,
        command: None,
        select: false,
    }
}

/// A subscription to the `topics`, for a caller that may read nothing.
fn watch(cx: &mut TestAppContext, topics: &[&str]) -> Subscription {
    let bus = cx.update(|cx| match crate::automation::bus(cx) {
        Some(bus) => bus,
        None => {
            let bus = Bus::new();
            crate::automation::install(bus.clone(), cx);
            bus
        }
    });
    let spec = EventsSpec { topics: topics.iter().map(|t| t.to_string()).collect(), ..EventsSpec::default() };
    let filter = spec.filter().expect("valid topics");
    bus.subscribe(filter, Grant { reads_all: false, own: vec![] }, None).expect("room").0
}

fn told(subscription: &Subscription) -> Vec<Arc<Published>> {
    let mut out = Vec::new();
    while let Next::Event(published) = subscription.next(Duration::ZERO) {
        out.push(published);
    }
    out
}

/// The panes of the split terminal `id` is in, in reading order, and the one with the keyboard.
fn panes(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> Option<(Vec<i64>, i64)> {
    read(cx, window, |ws| {
        let ix = ws.find_session(id)?;
        ws.split_of(ix).map(|split| (split.layout.panes(), split.layout.focused()))
    })
}

fn selected(cx: &mut TestAppContext, window: &OpenWindow) -> i64 {
    read(cx, window, |ws| ws.sessions[ws.active].id)
}

#[gpui_kit::test]
fn every_verb_needs_the_flag_and_the_switch(cx: &mut TestAppContext) {
    if isolate("every_verb_needs_the_flag_and_the_switch") {
        return;
    }
    let _window = start(cx, 1);
    let target = Target::Terminal(1);
    let actions = || {
        [
            Action::NewWindow(NewWindowSpec::default()),
            Action::Split(split_of(1, Side::Right)),
            Action::Focus(FocusSpec { target, activate: false }),
            Action::Rename(RenameSpec { target, name: Some("x".into()) }),
            Action::Close(CloseSpec { target }),
        ]
    };
    allow(cx, false);
    for action in actions() {
        let name = action.name();
        assert_eq!(code(&ask(cx, action, None)), Some(ErrorCode::NotAllowed), "{name}");
    }
    allow(cx, true);
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.feature_flags.insert("automation".into(), false);
        })
    });
    for action in actions() {
        let name = action.name();
        let reply = ask(cx, action, None);
        assert_eq!(
            (code(&reply), reason(&reply)),
            (Some(ErrorCode::Unsupported), Some(Reason::FlagOff)),
            "{name}"
        );
    }
}

#[gpui_kit::test]
fn a_split_goes_to_the_side_asked_and_leaves_the_selection_and_the_keyboard_alone(cx: &mut TestAppContext) {
    if isolate("a_split_goes_to_the_side_asked_and_leaves_the_selection_and_the_keyboard_alone") {
        return;
    }
    let window = start(cx, 2);
    allow(cx, true);
    let watcher = watch(cx, &["splits", "terminals"]);
    draw(cx, &window);
    let big = Some((&window, 4000.0, 2400.0));

    // Terminal 2 is not the selected one; the new terminal goes to its right, in a split of its own.
    let reply = ask_in(cx, big, Action::Split(split_of(2, Side::Right)), None);
    let made: SplitMade = result(&reply);
    assert_eq!(made.window, 1);
    assert_eq!(panes(cx, &window, 2), Some((vec![2, made.terminal], 2)), "the keyboard stays in pane 2");
    assert_eq!(selected(cx, &window), 1, "the selection is where it was");
    let events = told(&watcher);
    let names: Vec<_> = events.iter().map(|p| p.event.body.name()).collect();
    assert_eq!(names, ["terminal_opened", "split"], "{events:?}");
    assert_eq!(events[0].event.terminal, Some(made.terminal));
    let Body::Split { split, terminals, focused } = &events[1].event.body else { panic!("a split") };
    assert_eq!((*split, terminals.clone(), *focused), (made.split, vec![2, made.terminal], Some(2)));

    // Down, left and up, each of the new split's panes by the one named.
    let down: SplitMade = result(&ask_in(cx, big, Action::Split(split_of(1, Side::Down)), None));
    assert_eq!(panes(cx, &window, 1), Some((vec![1, down.terminal], 1)));
    assert_ne!(down.split, made.split);
    let left: SplitMade = result(&ask_in(cx, big, Action::Split(split_of(2, Side::Left)), None));
    assert_eq!(left.split, made.split, "the target's own split");
    assert_eq!(panes(cx, &window, 2).map(|(panes, _)| panes), Some(vec![left.terminal, 2, made.terminal]));
    let up: SplitMade = result(&ask_in(cx, big, Action::Split(split_of(1, Side::Up)), None));
    assert_eq!(panes(cx, &window, 1).map(|(panes, _)| panes), Some(vec![up.terminal, 1, down.terminal]));
    assert_eq!(selected(cx, &window), 1);

    // The listing tells the splits and who is in which.
    let listing: Listing = result(&ask(cx, Action::List, None));
    let info = &listing.windows[0];
    assert_eq!(info.splits.len(), 2);
    let ours = info.splits.iter().find(|s| s.id == made.split).expect("listed");
    assert_eq!((ours.terminals.clone(), ours.focused), (vec![left.terminal, 2, made.terminal], 2));
    let row = info.terminals.iter().find(|t| t.id == made.terminal).expect("listed");
    assert_eq!(row.split, Some(made.split));

    // With --select the new terminal is the selected one, with the keyboard.
    let spec = SplitSpec { select: true, ..split_of(2, Side::Right) };
    let chosen: SplitMade = result(&ask_in(cx, big, Action::Split(spec), None));
    assert_eq!(selected(cx, &window), chosen.terminal);
    assert_eq!(panes(cx, &window, 2).map(|(_, focused)| focused), Some(chosen.terminal));
}

#[gpui_kit::test]
fn a_split_takes_its_size_its_folder_and_has_no_room_when_the_panes_would_be_too_small(
    cx: &mut TestAppContext,
) {
    if isolate("a_split_takes_its_size_its_folder_and_has_no_room_when_the_panes_would_be_too_small") {
        return;
    }
    let window = start(cx, 1);
    allow(cx, true);
    let wide = Some((&window, 1000.0, 600.0));
    let folder = std::env::temp_dir();
    let spec = SplitSpec {
        size: Some(30),
        folder: Some(folder.to_string_lossy().into_owned()),
        command: Some("echo hi".into()),
        ..split_of(1, Side::Right)
    };
    let made: SplitMade = result(&ask_in(cx, wide, Action::Split(spec), None));
    assert_eq!(made.folder.as_deref(), folder.to_str());
    let share = read(cx, &window, |ws| {
        let ix = ws.find_session(made.terminal).expect("there");
        let rects = ws.split_of(ix).expect("split").layout.tree().rects(1000.0, 600.0);
        rects.iter().find(|(id, _)| *id == made.terminal).map(|(_, rect)| rect.w).expect("a pane")
    });
    assert!((share - 300.0).abs() < 1.0, "30 percent of 1000 points: {share}");

    // A pane would be under its minimum width: refused with nothing made.
    let before = read(cx, &window, |ws| ws.sessions.len());
    let narrow = Some((&window, 300.0, 600.0));
    let refused = ask_in(cx, narrow, Action::Split(split_of(1, Side::Right)), None);
    assert_eq!((code(&refused), reason(&refused)), (Some(ErrorCode::Refused), Some(Reason::NoRoom)));
    assert_eq!(read(cx, &window, |ws| ws.sessions.len()), before, "nothing changed");

    // A mistake in the request is named, and nothing opens.
    let bad = SplitSpec { size: Some(100), ..split_of(1, Side::Down) };
    assert_eq!(code(&ask(cx, Action::Split(bad), None)), Some(ErrorCode::BadRequest));
    let bad = SplitSpec { folder: Some("relative".into()), ..split_of(1, Side::Down) };
    assert_eq!(code(&ask(cx, Action::Split(bad), None)), Some(ErrorCode::NotFound));
    let bad = SplitSpec { command: Some("a\nb".into()), ..split_of(1, Side::Down) };
    assert_eq!(code(&ask(cx, Action::Split(bad), None)), Some(ErrorCode::BadRequest));
    assert_eq!(code(&ask(cx, Action::Split(split_of(9, Side::Down)), None)), Some(ErrorCode::NotFound));
    assert_eq!(read(cx, &window, |ws| ws.sessions.len()), before);
}

#[gpui_kit::test]
fn focus_selects_the_terminal_and_a_name_is_set_cleaned_and_cleared(cx: &mut TestAppContext) {
    if isolate("focus_selects_the_terminal_and_a_name_is_set_cleaned_and_cleared") {
        return;
    }
    let window = start(cx, 2);
    allow(cx, true);
    let watcher = watch(cx, &["terminals"]);
    let focused: Focused =
        result(&ask(cx, Action::Focus(FocusSpec { target: Target::Terminal(2), activate: false }), None));
    assert_eq!((focused.window, focused.terminal), (1, 2));
    assert_eq!(selected(cx, &window), 2);
    let missing = ask(cx, Action::Focus(FocusSpec { target: Target::Terminal(9), activate: false }), None);
    assert_eq!(code(&missing), Some(ErrorCode::NotFound));

    // The caller's own terminal may be named: nothing is typed into it.
    let rename = |name: Option<&str>| {
        Action::Rename(RenameSpec { target: Target::Terminal(2), name: name.map(str::to_string) })
    };
    let named: Renamed = result(&ask(cx, rename(Some("  dev \u{202e} server\n")), Some("2")));
    assert_eq!((named.terminal, named.name.as_str(), named.custom), (2, "dev server", true));
    let events = told(&watcher);
    let events: Vec<_> = events.iter().filter(|p| p.event.body.name() == "renamed").collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.body, Body::Renamed { name: Some("dev server".into()) });
    assert_eq!(events[0].event.terminal, Some(2));

    let long = "n".repeat(100);
    let cut: Renamed = result(&ask(cx, rename(Some(&long)), None));
    assert_eq!(cut.name.chars().count(), NAME_MAX_CHARS + 1, "cut with an ellipsis");
    assert_eq!(code(&ask(cx, rename(Some("\u{1b}\u{200b}")), None)), Some(ErrorCode::BadRequest));
    assert_eq!(read(cx, &window, |ws| ws.sessions[1].custom_name.clone()), Some(cut.name.clone()));

    let cleared: Renamed = result(&ask(cx, rename(None), None));
    assert!(!cleared.custom);
    assert_eq!(read(cx, &window, |ws| ws.sessions[1].custom_name.clone()), None);
    assert_eq!(
        code(&ask(cx, Action::Rename(RenameSpec { target: Target::Terminal(9), name: None }), None)),
        Some(ErrorCode::NotFound)
    );
}

#[gpui_kit::test]
fn close_only_asks_and_a_second_question_waits_for_the_first(cx: &mut TestAppContext) {
    if isolate("close_only_asks_and_a_second_question_waits_for_the_first") {
        return;
    }
    let window = start(cx, 2);
    allow(cx, true);
    let watcher = watch(cx, &["terminals"]);
    let close = |id| Action::Close(CloseSpec { target: Target::Terminal(id) });

    // Nothing is running in terminal 1: a person's close would be at once. A script's asks anyway.
    let asked: CloseAsked = result(&ask(cx, close(1), None));
    assert_eq!((asked.asked, asked.window, asked.terminal), (true, 1, 1));
    assert!(cx.has_pending_prompt(), "a sheet on the window");
    assert_eq!(read(cx, &window, |ws| ws.sessions.len()), 2, "nothing is closed");
    let again = ask(cx, close(2), None);
    assert_eq!((code(&again), reason(&again)), (Some(ErrorCode::Refused), Some(Reason::QuestionOpen)));

    // Cancel: it stays, the window is free to ask again, and the watcher heard nothing close.
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    assert_eq!(read(cx, &window, |ws| ws.sessions.len()), 2);
    assert!(told(&watcher).iter().all(|p| p.event.body.name() != "terminal_closed"));

    // Close: the person said yes, and then the event follows. A terminal's own caller may ask about itself.
    let own: CloseAsked = result(&ask(cx, close(1), Some("1")));
    assert_eq!(own.terminal, 1);
    cx.simulate_prompt_answer("Close");
    cx.run_until_parked();
    assert_eq!(read(cx, &window, |ws| ws.sessions.iter().map(|s| s.id).collect::<Vec<_>>()), [2]);
    let closed = told(&watcher);
    let closed: Vec<_> = closed.iter().filter(|p| p.event.body.name() == "terminal_closed").collect();
    assert_eq!(closed.len(), 1);
    assert_eq!((closed[0].event.window, closed[0].event.terminal), (Some(1), Some(1)));

    // A busy terminal is asked about with what runs in it; the sheet is there all the same.
    hook(cx, &window, 2, "UserPromptSubmit", None);
    assert!(result::<CloseAsked>(&ask(cx, close(2), None)).asked);
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    assert_eq!(code(&ask(cx, close(9), None)), Some(ErrorCode::NotFound));
}

#[gpui_kit::test]
fn a_new_window_opens_behind_unless_asked_and_answers_the_ids(cx: &mut TestAppContext) {
    if isolate("a_new_window_opens_behind_unless_asked_and_answers_the_ids") {
        return;
    }
    let _window = start(cx, 1);
    allow(cx, true);
    let watcher = watch(cx, &["windows", "terminals"]);
    let front_before = cx.update(|cx| app(cx).front);

    let folder = std::env::temp_dir();
    let spec = NewWindowSpec {
        folder: Some(folder.to_string_lossy().into_owned()),
        command: Some("echo hi".into()),
        activate: false,
    };
    let opened: WindowOpened = result(&ask(cx, Action::NewWindow(spec), None));
    cx.run_until_parked();
    assert_ne!(opened.window, 1);
    assert_eq!(opened.folder.as_deref(), folder.to_str());
    cx.update(|cx| {
        let state = app(cx);
        let listed = state.windows.iter().find(|w| w.id == opened.window).expect("listed");
        let terminals =
            listed.workspace.read_with(cx, |ws, _| ws.sessions.iter().map(|s| s.id).collect::<Vec<_>>());
        assert_eq!(terminals.ok(), Some(vec![opened.terminal]));
        assert_eq!(state.front, front_before, "a window that opened behind is not the one in front");
    });
    let events = told(&watcher);
    let names: Vec<_> =
        events.iter().map(|p| (p.event.body.name(), p.event.window, p.event.terminal)).collect();
    assert_eq!(
        names,
        [
            ("window_opened", Some(opened.window), None),
            ("terminal_opened", Some(opened.window), Some(opened.terminal))
        ]
    );

    // Asked to come forward, it is the front one.
    let spec = NewWindowSpec { activate: true, ..Default::default() };
    let forward: WindowOpened = result(&ask(cx, Action::NewWindow(spec), None));
    assert_eq!(cx.update(|cx| app(cx).front), forward.window);

    // A folder that is not there opens nothing; the third window of the minute is the last.
    let windows = cx.update(|cx| app(cx).windows.len());
    let bad = NewWindowSpec { folder: Some("/definitely/not/here".into()), ..Default::default() };
    assert_eq!(code(&ask(cx, Action::NewWindow(bad), None)), Some(ErrorCode::NotFound));
    assert_eq!(cx.update(|cx| app(cx).windows.len()), windows);
    let _: WindowOpened = result(&ask(cx, Action::NewWindow(NewWindowSpec::default()), None));
    let limited = ask(cx, Action::NewWindow(NewWindowSpec::default()), None);
    assert_eq!(code(&limited), Some(ErrorCode::RateLimited));
}
