//! The terminal welcome in a real window: a replay opens a terminal with its own token, the program's
//! requests are let through only with it, and what they ask is done (and only what they ask).

use gpui_kit::TestAppContext;
use wormhole_core::agents::AgentId;
use wormhole_core::agents::integrations;
use wormhole_core::config::NotifySound;
use wormhole_core::features::FeatureFlag;
use wormhole_core::intro::Handover;
use wormhole_core::ipc::control::{
    Action, AgentFact, AgentsFacts, ErrorCode, HooksFact, PermissionFact, Reply, Request, WelcomeOp,
    WelcomeState,
};
use wormhole_core::sidebar::RowState;

use super::super::notifications;
use super::super::view_tests::{draw, drive, isolate_in, read, start};
use super::super::{app, app_mut, control};
use super::{Plan, handle};
use crate::platform::notifications::Permission;
use crate::settings::settings;
use crate::workspace::settings_page::update_settings;

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// The welcome terminal a replay opened: its tab and token.
fn welcome_terminal(cx: &mut TestAppContext) -> (i64, String) {
    cx.update(|cx| {
        let run = app(cx).welcomes.runs.last().expect("a welcome is running");
        (run.tab, run.token.clone())
    })
}

fn ask(cx: &mut TestAppContext, from: Option<i64>, token: &str, op: WelcomeOp) -> Reply {
    cx.update(|cx| handle(token.to_string(), op, from, cx))
}

fn state_of(reply: &Reply) -> WelcomeState {
    serde_json::from_value(reply.result.clone().expect("a result")).expect("a state")
}

fn state() -> WelcomeOp {
    WelcomeOp::State
}

fn code(reply: &Reply) -> Option<ErrorCode> {
    reply.err().map(|error| error.code)
}

fn set_notify(
    needs_you: Option<bool>,
    agent_done: Option<bool>,
    sound: Option<NotifySound>,
    allow: bool,
) -> WelcomeOp {
    WelcomeOp::SetNotify { needs_you, agent_done, sound, allow }
}

/// What `Wormhole list` answers, as the JSON a script reads.
fn control_list(cx: &mut TestAppContext) -> serde_json::Value {
    let reply = cx.update(|cx| control::handle(Request::new(Action::List, None), cx));
    reply.result.expect("a listing")
}

#[gpui_kit::test]
fn a_replay_opens_a_welcome_terminal_and_only_its_token_opens_the_door(cx: &mut TestAppContext) {
    if isolate("a_replay_opens_a_welcome_terminal_and_only_its_token_opens_the_door") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Logo, window, cx));
    read(cx, &window, |ws| {
        assert_eq!(ws.sessions.len(), 2, "a new terminal for the welcome");
        assert_eq!(ws.active, 1, "and it is the one in front");
    });
    let (tab, token) = welcome_terminal(cx);
    assert_eq!(token.len(), 32);

    // The right terminal with its token: the facts.
    let reply = ask(cx, Some(tab), &token, state());
    let facts = state_of(&reply);
    assert_eq!(facts.handover, Handover::Logo);
    assert!(!facts.first_run, "a replay");
    assert!(!facts.look.themes.is_empty());
    assert!(facts.notify.is_some(), "and what reaches the person");
    assert_eq!(facts.wait_ms, 0, "no run plays in the test platform, so there is nothing to wait for");

    // Anyone else: refused, however it is asked.
    for (from, token) in [
        (None, token.as_str()),
        (Some(1), token.as_str()),
        (Some(tab), "wrong"),
        (Some(tab), ""),
        (Some(tab + 50), token.as_str()),
    ] {
        assert_eq!(code(&ask(cx, from, token, state())), Some(ErrorCode::Refused), "{from:?} {token:?}");
    }
}

/// With the flag on, Help → Welcome to Wormhole is a terminal that runs the welcome with the cursor
/// hand-over, not the intro window; with it off, nothing of this opens.
#[gpui_kit::test]
fn help_welcome_is_the_terminal_welcome_while_its_flag_is_on(cx: &mut TestAppContext) {
    if isolate("help_welcome_is_the_terminal_welcome_while_its_flag_is_on") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.show_intro(window, cx));
    read(cx, &window, |ws| {
        assert_eq!(ws.sessions.len(), 2, "a new terminal runs the welcome");
        assert!(!ws.first_run.is_open(), "and no intro window opens");
    });
    let (tab, token) = welcome_terminal(cx);
    assert_eq!(state_of(&ask(cx, Some(tab), &token, state())).handover, Handover::Cursor);

    // Off: the intro window, no new terminal.
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.feature_flags.insert(FeatureFlag::TerminalWelcome.key().to_string(), false);
        })
    });
    drive(cx, &window, |ws, window, cx| ws.show_intro(window, cx));
    cx.executor().advance_clock(std::time::Duration::from_millis(400));
    cx.run_until_parked();
    read(cx, &window, |ws| {
        assert_eq!(ws.sessions.len(), 2, "no terminal this time");
        assert!(ws.first_run.is_open(), "the intro window");
    });
}

#[gpui_kit::test]
fn the_look_changes_only_to_what_the_page_offers(cx: &mut TestAppContext) {
    if isolate("the_look_changes_only_to_what_the_page_offers") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);
    let look = |theme: Option<&str>, step: Option<i8>, ligatures: Option<bool>| WelcomeOp::SetLook {
        theme: theme.map(str::to_string),
        primary: None,
        ligatures,
        font_step: step,
    };
    let before =
        cx.update(|cx| (settings(cx).theme.clone(), settings(cx).font_size, settings(cx).font_ligatures));
    // A theme that is not on offer, whatever its name: refused, nothing changed.
    let refused = ask(cx, Some(tab), &token, look(Some("No Such Theme"), None, None));
    assert_eq!(code(&refused), Some(ErrorCode::NotFound));
    let steps = ask(cx, Some(tab), &token, look(None, Some(9), None));
    assert_eq!(code(&steps), Some(ErrorCode::BadRequest), "the font moves one step at a time");
    let unchanged =
        cx.update(|cx| (settings(cx).theme.clone(), settings(cx).font_size, settings(cx).font_ligatures));
    assert_eq!(before, unchanged);
    // One that is: applied, with the font a step bigger and the ligatures flipped.
    let offered = state_of(&ask(cx, Some(tab), &token, state())).look;
    let other =
        offered.themes.iter().find(|theme| theme.name != offered.theme).expect("another theme").name.clone();
    let reply = ask(cx, Some(tab), &token, look(Some(&other), Some(1), Some(!before.2)));
    assert!(reply.ok, "{reply:?}");
    let after =
        cx.update(|cx| (settings(cx).theme.clone(), settings(cx).font_size, settings(cx).font_ligatures));
    assert_eq!(after.0, other);
    assert_eq!(after.1, before.1 + 1.0);
    assert_eq!(after.2, !before.2);
}

/// What reaches the person is behind the same door as every other request: the welcome's terminal,
/// its token, while it runs. Nothing changes for a request that is turned away.
#[gpui_kit::test]
fn what_reaches_you_changes_only_for_the_welcomes_own_terminal_and_token(cx: &mut TestAppContext) {
    if isolate("what_reaches_you_changes_only_for_the_welcomes_own_terminal_and_token") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);
    cx.update(|cx| notifications::pretend_permission(Permission::NotAsked, cx));
    let now = |cx: &mut TestAppContext| {
        cx.update(|cx| {
            let s = settings(cx);
            (
                s.notifications,
                s.notify_needs_you,
                s.notify_agent_done,
                s.notify_sound,
                notifications::permission_asks(cx),
            )
        })
    };
    let before = now(cx);
    assert_eq!(before, (true, true, true, NotifySound::NeedsYou, 0), "the defaults");

    let everything = set_notify(Some(false), Some(false), Some(NotifySound::None), true);
    for (from, token) in [
        (None, token.as_str()),
        (Some(1), token.as_str()),
        (Some(tab), "wrong"),
        (Some(tab), ""),
        (Some(tab + 50), token.as_str()),
    ] {
        let reply = ask(cx, from, token, everything.clone());
        assert_eq!(code(&reply), Some(ErrorCode::Refused), "{from:?} {token:?}");
    }
    assert_eq!(now(cx), before, "turned away: nothing changed, and macOS was not asked");

    // Through the door: only what is named changes.
    assert!(ask(cx, Some(tab), &token, set_notify(None, Some(false), None, false)).ok);
    assert_eq!(now(cx), (true, true, false, NotifySound::NeedsYou, 0));
    assert!(ask(cx, Some(tab), &token, set_notify(None, None, Some(NotifySound::All), false)).ok);
    assert_eq!(now(cx), (true, true, false, NotifySound::All, 0));
    let facts = state_of(&ask(cx, Some(tab), &token, state())).notify.expect("notify facts");
    assert!((facts.needs_you, facts.agent_done, facts.sound) == (true, false, NotifySound::All));

    // Both kinds off turns the master switch off, and the page reads it as nothing on.
    assert!(ask(cx, Some(tab), &token, set_notify(Some(false), None, None, false)).ok);
    // The kinds the welcome has no switch for (a command finishing or failing, the bell) keep the
    // master switch on until they are off too.
    cx.update(|cx| {
        update_settings(cx, |s| s.change_notify_kind(|s| s.notify_command_done = false));
        update_settings(cx, |s| s.change_notify_kind(|s| s.notify_command_failed = false));
    });
    let facts = state_of(&ask(cx, Some(tab), &token, state())).notify.expect("notify facts");
    assert!(!facts.needs_you && !facts.agent_done);
    assert!(!cx.update(|cx| settings(cx).notifications), "no kind on: the master switch is off");

    // After the welcome has finished the token opens nothing.
    assert!(ask(cx, Some(tab), &token, WelcomeOp::Finish { skipped: false }).ok);
    let late = ask(cx, Some(tab), &token, set_notify(Some(true), None, None, true));
    assert_eq!(code(&late), Some(ErrorCode::Refused));
    assert!(!cx.update(|cx| settings(cx).notify_needs_you), "still off");
}

/// macOS is asked for permission by exactly two kinds of press: one that turns a banner on and the
/// page's own Allow. Showing the page, reading its state, turning things off and choosing a sound
/// never ask, and neither does anything once macOS has been asked.
#[gpui_kit::test]
fn macos_is_asked_only_when_a_press_turns_a_banner_on_or_says_allow(cx: &mut TestAppContext) {
    if isolate("macos_is_asked_only_when_a_press_turns_a_banner_on_or_says_allow") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);
    let asks = |cx: &mut TestAppContext| cx.update(|cx| notifications::permission_asks(cx));
    let permission = |cx: &mut TestAppContext| cx.update(|cx| notifications::permission(cx));
    cx.update(|cx| notifications::pretend_permission(Permission::NotAsked, cx));

    // The page showing, over and over: the state says "not asked", and nothing asks.
    for _ in 0..3 {
        let facts = state_of(&ask(cx, Some(tab), &token, state()));
        assert_eq!(facts.notify.map(|notify| notify.permission), Some(PermissionFact::NotAsked));
    }
    draw(cx, &window);
    assert_eq!(asks(cx), 0);

    // Off, a sound, a switch pressed to the state it already has: none of them turns a banner on.
    assert!(ask(cx, Some(tab), &token, set_notify(Some(false), None, None, false)).ok);
    assert!(ask(cx, Some(tab), &token, set_notify(None, Some(false), Some(NotifySound::All), false)).ok);
    assert!(ask(cx, Some(tab), &token, set_notify(Some(false), Some(false), None, false)).ok);
    assert_eq!(asks(cx), 0, "turning off, a sound and the same again ask nothing");
    assert_eq!(permission(cx), Permission::NotAsked);

    // Turning "Needs you" on is a banner turned on: macOS is asked, once.
    assert!(ask(cx, Some(tab), &token, set_notify(Some(true), None, None, false)).ok);
    assert_eq!(asks(cx), 1);
    assert!(matches!(permission(cx), Permission::Allowed { .. }), "(a test has no macOS to say no)");
    // It has been asked: neither another banner nor Allow asks again.
    assert!(ask(cx, Some(tab), &token, set_notify(None, Some(true), None, true)).ok);
    assert_eq!(asks(cx), 1);

    // The page's own Allow asks, while macOS has not been asked, whatever the switches say.
    cx.update(|cx| notifications::pretend_permission(Permission::NotAsked, cx));
    assert!(ask(cx, Some(tab), &token, set_notify(None, None, None, true)).ok);
    assert_eq!(asks(cx), 2);
    // Denied is final here: only System Settings can turn it on, so pressing Allow asks nothing.
    cx.update(|cx| notifications::pretend_permission(Permission::Denied, cx));
    assert!(ask(cx, Some(tab), &token, set_notify(Some(false), None, None, false)).ok);
    assert!(ask(cx, Some(tab), &token, set_notify(Some(true), None, None, true)).ok);
    assert_eq!(asks(cx), 2);
    assert_eq!(permission(cx), Permission::Denied);
}

#[gpui_kit::test]
fn hooks_are_written_only_for_an_agent_that_was_found_and_asked_for(cx: &mut TestAppContext) {
    if isolate("hooks_are_written_only_for_an_agent_that_was_found_and_asked_for") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);
    let install = |agent| WelcomeOp::InstallHooks { agent };
    // The page offers Codex (its hooks missing) and nothing else, whatever this Mac has on its PATH.
    cx.run_until_parked();
    cx.update(|cx| {
        let found = vec![AgentFact {
            id: AgentId::Codex,
            name: "Codex CLI".to_string(),
            hooks: HooksFact::Missing,
            path: None,
        }];
        app_mut(cx).welcomes.run_mut(tab).expect("the run").agents = Some(AgentsFacts { found, others: 3 });
    });
    let home = integrations::home();
    // An agent that wasn't offered is refused, and nothing is written for it.
    let reply = ask(cx, Some(tab), &token, install(AgentId::Claude));
    assert_eq!(code(&reply), Some(ErrorCode::NotFound));
    assert!(!home.join(".claude").exists(), "nothing was written under {}", home.display());
    // The one that was: its hooks are written, and only its.
    assert!(ask(cx, Some(tab), &token, install(AgentId::Codex)).ok);
    assert!(home.join(".codex").exists(), "Codex's hooks are in {}", home.display());
    assert!(!home.join(".claude").exists(), "and nothing else's");
    let facts = state_of(&ask(cx, Some(tab), &token, state()));
    let codex = facts.agents.expect("agents").found.remove(0);
    assert_eq!(codex.hooks, HooksFact::Installed, "the next state says so");
}

#[gpui_kit::test]
fn finishing_shuts_the_door_and_closing_the_terminal_forgets_it(cx: &mut TestAppContext) {
    if isolate("finishing_shuts_the_door_and_closing_the_terminal_forgets_it") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);
    let finish = WelcomeOp::Finish { skipped: false };
    assert!(ask(cx, Some(tab), &token, finish.clone()).ok);
    // Done: the same token opens nothing now.
    assert_eq!(code(&ask(cx, Some(tab), &token, state())), Some(ErrorCode::Refused));
    assert_eq!(code(&ask(cx, Some(tab), &token, finish)), Some(ErrorCode::Refused));

    // A second welcome, closed with its terminal: gone, and so is its door.
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Portal, window, cx));
    let (second, second_token) = welcome_terminal(cx);
    assert_ne!(second, tab);
    assert!(ask(cx, Some(second), &second_token, state()).ok);
    drive(cx, &window, |ws, window, cx| {
        let ix = ws.find_session(second).expect("the welcome's terminal");
        ws.close_session(ix, window, cx);
    });
    assert_eq!(code(&ask(cx, Some(second), &second_token, state())), Some(ErrorCode::Refused));
    cx.update(|cx| assert!(app(cx).welcomes.run(second).is_none()));
}

#[gpui_kit::test]
fn the_first_launch_makes_the_windows_first_terminal_the_welcome(cx: &mut TestAppContext) {
    if isolate("the_first_launch_makes_the_windows_first_terminal_the_welcome") {
        return;
    }
    // The window opens with no terminal of its own, and the plan says what its first one is.
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_app_identity("app.getwormhole.terminal.test", "Wormhole");
        cx.set_global(crate::settings::SettingsStore::load());
        cx.set_global(super::super::app::AppState::for_test(100));
        super::super::actions::init(cx);
        app_mut(cx).first_welcome = Some(Plan { handover: Handover::Portal, first_run: true });
        super::super::app::open_workspace_window(None, None, cx);
        app_mut(cx).first_welcome = None;
    });
    cx.run_until_parked();
    let (tab, token) = welcome_terminal(cx);
    let window = cx.update(|cx| app(cx).windows.first().cloned()).expect("the window opened");
    read(cx, &window, |ws| {
        assert_eq!(ws.sessions.len(), 1);
        assert_eq!(ws.sessions[0].id, tab, "the one terminal is the welcome");
    });
    let facts = state_of(&ask(cx, Some(tab), &token, state()));
    assert!(facts.first_run);
    assert_eq!(facts.handover, Handover::Portal);
}

// ------------------------------------------------------------------------- the example band

/// The three states the band shows, as `state` would list them.
fn band(cx: &mut TestAppContext, window: &super::super::app::OpenWindow) -> Option<Vec<RowState>> {
    cx.update(|cx| {
        window
            .workspace
            .read_with(cx, |ws, cx| ws.welcome_demo_rows(cx).map(|rows| rows.map(|row| row.state).to_vec()))
            .expect("the window is open")
    })
}

/// What a script, the saved window and the sidebar's list each say of the window's terminals.
fn terminals_as_seen(
    cx: &mut TestAppContext,
    window: &super::super::app::OpenWindow,
) -> (serde_json::Value, String, wormhole_core::sidebar::Listing, usize) {
    let (record, listing, sessions) = cx.update(|cx| {
        window
            .workspace
            .read_with(cx, |ws, cx| (ws.window_record(cx), ws.session_listing(cx), ws.sessions.len()))
            .expect("the window is open")
    });
    (control_list(cx), format!("{record:?}"), listing, sessions)
}

/// The example band is shown for the sidebar step and nothing else: it comes up when the program
/// asks, stays out of everything that lists, saves or numbers terminals, and is gone when the step is
/// left, the welcome finishes or its terminal closes.
#[gpui_kit::test]
fn the_example_band_comes_up_with_the_step_and_is_no_terminal(cx: &mut TestAppContext) {
    if isolate("the_example_band_comes_up_with_the_step_and_is_no_terminal") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);
    let demo = |show| WelcomeOp::Demo { show };
    assert_eq!(band(cx, &window), None, "nothing until the program asks");
    let before = terminals_as_seen(cx, &window);

    // Not for just anyone: another terminal, a wrong token, a stranger outside any terminal.
    for (from, token) in [(None, token.as_str()), (Some(1), token.as_str()), (Some(tab), "wrong")] {
        assert_eq!(code(&ask(cx, from, token, demo(true))), Some(ErrorCode::Refused), "{from:?}");
    }
    assert_eq!(band(cx, &window), None);

    // Asked: three rows, one in each state, drawn without any trouble.
    assert!(ask(cx, Some(tab), &token, demo(true)).ok);
    cx.run_until_parked();
    assert_eq!(
        band(cx, &window),
        Some(vec![RowState::Working, RowState::NeedsYou, RowState::Finished]),
        "working, needs you, done"
    );
    draw(cx, &window);

    // …and none of it is a terminal: the window's sessions, the sidebar's list and its numbers, what
    // the window saves and what `Wormhole list` says are exactly what they were.
    assert_eq!(terminals_as_seen(cx, &window), before);

    // Asking again keeps it as it is; the step left takes it away; and it can come back.
    assert!(ask(cx, Some(tab), &token, demo(true)).ok);
    assert!(band(cx, &window).is_some());
    assert!(ask(cx, Some(tab), &token, demo(false)).ok);
    assert_eq!(band(cx, &window), None, "the step was left");
    draw(cx, &window);
    assert!(ask(cx, Some(tab), &token, demo(true)).ok);
    assert!(band(cx, &window).is_some());

    // The welcome finishing takes it away, and the door is shut behind it.
    assert!(ask(cx, Some(tab), &token, WelcomeOp::Finish { skipped: false }).ok);
    assert_eq!(band(cx, &window), None, "the welcome is over");
    assert_eq!(code(&ask(cx, Some(tab), &token, demo(true))), Some(ErrorCode::Refused));
    assert_eq!(band(cx, &window), None, "and cannot be asked for again");
    assert_eq!(terminals_as_seen(cx, &window), before);
}

#[gpui_kit::test]
fn the_example_band_goes_when_its_terminal_closes(cx: &mut TestAppContext) {
    if isolate("the_example_band_goes_when_its_terminal_closes") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);
    assert!(ask(cx, Some(tab), &token, WelcomeOp::Demo { show: true }).ok);
    assert!(band(cx, &window).is_some());
    drive(cx, &window, |ws, window, cx| {
        let ix = ws.find_session(tab).expect("the welcome's terminal");
        ws.close_session(ix, window, cx);
    });
    assert_eq!(band(cx, &window), None, "the terminal is gone");
    read(cx, &window, |ws| assert!(ws.sidebar.demo.is_none(), "and so is the state, not just the drawing"));
    draw(cx, &window);
}

#[gpui_kit::test]
fn the_example_band_goes_with_its_window(cx: &mut TestAppContext) {
    if isolate("the_example_band_goes_with_its_window") {
        return;
    }
    let window = start(cx, 1);
    cx.update(|cx| super::super::app::open_workspace_window(None, None, cx));
    cx.run_until_parked();
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);
    assert!(ask(cx, Some(tab), &token, WelcomeOp::Demo { show: true }).ok);
    assert!(band(cx, &window).is_some());
    let workspace = window.workspace.clone();
    cx.update(|cx| window.handle.update(cx, |_, window, _| window.remove_window()))
        .expect("the window is open");
    cx.run_until_parked();
    assert!(workspace.upgrade().is_none(), "the window's workspace, and the band with it, is dropped");
}

/// A request for the band from a terminal of another window than the welcome's own cannot put it in
/// the wrong sidebar: it goes where the terminal is.
#[gpui_kit::test]
fn the_example_band_goes_in_the_sidebar_of_the_welcomes_own_window(cx: &mut TestAppContext) {
    if isolate("the_example_band_goes_in_the_sidebar_of_the_welcomes_own_window") {
        return;
    }
    let first = start(cx, 1);
    cx.update(|cx| super::super::app::open_workspace_window(None, None, cx));
    cx.run_until_parked();
    let second =
        cx.update(|cx| app(cx).windows.iter().find(|w| w.id != first.id).cloned()).expect("a second window");
    drive(cx, &second, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);
    assert!(ask(cx, Some(tab), &token, WelcomeOp::Demo { show: true }).ok);
    assert!(band(cx, &second).is_some(), "in the window the welcome runs in");
    assert_eq!(band(cx, &first), None, "and in no other");
}
