//! The notification rules at the app's edge: routes, words, bursts and what a click does.

use std::time::{Duration, Instant};

use gpui_kit::{SystemNotificationResponse, TestAppContext};
use wormhole_core::notify::{Kind, Level, Source, State};

use super::announce::{Happened, category, spoken_words};
use super::respond::{dismiss, tag};
use super::route::{Route, logs, route_for, settled};
use super::{Test, actions, init, permission, post, test_banner};
use crate::platform::notifications::ButtonKind;
use crate::platform::notifications::{Category, Notice, Permission};
use crate::terminal_view::Failure;
use crate::window::ambient::TabFacts;

fn facts(state: State) -> TabFacts {
    TabFacts {
        session: 1,
        tab: 7,
        name: "build".into(),
        state,
        since: Instant::now(),
        agent: None,
        source: Source::Plain,
        program: None,
        message: None,
        reported: false,
        failure: None,
        notice: None,
        foreground: None,
        folder: Some("scratch".into()),
        branch: None,
        changed: Some(3),
        selected: false,
        key: false,
    }
}

fn notice(tab: i64, title: &str, body: &str) -> Notice {
    Notice {
        id: wormhole_core::notify::notice_id(tab),
        thread: "t".into(),
        title: title.into(),
        subtitle: String::new(),
        body: body.into(),
        level: Level::Active,
        sound: false,
        category: Category::Finished,
    }
}

/// F-41: "build failed · exit 101", the command and folder under it, and after how long and the
/// last line it printed.
#[test]
fn a_failed_command_is_told_with_its_exit_code_and_last_line() {
    let mut build = facts(State::Failed);
    build.failure = Some(Failure {
        serial: 1,
        code: 101,
        at: Instant::now(),
        command: Some("cargo build".into()),
        took: Some(Duration::from_secs(38)),
        last_line: Some("error[E0425]: cannot find value `tab_ix`".into()),
    });
    let happened = Happened {
        kind: Kind::CommandFailed,
        facts: &build,
        took: Some(Duration::from_secs(38)),
        command: Some("cargo build"),
    };
    let text = happened.text();
    assert_eq!(text.title, "build failed · exit 101");
    assert_eq!(text.subtitle, "cargo build · scratch");
    assert_eq!(text.body, "after 38s · error[E0425]: cannot find value `tab_ix`");
    assert_eq!(spoken_words(Kind::CommandFailed, &build).as_deref(), Some("exit 101"));
    assert_eq!(category(Kind::CommandFailed), Category::Plain, "a click opens the terminal");
    // The last line is the shell's; without it the duration is what there is.
    build.failure = build.failure.map(|f| Failure { last_line: None, ..f });
    let happened = Happened { kind: Kind::CommandFailed, facts: &build, took: None, command: None };
    assert_eq!(happened.text().body, "after 38s");
}

/// A program that reports itself is named where an agent would be, its words are the body, and it
/// has no repository changes of its own to count.
#[test]
fn a_reporting_programs_banner_names_it_and_quotes_it() {
    let mut brew = facts(State::Done);
    brew.source = Source::Program;
    brew.program = Some("brew".into());
    brew.message = Some("Upgraded 12 packages".into());
    brew.changed = None;
    let happened =
        Happened { kind: Kind::AgentDone, facts: &brew, took: Some(Duration::from_secs(252)), command: None };
    let text = happened.text();
    assert_eq!(text.title, "build finished");
    assert_eq!(text.subtitle, "brew · scratch");
    assert_eq!(text.body, "Upgraded 12 packages · 4m 12s");
    brew.state = State::Failed;
    brew.message = Some("Error: no such formula".into());
    let happened = Happened { kind: Kind::AgentFailed, facts: &brew, took: None, command: None };
    assert_eq!(happened.text().title, "build failed");
    assert_eq!(happened.text().body, "Error: no such formula");
    assert_eq!(spoken_words(Kind::AgentFailed, &brew).as_deref(), Some("Error: no such formula"));
}

/// The bundled app always posts through the system's centre and never through GPUI, whose
/// macOS centre would replace the delegate the replies arrive at.
#[test]
fn the_bundled_app_never_routes_through_gpui() {
    assert_eq!(route_for(false, false, true, true), Route::Center);
    assert_eq!(route_for(false, false, true, false), Route::Script, "no bundle: the script");
    assert_eq!(route_for(false, false, false, true), Route::Gpui, "no notifications of the system's own");
    assert_eq!(route_for(false, false, false, false), Route::Gpui);
    assert_eq!(route_for(true, false, true, true), Route::Gpui, "a test");
    assert_eq!(route_for(false, true, true, true), Route::Log, "devctl's log");
    assert_eq!(settled(Route::Center, true), Route::Center);
    assert_eq!(settled(Route::Center, false), Route::Script, "a centre that can't be had: the script");
    assert_eq!(settled(Route::Gpui, false), Route::Gpui);
}

#[test]
fn reply_is_a_text_field_and_open_and_show_changes_bring_the_app_forward() {
    let kinds = |category| actions(category).iter().map(|b| (b.id, b.kind)).collect::<Vec<_>>();
    assert_eq!(kinds(Category::NeedsYou), [("open", ButtonKind::Foreground), ("reply", ButtonKind::Reply)]);
    assert_eq!(
        kinds(Category::Finished),
        [("open", ButtonKind::Foreground), ("show-changes", ButtonKind::Foreground)]
    );
    assert!(actions(Category::Plain).is_empty());
}

#[test]
fn background_mode_logs_notifications_unless_told_otherwise() {
    assert!(!logs(None, false), "the usual app posts them");
    assert!(logs(Some("log"), false));
    assert!(logs(None, true), "a test instance keeps them to the log");
    assert!(logs(Some("log"), true));
    assert!(!logs(Some("center"), true), "an explicit route wins");
    assert!(!logs(Some(""), false));
}

#[gpui_kit::test]
fn a_newer_notification_replaces_the_tabs_older_one(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_app_identity("app.getwormhole.terminal", "Wormhole");
        init(cx);
        post(notice(3, "api needs input", "Allow?"), cx);
        post(notice(3, "api finished", "Finished"), cx);
        post(notice(4, "web finished", "Finished"), cx);
    });
    let up = cx.delivered_system_notifications();
    assert_eq!(up.len(), 2, "one per tab: {up:?}");
    let three = up.iter().find(|n| n.tag == tag(3)).expect("tab 3's notification");
    assert_eq!((&*three.title, &*three.body), ("api finished", "Finished"));
    let labels: Vec<_> = three.actions.iter().map(|a| a.label.to_string()).collect();
    assert_eq!(labels, ["Open", "Show Changes"]);
}

#[gpui_kit::test]
fn taking_one_down_asks_the_system_only_when_one_is_up(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_app_identity("app.getwormhole.terminal", "Wormhole");
        init(cx);
        // The tick does this for the selected tab every 400 ms.
        dismiss(3, cx);
    });
    assert!(cx.dismissed_system_notifications().is_empty());
    cx.update(|cx| {
        post(notice(3, "api finished", "Finished"), cx);
        dismiss(3, cx);
        dismiss(3, cx);
    });
    assert_eq!(cx.dismissed_system_notifications(), [tag(3)]);
    assert!(cx.delivered_system_notifications().is_empty());
}

#[gpui_kit::test]
fn a_response_to_someone_elses_notification_changes_nothing(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_app_identity("app.getwormhole.terminal", "Wormhole");
        init(cx);
        post(notice(9, "api needs input", "Allow?"), cx);
    });
    cx.simulate_system_notification_response(SystemNotificationResponse {
        tag: "gpui-component/notification/abc".into(),
        action_id: None,
    });
    cx.run_until_parked();
    assert_eq!(cx.delivered_system_notifications().len(), 1, "ours stays up");
}

/// "Send test" never ends silent: it asks macOS first when that has not been asked, and sends once
/// the answer is yes (a test has no macOS to ask, so the question counts as answered yes).
#[gpui_kit::test]
fn send_test_asks_macos_first_and_then_sends(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_app_identity("app.getwormhole.terminal", "Wormhole");
        cx.set_global(crate::settings::SettingsStore::for_test());
        cx.set_global(crate::app_state::AppState::for_test(1));
        init(cx);
        super::pretend_permission(Permission::NotAsked, cx);
        super::send_test(cx);
        assert_eq!(super::permission_asks(cx), 1);
        assert_eq!(super::test_state(cx), Test::Sent);
    });
    assert_eq!(cx.delivered_system_notifications().len(), 1);
}

/// Off in System Settings, the system would drop a banner without a word: nothing is posted and the
/// row says why.
#[gpui_kit::test]
fn send_test_says_so_when_notifications_are_off(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_app_identity("app.getwormhole.terminal", "Wormhole");
        cx.set_global(crate::settings::SettingsStore::for_test());
        cx.set_global(crate::app_state::AppState::for_test(1));
        init(cx);
        super::pretend_permission(Permission::Denied, cx);
        super::send_test(cx);
        assert_eq!(super::test_state(cx), Test::Denied);
        assert_eq!(super::permission_asks(cx), 0, "after a no only System Settings can change it");
    });
    assert!(cx.delivered_system_notifications().is_empty());
}

/// While the permission is read or macOS's question is up, the press waits; polling the permission
/// while the question is open is not an answer, the question's own answer is.
#[gpui_kit::test]
fn send_test_waits_for_macos_and_gives_up_only_on_its_answer(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_app_identity("app.getwormhole.terminal", "Wormhole");
        cx.set_global(crate::settings::SettingsStore::for_test());
        cx.set_global(crate::app_state::AppState::for_test(1));
        init(cx);
        super::pretend_permission(Permission::Checking, cx);
        super::send_test(cx);
        assert_eq!(super::test_state(cx), Test::Waiting { asked: false });
        // Read: not asked yet, so now it asks.
        permission::learned(Permission::NotAsked, false, cx);
        assert_eq!(super::test_state(cx), Test::Waiting { asked: true });
        assert_eq!(super::permission_asks(cx), 1);
        // A poll while the question is up.
        permission::learned(Permission::NotAsked, false, cx);
        assert_eq!(super::test_state(cx), Test::Waiting { asked: true });
        // Its answer is still "not asked": no question ever showed.
        permission::learned(Permission::NotAsked, true, cx);
        assert_eq!(super::test_state(cx), Test::Unanswered);
        // Nothing waits any more, so a later answer sends nothing.
        permission::learned(Permission::Allowed { time_sensitive: false }, true, cx);
        assert_eq!(super::test_state(cx), Test::Unanswered);
    });
    assert!(cx.delivered_system_notifications().is_empty());
}

#[gpui_kit::test]
fn send_test_sends_when_the_answer_is_yes_and_shows_a_refusal(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_app_identity("app.getwormhole.terminal", "Wormhole");
        cx.set_global(crate::settings::SettingsStore::for_test());
        cx.set_global(crate::app_state::AppState::for_test(1));
        init(cx);
        super::pretend_permission(Permission::Checking, cx);
        super::send_test(cx);
        permission::learned(Permission::Allowed { time_sensitive: false }, false, cx);
        assert_eq!(super::test_state(cx), Test::Sent);
        // The centre's completion handler reports a refusal later.
        test_banner::posted("something else", Some("no".into()), cx);
        assert_eq!(super::test_state(cx), Test::Sent, "only the test's own answer counts");
        test_banner::posted(super::TEST_ID, Some("not allowed".into()), cx);
        assert_eq!(super::test_state(cx), Test::Refused("not allowed".into()));
    });
}
