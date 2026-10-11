//! The terminal welcome in a real window: a replay opens a terminal with its own token, the program's
//! requests are let through only with it, and what they ask is done (and only what they ask).

use gpui_kit::TestAppContext;
use wormhole_core::agents::AgentId;
use wormhole_core::agents::integrations;
use wormhole_core::features::FeatureFlag;
use wormhole_core::intro::Handover;
use wormhole_core::ipc::control::{
    AgentFact, AgentsFacts, ErrorCode, HooksFact, Reply, WelcomeOp, WelcomeState,
};

use super::super::view_tests::{drive, isolate_in, read, start};
use super::super::{app, app_mut};
use super::{Plan, handle};
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
    WelcomeOp::State { folder: None }
}

fn code(reply: &Reply) -> Option<ErrorCode> {
    reply.err().map(|error| error.code)
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
    assert!(!facts.places.is_empty() && !facts.look.themes.is_empty());
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
    // Even with the right token a Trust of a folder that is no project is refused.
    let trust = WelcomeOp::Trust { folder: "/tmp".to_string() };
    assert!(matches!(
        code(&ask(cx, Some(tab), &token, trust)),
        Some(ErrorCode::NotFound | ErrorCode::NotAllowed)
    ));
}

#[gpui_kit::test]
fn finishing_shuts_the_door_and_closing_the_terminal_forgets_it(cx: &mut TestAppContext) {
    if isolate("finishing_shuts_the_door_and_closing_the_terminal_forgets_it") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.replay_welcome(Handover::Cursor, window, cx));
    let (tab, token) = welcome_terminal(cx);
    let finish = WelcomeOp::Finish { folder: None, skipped: false };
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
