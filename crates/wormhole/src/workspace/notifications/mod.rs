//! Notifications: native banners for what happens in a terminal you aren't looking at
//! (while Wormhole is in front only if asked for: nothing shows inside the window), the Dock's
//! bounce, and what a click, Reply or Show Changes does.
//!
//! The rules are core's (`wormhole_core::notify`). Every tick (and right after an agent's
//! hook) [`sweep`] reads every tab of every window, `watch` turns what changed into events, and
//! the rules decide. Banners go out by one of these routes:
//! - the bundled app: `UNUserNotificationCenter` (`center`), with interruption levels (Time
//!   Sensitive for "needs you" when the build is entitled to it), a thread per terminal, sound,
//!   and the Open / Reply… / Show Changes actions;
//! - a dev build (`cargo run`) has no bundle, and the notification center would abort the
//!   process: `osascript` instead, without actions or levels;
//! - tests: GPUI's test platform. devctl builds can ask for `WORMHOLE_NOTIFY=log`, which keeps
//!   them in a list instead of posting anything; a test instance in background mode does that
//!   unless `WORMHOLE_NOTIFY` says otherwise.
//!
//! One notification per tab: a newer one replaces the older, and looking at the tab (or the
//! agent moving on) takes it down.

#[cfg(target_os = "macos")]
mod center;
mod watch;

use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::time::{Instant, SystemTime};

use wormhole_core::config::Settings;
use wormhole_core::notify::{
    self, About, Announcer, Bursts, Grouped, Kind, Level, Priority, Said, Seen, Text, Throttle,
};
use wormhole_core::update::current_app_bundle;

use super::ambient::{self, TabFacts};
use super::app::{AppState, app, open_tab};
use super::spawn::spawn_detached;
use super::*;
use watch::{Event, Watch};

/// The actions a notification can carry (`Notice::category` picks which).
pub(crate) const OPEN: &str = "open";
pub(crate) const REPLY: &str = "reply";
pub(crate) const SHOW_CHANGES: &str = "show-changes";
/// The banner "Send test" posts.
const TEST_ID: &str = "wormhole/test";

/// Which buttons a notification carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Category {
    /// Open, Reply…
    NeedsYou,
    /// Open, Show Changes.
    Finished,
    /// No buttons; a click opens.
    Plain,
}

/// One notification, ready for any route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Notice {
    pub id: String,
    pub thread: String,
    pub title: String,
    pub subtitle: String,
    pub body: String,
    pub level: Level,
    pub sound: bool,
    pub category: Category,
}

/// A click on one of our notifications, or one of its actions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Response {
    pub id: String,
    /// `None` for a click on the notification itself.
    pub action: Option<String>,
    /// What was typed into Reply.
    pub reply: Option<String>,
}

/// Whether macOS lets Wormhole notify.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) enum Permission {
    Checking,
    /// macOS hasn't asked yet; nothing is shown until it has.
    NotAsked,
    /// `time_sensitive`: the user lets Time Sensitive notifications through.
    Allowed {
        time_sensitive: bool,
    },
    /// Off in System Settings.
    Denied,
    /// A dev build: `osascript`, no actions or grouping.
    OutsideBundle,
}

/// What the notification center reports back, from any thread.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) enum FromCenter {
    Response(Response),
    Permission(Permission),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Route {
    Center,
    Osascript,
    /// GPUI's test platform.
    Gpui,
    /// devctl: kept in [`Notifier::log`], never posted.
    #[cfg_attr(not(feature = "devctl"), allow(dead_code))]
    Log,
    #[default]
    Off,
}

fn route() -> Route {
    if cfg!(test) {
        return Route::Gpui;
    }
    #[cfg(feature = "devctl")]
    if logs(std::env::var("WORMHOLE_NOTIFY").ok().as_deref(), crate::background::on()) {
        return Route::Log;
    }
    match () {
        _ if !cfg!(target_os = "macos") => Route::Off,
        _ if current_app_bundle().is_some() => Route::Center,
        _ => Route::Osascript,
    }
}

/// devctl builds: whether notifications go to the log (`notify-log`) and nowhere else.
/// `WORMHOLE_NOTIFY=log` asks for it, and background mode does unless `WORMHOLE_NOTIFY`
/// names another route.
#[cfg_attr(not(feature = "devctl"), allow(dead_code))]
fn logs(notify: Option<&str>, background: bool) -> bool {
    match notify {
        Some(route) => route == "log",
        None => background,
    }
}

/// The app's notification state.
#[derive(Default)]
struct Notifier {
    route: Route,
    watch: Watch,
    bursts: Bursts,
    bells: Option<Throttle>,
    /// What VoiceOver was told lately, so a burst is said as a count.
    announcer: Announcer,
    /// The ids of the notifications that are up, so taking one down asks the system only when
    /// there is something to take down (the tick checks the tab in front every 400 ms).
    up: HashSet<String>,
    permission: Option<Permission>,
    /// What the `Log` route would have posted (devctl reads it).
    log: Vec<Notice>,
    #[cfg(target_os = "macos")]
    center: Option<center::Center>,
}

impl Global for Notifier {}

fn notifier(cx: &mut App) -> &mut Notifier {
    cx.default_global::<Notifier>()
}

/// Set up the route and the handling of clicks and replies. Call once, at start.
pub(crate) fn init(cx: &mut App) {
    let (tx, rx) = smol::channel::unbounded();
    let route = route();
    #[cfg(target_os = "macos")]
    let center = (route == Route::Center).then(|| center::Center::open(tx.clone())).flatten();
    #[cfg(target_os = "macos")]
    let route = if route == Route::Center && center.is_none() { Route::Osascript } else { route };
    #[cfg(not(target_os = "macos"))]
    let _ = &tx;
    let permission = match route {
        Route::Center => Permission::Checking,
        _ => Permission::OutsideBundle,
    };
    cx.set_global(Notifier {
        route,
        bells: Some(Throttle::new(notify::BELL_RATE)),
        permission: Some(permission),
        #[cfg(target_os = "macos")]
        center,
        ..Notifier::default()
    });
    if route == Route::Gpui {
        // GPUI keeps one response handler for the whole app (GPUI Component installs its own in
        // `gpui_kit::init`); this replaces it, which is harmless while nothing posts a Component
        // notification with system delivery. The bundled app uses its own delegate (`center`).
        cx.on_system_notification_response(|response, cx| {
            let action = response.action_id.map(|id| id.to_string());
            respond(Response { id: response.tag.to_string(), action, reply: None }, cx);
        });
    }
    cx.spawn(async move |cx| {
        while let Ok(message) = rx.recv().await {
            cx.update(|cx| match message {
                FromCenter::Response(response) => respond(response, cx),
                FromCenter::Permission(permission) => {
                    if notifier(cx).permission.replace(permission) != Some(permission) {
                        cx.refresh_windows();
                    }
                }
            });
        }
    })
    .detach();
    refresh_permission(cx);
}

/// Once per app tick: what changed, and (while the settings window is open) whether macOS still
/// allows notifications.
pub(crate) fn tick(ticks: u64, cx: &mut App) {
    sweep(cx);
    if ticks.is_multiple_of(5) && app(cx).settings_window.is_some() {
        refresh_permission(cx);
    }
}

/// Read every tab of every window and act on what changed since the last look. Never call it
/// from inside a window's update (that window can't be read then): defer it.
pub(crate) fn sweep(cx: &mut App) {
    let tabs = ambient::all_tabs(cx);
    let app_active = crate::dock::app_is_active(cx);
    let after = settings(cx).notify_command_after;
    let events = notifier(cx).watch.observe(&tabs, app_active, after, SystemTime::now());
    if events.is_empty() {
        return;
    }
    let settings = settings(cx).clone();
    let mut said = Vec::new();
    for event in events {
        match event {
            Event::Post { tab, kind, took, command } => {
                if let Some(facts) = tabs.iter().find(|facts| facts.tab == tab) {
                    said.push(Said { tab, name: &facts.name, kind, question: spoken_words(kind, facts) });
                    let happened = Happened { kind, facts, took, command: command.as_deref() };
                    announce(happened, app_active, &settings, cx);
                }
            }
            Event::Retract(tab) | Event::Seen(tab) => dismiss(tab, cx),
        }
    }
    // VoiceOver hears every terminal that needs you or finished, the one on screen too: unlike a
    // banner, nothing else would read it out.
    for spoken in notifier(cx).announcer.announce(Instant::now(), &said) {
        crate::a11y::announce(&spoken.text, spoken.priority == Priority::High);
    }
}

/// One event, with what it is about.
struct Happened<'a> {
    kind: Kind,
    facts: &'a TabFacts,
    took: Option<std::time::Duration>,
    command: Option<&'a str>,
}

impl Happened<'_> {
    fn text(&self) -> Text {
        let facts = self.facts;
        let command = self.command.or(facts.foreground.as_ref().map(|f| f.command.as_str()));
        // A program's own notification speaks with its words, not the agent's question.
        let notice = facts.notice.as_ref().filter(|_| self.kind == Kind::Notice).map(|n| &n.notification);
        notify::text(
            self.kind,
            &About {
                name: &facts.name,
                agent: facts.agent_name(),
                command,
                folder: facts.folder.as_deref(),
                branch: facts.branch.as_deref(),
                message: notice.map(|n| n.body.as_str()).or(facts.message.as_deref()),
                title: notice.map(|n| n.title.as_str()),
                took: self.took,
                changed: facts.changed,
            },
        )
    }
}

/// What VoiceOver reads of an event: a program's notification in its words, else the agent's
/// question.
fn spoken_words(kind: Kind, facts: &TabFacts) -> Option<&str> {
    match kind {
        Kind::Notice => {
            let notice = facts.notice.as_ref()?;
            [&notice.notification.body, &notice.notification.title]
                .into_iter()
                .find(|s| !s.is_empty())
                .map(String::as_str)
        }
        _ => facts.message.as_deref(),
    }
}

/// Tell the user about one event as the rules say: a banner (maybe as part of a burst's
/// summary) or nothing, and the Dock's bounce.
fn announce(happened: Happened, app_active: bool, settings: &Settings, cx: &mut App) {
    let Happened { kind, facts, .. } = happened;
    if notify::bounces(kind, app_active, settings.dock_bounce) {
        crate::dock::bounce(cx);
    }
    if kind == Kind::Bell
        && !notifier(cx).bells.get_or_insert_with(bell_rate).allow(facts.session, Instant::now())
    {
        return;
    }
    let seen = Seen { looking: facts.looking(app_active), app_active };
    match notify::delivery(kind, seen, settings) {
        notify::Delivery::Nothing => {}
        notify::Delivery::Banner => banner(&happened, settings, cx),
    }
}

fn bell_rate() -> Throttle {
    Throttle::new(notify::BELL_RATE)
}

fn banner(happened: &Happened, settings: &Settings, cx: &mut App) {
    let Happened { kind, facts, .. } = *happened;
    let text = happened.text();
    let level = notify::interruption_level(kind, time_sensitive(cx));
    let single = Notice {
        id: notify::notice_id(facts.tab),
        thread: notify::thread_id(facts.session),
        title: text.title,
        subtitle: text.subtitle,
        body: text.body,
        level,
        sound: notify::sound(kind, level, settings.notify_sound),
        category: category(kind),
    };
    // What needs you is never folded into a summary: it is the one worth a banner of its own.
    if !settings.notify_group_bursts || kind == Kind::NeedsYou {
        post(single, cx);
        return;
    }
    match notifier(cx).bursts.add(Instant::now(), facts.tab, &facts.name, kind) {
        Grouped::Single => post(single, cx),
        Grouped::Summary { text, retract } => {
            for tab in retract {
                dismiss(tab, cx);
            }
            post(
                Notice {
                    id: notify::SUMMARY_ID.to_string(),
                    thread: notify::SUMMARY_ID.to_string(),
                    title: text.title,
                    subtitle: text.subtitle,
                    body: text.body,
                    level: Level::Active,
                    sound: notify::sound(kind, Level::Active, settings.notify_sound),
                    category: Category::Plain,
                },
                cx,
            );
        }
    }
}

fn category(kind: Kind) -> Category {
    match kind {
        Kind::NeedsYou => Category::NeedsYou,
        Kind::AgentDone => Category::Finished,
        Kind::CommandDone | Kind::Bell | Kind::Notice => Category::Plain,
    }
}

/// Whether "needs you" goes out as Time Sensitive: only from a bundle signed with the entitlement
/// (a Developer ID build with its provisioning profile); every other build uses Active.
pub(crate) fn time_sensitive(cx: &mut App) -> bool {
    #[cfg(target_os = "macos")]
    if notifier(cx).route == Route::Center {
        return center::entitled_to_time_sensitive();
    }
    let _ = cx;
    false
}

fn post(notice: Notice, cx: &mut App) {
    let notifier = notifier(cx);
    notifier.up.insert(notice.id.clone());
    match notifier.route {
        #[cfg(target_os = "macos")]
        Route::Center => {
            if let Some(center) = &notifier.center {
                center.post(&notice);
            }
        }
        Route::Osascript => {
            if let Err(err) = spawn_detached(osascript(&notice)) {
                tracing::warn!("could not show a notification: {err}");
            }
        }
        Route::Gpui => {
            let actions = actions(notice.category)
                .iter()
                .map(|(id, label)| SystemNotificationAction { id: (*id).into(), label: (*label).into() })
                .collect();
            cx.show_system_notification(SystemNotification {
                tag: notice.id.into(),
                title: notice.title.into(),
                body: notice.body.into(),
                actions,
            });
        }
        Route::Log => notifier.log.push(notice),
        _ => {}
    }
}

/// A category's buttons: id and label.
pub(crate) fn actions(category: Category) -> &'static [(&'static str, &'static str)] {
    match category {
        Category::NeedsYou => &[(OPEN, "Open"), (REPLY, "Reply…")],
        Category::Finished => &[(OPEN, "Open"), (SHOW_CHANGES, "Show Changes")],
        Category::Plain => &[],
    }
}

/// Take down a notification by its id, if it is up.
fn take_down(id: &str, cx: &mut App) {
    let notifier = notifier(cx);
    if !notifier.up.remove(id) {
        return;
    }
    match notifier.route {
        #[cfg(target_os = "macos")]
        Route::Center => {
            if let Some(center) = &notifier.center {
                center.remove(id);
            }
        }
        Route::Gpui => cx.dismiss_system_notification(id),
        Route::Log => notifier.log.retain(|notice| notice.id != id),
        _ => {}
    }
}

/// Take down `tab`'s notification, if there is one.
pub(crate) fn dismiss(tab: i64, cx: &mut App) {
    take_down(&notify::notice_id(tab), cx);
}

/// A tab's notification tag.
#[cfg(test)]
pub(super) fn tag(tab: i64) -> SharedString {
    notify::notice_id(tab).into()
}

/// The last command that ended by itself in each tab, and how long it ran.
pub(crate) fn finished_commands(cx: &App) -> Option<&HashMap<i64, (String, std::time::Duration)>> {
    cx.try_global::<Notifier>().map(|notifier| &notifier.watch.finished)
}

/// A click or an action on one of our notifications (from the platform, outside any window's
/// update): the window, terminal and tab come forward; Reply types into the tab and presses
/// Return without bringing anything forward.
pub(crate) fn respond(response: Response, cx: &mut App) {
    tracing::debug!(?response, "notification response");
    take_down(&response.id, cx);
    let Some(tab) = notify::notice_tab(&response.id) else {
        // The summary or the test banner: Wormhole comes forward.
        if response.id == notify::SUMMARY_ID || response.id == TEST_ID {
            crate::background::activate(cx);
            if let Some(front) = AppState::front_window(cx) {
                let _ =
                    front.handle.update(cx, |_, window, cx| crate::background::activate_window(window, cx));
            }
        }
        return;
    };
    match response.action.as_deref() {
        Some(REPLY) => {
            if let Some(text) = response.reply {
                reply(tab, &text, cx);
            }
        }
        Some(SHOW_CHANGES) => {
            crate::background::activate(cx);
            open_tab(tab, cx);
            if let Some(owner) = AppState::find_window(cx, |ws, _| ws.find_session(tab).is_some()) {
                AppState::with_workspace(&owner, cx, |ws, _, cx| ws.show_tab(RightTab::Changes, cx));
            }
        }
        _ => {
            crate::background::activate(cx);
            open_tab(tab, cx);
        }
    }
}

/// Type `text` into `tab`'s terminal and press Return (Reply, from a notification or the quick
/// panel), in whichever window it is.
pub(crate) fn reply(tab: i64, text: &str, cx: &mut App) {
    let Some(owner) = AppState::find_window(cx, |ws, _| ws.find_session(tab).is_some()) else { return };
    AppState::with_workspace(&owner, cx, |ws, _, cx| ws.type_reply(tab, text, cx));
}

impl Workspace {
    /// Paste `text` into a tab and press Return a moment later (the program reads the paste
    /// first). Returns whether there was anything to send.
    pub(crate) fn type_reply(&mut self, tab: i64, text: &str, cx: &mut Context<Self>) -> bool {
        let text = text.trim();
        let Some(terminal) = self.pane_terminal(tab) else { return false };
        if text.is_empty() {
            return false;
        }
        terminal.update(cx, |t, _| t.paste(text));
        cx.spawn(async move |_, cx| {
            cx.background_executor().timer(Duration::from_millis(60)).await;
            terminal.update(cx, |t, _| t.write(b"\r".to_vec()));
        })
        .detach();
        true
    }
}

/// Whether macOS lets Wormhole notify, as last read.
pub(crate) fn permission(cx: &App) -> Permission {
    cx.try_global::<Notifier>().and_then(|n| n.permission).unwrap_or(Permission::OutsideBundle)
}

/// Read the permission again (the answer arrives a moment later).
pub(crate) fn refresh_permission(cx: &mut App) {
    #[cfg(target_os = "macos")]
    if let Some(center) = &notifier(cx).center {
        center.refresh_permission();
    }
    let _ = cx;
}

/// "Allow notifications": macOS asks the user (once; after that only System Settings can).
pub(crate) fn ask_permission(cx: &mut App) {
    #[cfg(target_os = "macos")]
    if let Some(center) = &notifier(cx).center {
        center.ask();
    }
    let _ = cx;
}

/// "Open System Settings": Wormhole's page under Notifications.
pub(crate) fn open_system_settings() {
    let mut command = Command::new("open");
    command.arg("x-apple.systempreferences:com.apple.Notifications-Settings.extension");
    if let Err(err) = spawn_detached(command) {
        tracing::warn!("could not open System Settings: {err}");
    }
}

/// "Send test": a real banner, as "needs you" would be, about the terminal in front.
pub(crate) fn send_test(cx: &mut App) {
    let name = AppState::front_window(cx)
        .and_then(|front| front.workspace.read_with(cx, |ws, cx| ws.active_session().name(cx)).ok())
        .unwrap_or_else(|| "Terminal".into());
    let level = notify::interruption_level(Kind::NeedsYou, time_sensitive(cx));
    let sound = notify::sound(Kind::NeedsYou, level, settings(cx).notify_sound);
    post(
        Notice {
            id: TEST_ID.to_string(),
            thread: TEST_ID.to_string(),
            title: format!("{name} needs you"),
            subtitle: "Wormhole · test".to_string(),
            body: "This is how a terminal that needs you is announced.".to_string(),
            level,
            sound,
            category: Category::Plain,
        },
        cx,
    );
}

/// devctl: what the `Log` route kept, oldest first.
#[cfg(feature = "devctl")]
pub(crate) fn logged(cx: &App) -> Vec<Notice> {
    cx.try_global::<Notifier>().map(|n| n.log.clone()).unwrap_or_default()
}

/// devctl: run one event through the rules, as if it had just happened in `facts`'s tab, and say
/// what they decided (nothing is posted unless `post` is set).
#[cfg(feature = "devctl")]
pub(crate) fn simulate(kind: Kind, facts: &TabFacts, post_it: bool, cx: &mut App) -> String {
    let app_active = crate::dock::app_is_active(cx);
    let settings = settings(cx).clone();
    let seen = Seen { looking: facts.looking(app_active), app_active };
    let delivery = notify::delivery(kind, seen, &settings);
    let level = notify::interruption_level(kind, time_sensitive(cx));
    let happened = Happened { kind, facts, took: None, command: None };
    let text = happened.text();
    let line = format!(
        "notify-sim {kind:?} tab={} app_active={app_active} looking={} -> {delivery:?} level={level:?} \
         sound={} bounce={} title={:?} subtitle={:?} body={:?} actions={:?}\n",
        facts.tab,
        seen.looking,
        notify::sound(kind, level, settings.notify_sound),
        notify::bounces(kind, app_active, settings.dock_bounce),
        text.title,
        text.subtitle,
        text.body,
        actions(category(kind)).iter().map(|(_, label)| *label).collect::<Vec<_>>(),
    );
    if post_it {
        announce(happened, app_active, &settings, cx);
    }
    line
}

/// The dev build's way: no bundle, so no notification center. The script gets the text as
/// AppleScript string literals, so it is escaped rather than passed as arguments.
fn osascript(notice: &Notice) -> Command {
    let escape = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        "display notification \"{}\" with title \"{}\" subtitle \"{}\"",
        escape(&notice.body),
        escape(&notice.title),
        escape(&notice.subtitle)
    );
    let mut command = Command::new("osascript");
    command.arg("-e").arg(script);
    command
}

#[cfg(test)]
mod tests {
    use gpui_kit::{SystemNotificationResponse, TestAppContext};
    use wormhole_core::notify::Level;

    use super::{Category, Notice, dismiss, init, logs, osascript, post, tag};

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

    #[test]
    fn background_mode_logs_notifications_unless_told_otherwise() {
        assert!(!logs(None, false), "the usual app posts them");
        assert!(logs(Some("log"), false));
        assert!(logs(None, true), "a test instance keeps them to the log");
        assert!(logs(Some("log"), true));
        assert!(!logs(Some("center"), true), "an explicit route wins");
        assert!(!logs(Some(""), false));
    }

    #[test]
    fn the_fallback_script_escapes_quotes_and_backslashes() {
        let mut n = notice(1, r#"my "terminal" finished"#, r#"C:\temp says "hi""#);
        n.subtitle = "Claude Code · api".into();
        let command = osascript(&n);
        let args: Vec<_> = command.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(command.get_program(), "osascript");
        assert_eq!(args[0], "-e");
        assert_eq!(
            args[1],
            r#"display notification "C:\\temp says \"hi\"" with title "my \"terminal\" finished" subtitle "Claude Code · api""#
        );
    }

    #[gpui_kit::test]
    fn a_newer_notification_replaces_the_tabs_older_one(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_app_identity("app.getwormhole.terminal", "Wormhole");
            init(cx);
            post(notice(3, "api needs you", "Allow?"), cx);
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
            post(notice(9, "api needs you", "Allow?"), cx);
        });
        cx.simulate_system_notification_response(SystemNotificationResponse {
            tag: "gpui-component/notification/abc".into(),
            action_id: None,
        });
        cx.run_until_parked();
        assert_eq!(cx.delivered_system_notifications().len(), 1, "ours stays up");
    }
}
