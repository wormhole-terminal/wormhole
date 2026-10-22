//! Notifications: native banners for what happens in a terminal you aren't looking at
//! (while Wormhole is in front only if asked for: nothing shows inside the window), the Dock's
//! bounce, and what a click, Reply or Show Changes does.
//!
//! The rules are core's (`wormhole_core::notify`). Every tick (and right after an agent's
//! hook) [`sweep`] reads every tab of every window, `watch` turns what changed into events, and
//! the rules decide. Banners go out by one of these routes:
//! - the bundled app: the system's notification centre (`platform::notifications`), with interruption levels (Time
//!   Sensitive for "needs you" when the build is entitled to it), a thread per terminal, sound,
//!   and the Open / Reply… / Show Changes actions;
//! - a dev build (`cargo run`) has no bundle, and the notification center would abort the
//!   process: the platform's script instead, without actions or levels;
//! - tests, and a system without notifications of its own: GPUI's. devctl builds can ask for `WORMHOLE_NOTIFY=log`, which keeps
//!   them in a list instead of posting anything; a test instance in background mode does that
//!   unless `WORMHOLE_NOTIFY` says otherwise.
//!
//! One notification per tab: a newer one replaces the older, and looking at the tab (or the
//! agent moving on) takes it down.

mod watch;

use std::collections::{HashMap, HashSet};
use std::time::{Instant, SystemTime};

use wormhole_core::config::Settings;
use wormhole_core::notify::{
    self, About, Announcer, Bursts, Grouped, Kind, Level, Priority, Said, Seen, Text, Throttle,
};
use wormhole_core::update::current_app_bundle;

use super::ambient::{self, TabFacts};
use super::app::{AppState, app, open_tab};
use super::*;
use crate::platform::notifications::{
    self as system, Button, ButtonKind, Category, Center, FromCenter, Notice, Permission, Response,
};
use watch::{Event, Watch};

/// The actions a notification can carry (`Notice::category` picks which).
pub(crate) const OPEN: &str = "open";
pub(crate) const REPLY: &str = "reply";
pub(crate) const SHOW_CHANGES: &str = "show-changes";
/// The banner "Send test" posts.
const TEST_ID: &str = "wormhole/test";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Route {
    /// The bundled app: the system's notification centre.
    Center,
    /// No bundle: the platform's script.
    Script,
    /// GPUI's notifications: tests, and a system without notifications of its own.
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
        _ if !system::available() => Route::Gpui,
        _ if current_app_bundle().is_some() => Route::Center,
        _ => Route::Script,
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
    /// What programs' own reports (OSC 7501) may ask for: see [`report_key`].
    reports: Option<Throttle>,
    /// What VoiceOver was told lately, so a burst is said as a count.
    announcer: Announcer,
    /// The ids of the notifications that are up, so taking one down asks the system only when
    /// there is something to take down (the tick checks the tab in front every 400 ms).
    up: HashSet<String>,
    permission: Option<Permission>,
    /// What the `Log` route would have posted (devctl reads it).
    log: Vec<Notice>,
    /// How many times macOS was asked for permission (a test or test instance has no macOS to ask:
    /// the question counts as answered yes).
    #[cfg(any(test, feature = "devctl"))]
    asks: u32,
    center: Option<Center>,
}

impl Global for Notifier {}

fn notifier(cx: &mut App) -> &mut Notifier {
    cx.default_global::<Notifier>()
}

/// Set up the route and the handling of clicks and replies. Call once, at start.
pub(crate) fn init(cx: &mut App) {
    let (tx, rx) = smol::channel::unbounded();
    let route = route();
    let center = (route == Route::Center).then(|| Center::open(tx.clone(), actions)).flatten();
    let route = if route == Route::Center && center.is_none() { Route::Script } else { route };
    let permission = match route {
        Route::Center => Permission::Checking,
        _ => Permission::OutsideBundle,
    };
    cx.set_global(Notifier {
        route,
        bells: Some(Throttle::new(notify::BELL_RATE)),
        reports: Some(Throttle::new(notify::REPORT_RATE)),
        permission: Some(permission),
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
    let app_active = crate::platform::dock::app_is_active(cx);
    let after = settings(cx).notify_command_after;
    let events = notifier(cx).watch.observe(&tabs, app_active, after, SystemTime::now());
    if events.is_empty() {
        return;
    }
    let mut heard = Vec::new();
    for event in events {
        match event {
            Event::Post { tab, kind, took, command } => {
                if let Some(facts) = tabs.iter().find(|facts| facts.tab == tab) {
                    heard.push((tab, facts.name.as_str(), kind, spoken_words(kind, facts)));
                    let happened = Happened { kind, facts, took, command: command.as_deref() };
                    // By the settings of the profile the terminal is in, shown or not.
                    let owner = AppState::find_window(cx, |ws, _| ws.find_session(tab).is_some());
                    let settings = match owner {
                        Some(owner) => app(cx).settings_of(&owner.profile, cx),
                        None => settings(cx).clone(),
                    };
                    announce(happened, app_active, &settings, cx);
                }
            }
            Event::Retract(tab) | Event::Seen(tab) => dismiss(tab, cx),
        }
    }
    // VoiceOver hears every terminal that needs you, finished or failed, the one on screen too:
    // unlike a banner, nothing else would read it out.
    let said: Vec<Said> = heard
        .iter()
        .map(|(tab, name, kind, words)| Said { tab: *tab, name, kind: *kind, question: words.as_deref() })
        .collect();
    for spoken in notifier(cx).announcer.announce(Instant::now(), &said) {
        crate::platform::a11y::announce(&spoken.text, spoken.priority == Priority::High);
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
        // A failed command: the exit code, and what the shell's blocks could say it last printed.
        let failure = facts.failure.as_ref().filter(|_| self.kind == Kind::CommandFailed);
        notify::text(
            self.kind,
            &About {
                name: &facts.name,
                who: facts.who(),
                command: failure.and_then(|f| f.command.as_deref()).or(command),
                folder: facts.folder.as_deref(),
                branch: facts.branch.as_deref(),
                message: notice.map(|n| n.body.as_str()).or(facts.message.as_deref()),
                title: notice.map(|n| n.title.as_str()),
                took: self.took.or(failure.and_then(|f| f.took)),
                exit: failure.map(|f| f.code),
                last_line: failure.and_then(|f| f.last_line.as_deref()),
                changed: facts.changed,
            },
        )
    }
}

/// What VoiceOver reads of an event: a program's notification in its words, a failed command's
/// exit code, else the agent's question (or what a program that failed said).
fn spoken_words(kind: Kind, facts: &TabFacts) -> Option<String> {
    match kind {
        Kind::Notice => {
            let notice = facts.notice.as_ref()?;
            [&notice.notification.body, &notice.notification.title]
                .into_iter()
                .find(|s| !s.is_empty())
                .cloned()
        }
        Kind::CommandFailed => facts.failure.as_ref().map(|failure| format!("exit {}", failure.code)),
        _ => facts.message.clone(),
    }
}

/// Tell the user about one event as the rules say: a banner (maybe as part of a burst's
/// summary) or nothing, and the Dock's bounce.
fn announce(happened: Happened, app_active: bool, settings: &Settings, cx: &mut App) {
    let Happened { kind, facts, .. } = happened;
    // A program that reports its own state can flip it as fast as it can write: its questions and
    // its finishing are each told at most once in a while, banner, sound and Dock bounce alike.
    if facts.reported
        && matches!(kind, Kind::NeedsYou | Kind::AgentDone | Kind::AgentFailed)
        && !notifier(cx)
            .reports
            .get_or_insert_with(report_rate)
            .allow(report_key(facts.session, kind), Instant::now())
    {
        return;
    }
    if notify::bounces(kind, app_active, settings.dock_bounce) {
        crate::platform::dock::bounce(cx);
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

fn report_rate() -> Throttle {
    Throttle::new(notify::REPORT_RATE)
}

/// One clock per terminal and kind: asking, finishing and failing in quick turn are all told, a
/// program repeating any of them is not.
fn report_key(session: i64, kind: Kind) -> i64 {
    let slot = match kind {
        Kind::NeedsYou => 1,
        Kind::AgentFailed => 2,
        _ => 0,
    };
    session.saturating_mul(3).saturating_add(slot)
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
        Kind::CommandDone | Kind::CommandFailed | Kind::AgentFailed | Kind::Bell | Kind::Notice => {
            Category::Plain
        }
    }
}

/// Whether "needs you" goes out as Time Sensitive: only from a bundle signed with the entitlement
/// (a Developer ID build with its provisioning profile); every other build uses Active.
pub(crate) fn time_sensitive(cx: &mut App) -> bool {
    notifier(cx).route == Route::Center && system::entitled_to_time_sensitive()
}

fn post(notice: Notice, cx: &mut App) {
    let notifier = notifier(cx);
    notifier.up.insert(notice.id.clone());
    match notifier.route {
        Route::Center => {
            if let Some(center) = &notifier.center {
                center.post(&notice);
            }
        }
        Route::Script => {
            if let Err(err) = system::post_by_script(&notice) {
                tracing::warn!("could not show a notification: {err}");
            }
        }
        Route::Gpui => {
            let actions = actions(notice.category)
                .iter()
                .map(|button| SystemNotificationAction { id: button.id.into(), label: button.label.into() })
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

/// A category's buttons. Open and Show Changes bring the app forward; typing a reply needs no
/// window, as the text goes to the terminal as it is.
pub(crate) fn actions(category: Category) -> &'static [Button] {
    const OPEN_BUTTON: Button = Button { id: OPEN, label: "Open", kind: ButtonKind::Foreground };
    match category {
        Category::NeedsYou => &[OPEN_BUTTON, Button { id: REPLY, label: "Reply…", kind: ButtonKind::Reply }],
        Category::Finished => {
            &[OPEN_BUTTON, Button { id: SHOW_CHANGES, label: "Show Changes", kind: ButtonKind::Foreground }]
        }
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
    if let Some(center) = &notifier(cx).center {
        center.refresh_permission();
    }
}

/// "Allow notifications": macOS asks the user (once; after that only System Settings can).
pub(crate) fn ask_permission(cx: &mut App) {
    #[cfg(any(test, feature = "devctl"))]
    {
        let notifier = notifier(cx);
        notifier.asks += 1;
        // No macOS to answer in a test or a test instance, so the answer is yes. (The bundled app
        // never gets here without a center: the route is `Center`, or the permission is not asked.)
        if matches!(notifier.route, Route::Gpui | Route::Log) {
            notifier.permission = Some(Permission::Allowed { time_sensitive: false });
        }
    }
    if let Some(center) = &notifier(cx).center {
        center.ask();
    }
}

/// The welcome's presses that turn a banner on or say Allow: macOS is asked, but only when it has not
/// been asked yet (after that its answer stands, and only System Settings can change it). Returns
/// whether it was asked.
pub(crate) fn ask_permission_if_new(cx: &mut App) -> bool {
    if permission(cx) != Permission::NotAsked {
        return false;
    }
    ask_permission(cx);
    true
}

/// How many times macOS was asked in this run.
#[cfg(any(test, feature = "devctl"))]
pub(crate) fn permission_asks(cx: &App) -> u32 {
    cx.try_global::<Notifier>().map_or(0, |notifier| notifier.asks)
}

/// A test or a test instance pretends macOS has said `permission`.
#[cfg(any(test, feature = "devctl"))]
pub(crate) fn pretend_permission(permission: Permission, cx: &mut App) {
    notifier(cx).permission = Some(permission);
    cx.refresh_windows();
}

/// "Open System Settings": Wormhole's page under Notifications.
pub(crate) fn open_system_settings(cx: &App) {
    system::open_settings(cx);
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
    let app_active = crate::platform::dock::app_is_active(cx);
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
        actions(category(kind)).iter().map(|button| button.label).collect::<Vec<_>>(),
    );
    if post_it {
        announce(happened, app_active, &settings, cx);
    }
    line
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use gpui_kit::{SystemNotificationResponse, TestAppContext};
    use wormhole_core::notify::{Kind, Level, Source, State};

    use super::{Category, Happened, Notice, category, dismiss, init, logs, post, spoken_words, tag};
    use crate::terminal_view::Failure;
    use crate::workspace::ambient::TabFacts;

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
        let happened = Happened {
            kind: Kind::AgentDone,
            facts: &brew,
            took: Some(Duration::from_secs(252)),
            command: None,
        };
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
