//! Notifications in a real window: a click brings its terminal forward, a background agent's banner follows
//! its hooks, closing a terminal takes its banner down, and Jump goes to the terminal waiting longest. The
//! harness is `test_harness`.

use gpui_kit::{SystemNotificationResponse, TestAppContext};
use wormhole_core::config::InFront;

use crate::notifications;
use crate::settings::SettingsStore;
use crate::window::test_harness::{drive, hook, isolate_in, read, start};

#[gpui_kit::test]
fn clicking_a_notification_brings_its_terminal_forward(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "clicking_a_notification_brings_its_terminal_forward") {
        return;
    }
    let window = start(cx, 2);
    // Terminal 2 is the one in front.
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    read(cx, &window, |ws| assert_eq!(ws.active, 1));

    let click = |cx: &mut TestAppContext, tag: Click| {
        let tag = match tag {
            Click::Tab(tab) => notifications::tag(tab),
            Click::Foreign(tag) => tag.into(),
        };
        cx.simulate_system_notification_response(SystemNotificationResponse { tag, action_id: None });
        cx.run_until_parked();
    };
    // Someone else's notification: nothing moves.
    click(cx, Click::Foreign("gpui-component/notification/abc"));
    read(cx, &window, |ws| assert_eq!(ws.active, 1));
    // The first terminal.
    click(cx, Click::Tab(1));
    read(cx, &window, |ws| assert_eq!(ws.active, 0));
    // A terminal that has been closed since: the app comes forward, nothing else happens.
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    click(cx, Click::Tab(999));
    read(cx, &window, |ws| assert_eq!(ws.active, 1));
}

/// What a notification was posted for.
enum Click {
    Tab(i64),
    /// A tag of someone else's (GPUI Component's own notifications, say).
    Foreign(&'static str),
}

/// Banners while Wormhole is in front (the test window always is): the default there is nothing.
fn banners_in_front(cx: &mut TestAppContext) {
    cx.update(|cx| SettingsStore::update(cx, |s| s.notify_in_front = InFront::Banner));
}

#[gpui_kit::test]
fn a_background_sessions_notification_follows_its_agent(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_background_sessions_notification_follows_its_agent") {
        return;
    }
    let window = start(cx, 2);
    banners_in_front(cx);
    let notices = |cx: &mut TestAppContext| -> Vec<(String, String, String)> {
        cx.delivered_system_notifications()
            .into_iter()
            .map(|n| (n.tag.to_string(), n.title.to_string(), n.body.to_string()))
            .collect()
    };
    let tag = notifications::tag(2).to_string();
    // Read when a notice is checked: an unnamed terminal is named after what runs in it,
    // so the first hook, which names the agent, renames it.
    let name = |cx: &mut TestAppContext| {
        cx.update(|cx| window.workspace.read_with(cx, |ws, _| ws.sessions[1].name().to_string()))
            .expect("the window is open")
    };

    hook(cx, &window, 2, "Notification", Some("Allow Bash?"));
    let needs_you = format!("{} needs input", name(cx));
    assert_eq!(notices(cx), [(tag.clone(), needs_you, "Allow Bash?".to_string())]);
    // The agent finishes instead: the same tab's notification is replaced, not added to.
    hook(cx, &window, 2, "Stop", None);
    let finished = format!("{} finished", name(cx));
    assert_eq!(notices(cx), [(tag.clone(), finished, "Finished".to_string())]);
    // It starts on something else: "finished" is history.
    hook(cx, &window, 2, "UserPromptSubmit", None);
    assert!(notices(cx).is_empty());
    assert_eq!(cx.dismissed_system_notifications(), [notifications::tag(2)]);

    // Looking at the session takes a pending notice down too.
    hook(cx, &window, 2, "Stop", None);
    assert_eq!(notices(cx).len(), 1);
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    assert!(notices(cx).is_empty());

    // The session in front, in a window that is: nothing is announced while you are looking at it.
    drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
    assert!(drive(cx, &window, |_, window, _| window.is_window_active()), "the test window is in front");
    hook(cx, &window, 1, "Notification", Some("Allow?"));
    assert!(notices(cx).is_empty());
}

#[gpui_kit::test]
fn closing_a_terminal_takes_its_notification_down(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "closing_a_terminal_takes_its_notification_down") {
        return;
    }
    let window = start(cx, 1);
    banners_in_front(cx);
    drive(cx, &window, |ws, window, cx| ws.new_session(window, cx));
    // The new terminal is the selected one; the hook is for the first, now in the back.
    hook(cx, &window, 1, "Stop", None);
    assert_eq!(cx.delivered_system_notifications().len(), 1);
    drive(cx, &window, |ws, window, cx| ws.request_close_session(0, window, cx));
    assert!(cx.delivered_system_notifications().is_empty());
}

/// In front, nothing shows by default: the sidebar, the Dock badge, the menu bar item and ⌥⌘J
/// carry the status. A banner comes only when asked for.
/// A turn that ends while work it waits on runs in the background (a subagent's build, a `Monitor`)
/// isn't finished; one that leaves only the agent's own server running is.
#[gpui_kit::test]
fn waiting_on_background_work_is_not_finished(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "waiting_on_background_work_is_not_finished") {
        return;
    }
    let window = start(cx, 2);
    banners_in_front(cx);
    let finished = |cx: &mut TestAppContext| cx.delivered_system_notifications().len();

    hook(cx, &window, 2, "UserPromptSubmit", None);
    hook(cx, &window, 2, "Stop:background", None);
    hook(cx, &window, 2, "PostToolUse:wakes=b7", None);
    // The subagent handed back while its build runs on.
    hook(cx, &window, 2, "Stop:shells=srv,b7", None);
    assert_eq!(finished(cx), 0, "it still waits on the build");
    // The build woke it, and it finished.
    hook(cx, &window, 2, "UserPromptSubmit", None);
    hook(cx, &window, 2, "Stop:shells=srv", None);
    assert_eq!(finished(cx), 1, "only the server runs on");
}

#[gpui_kit::test]
fn in_front_a_background_terminal_posts_nothing_unless_banners_are_asked_for(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "in_front_a_background_terminal_posts_nothing_unless_banners_are_asked_for")
    {
        return;
    }
    let window = start(cx, 2);
    hook(cx, &window, 2, "Notification", Some("Allow Bash?"));
    assert!(cx.delivered_system_notifications().is_empty(), "Wormhole is in front: nothing is posted");
    hook(cx, &window, 2, "Stop", None);
    assert!(cx.delivered_system_notifications().is_empty(), "finishing is not told either");
    banners_in_front(cx);
    hook(cx, &window, 2, "Notification", Some("Allow Bash?"));
    assert_eq!(cx.delivered_system_notifications().len(), 1, "asked for: the same event is a banner");
}

#[gpui_kit::test]
fn jump_goes_to_the_terminal_waiting_longest_then_the_next(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "jump_goes_to_the_terminal_waiting_longest_then_the_next") {
        return;
    }
    let window = start(cx, 3);
    hook(cx, &window, 3, "Notification", Some("first"));
    std::thread::sleep(std::time::Duration::from_millis(5));
    hook(cx, &window, 2, "Notification", Some("second"));
    let jump = |cx: &mut TestAppContext| {
        cx.update(crate::window::ambient::jump_to_waiting);
        cx.run_until_parked();
        read(cx, &window, |ws| ws.sessions[ws.active].id)
    };
    assert_eq!(jump(cx), 3, "the one that has waited longest");
    assert_eq!(jump(cx), 2, "pressed again: the next one");
    assert_eq!(jump(cx), 3, "and round again");
}
