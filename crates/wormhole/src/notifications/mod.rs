//! Notifications: native banners for what happens in a terminal you aren't looking at
//! (while Wormhole is in front only if asked for: nothing shows inside the window), the Dock's
//! bounce, and what a click, Reply or Show Changes does.
//!
//! Above the window and the shared state, which it reads; the window calls `dismiss` and `sweep` (a
//! known edge, kept until the window emits seen and closed events instead).
//!
//! The rules are core's (`wormhole_core::notify`). Every tick (and right after an agent's
//! hook) [`sweep`] reads every tab of every window, `watch` turns what changed into events, and
//! the rules decide. Banners go out by one of these routes:
//! - the bundled app: the system's notification centre (`platform::notifications`), with
//!   interruption levels (Time Sensitive for "needs you" when the build is entitled to it), a
//!   thread per terminal, sound, and the Open / Reply… / Show Changes actions;
//! - a dev build (`cargo run`) has no bundle, and the notification center would abort the
//!   process: the platform's script instead, without actions or levels;
//! - tests, and a system without notifications of its own: GPUI's. devctl builds can ask for
//!   `WORMHOLE_NOTIFY=log`, which keeps them in a list instead of posting anything; a test
//!   instance in background mode does that unless `WORMHOLE_NOTIFY` says otherwise.
//!
//! One notification per tab: a newer one replaces the older, and looking at the tab (or the
//! agent moving on) takes it down.
//!
//! By file: `route` which way a run posts, `announce` what an event says and where it goes, `respond`
//! what a click does and taking one down, `permission` macOS's say, `watch` what changed; this file
//! holds the shared state, the tick and the posting.

mod watch;

mod announce;
mod permission;
mod respond;
mod route;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::time::{Instant, SystemTime};

use gpui_kit::*;
use wormhole_core::notify::{self, Announcer, Bursts, Kind, Priority, Said, Throttle};

use crate::app_state::attention::{self, Attention};
use crate::app_state::{AppState, app};
use crate::platform::notifications::{
    self as system, Button, ButtonKind, Category, Center, FromCenter, Notice, Permission, Response,
};
use crate::settings::{settings, settings_in};
use crate::window::ambient;
#[cfg(feature = "devctl")]
use crate::window::ambient::TabFacts;
#[cfg(feature = "devctl")]
use announce::category;
use announce::{Happened, announce, spoken_words};
use watch::{Event, Watch};
#[cfg(feature = "devctl")]
use wormhole_core::notify::Seen;

#[cfg(test)]
pub(crate) use permission::permission_refreshes;
pub(crate) use permission::{
    ask_permission, ask_permission_if_new, open_system_settings, permission, refresh_permission,
};
#[cfg(any(test, feature = "devctl"))]
pub(crate) use permission::{permission_asks, pretend_permission};
#[cfg(test)]
pub(crate) use respond::tag;
pub(crate) use respond::{dismiss, reply, respond};
use route::{Route, route, settled};

/// The actions a notification can carry (`Notice::category` picks which).
pub(crate) const OPEN: &str = "open";
pub(crate) const REPLY: &str = "reply";
pub(crate) const SHOW_CHANGES: &str = "show-changes";
/// The banner "Send test" posts.
const TEST_ID: &str = "wormhole/test";

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
    /// How many times the permission was read again (the tick does while Settings is open).
    #[cfg(test)]
    refreshes: u32,
    center: Option<Center>,
}

impl Global for Notifier {}

fn notifier(cx: &mut App) -> &mut Notifier {
    cx.default_global::<Notifier>()
}

/// Set up the route and the handling of clicks and replies. Call once, at start.
pub(crate) fn init(cx: &mut App) {
    // What the windows tell: a terminal that is in front or gone takes its notification down, and a
    // status that changed is looked at now (after the update that told, which a window may be in).
    attention::listen(cx, |event, cx| match event {
        Attention::Seen(tab) => dismiss(tab, cx),
        Attention::Changed => cx.defer(sweep),
    });
    let (tx, rx) = smol::channel::unbounded();
    let route = route();
    let center = (route == Route::Center).then(|| Center::open(tx.clone(), actions)).flatten();
    let route = settled(route, center.is_some());
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
        // notification with system delivery. The bundled app uses its own delegate
        // (`platform::notifications::Center`).
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
    if ticks.is_multiple_of(5) && crate::app_state::live(&app(cx).settings_window).is_some() {
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
                        Some(owner) => settings_in(&owner.profile, cx).clone(),
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

/// The last command that ended by itself in each tab, and how long it ran.
pub(crate) fn finished_commands(cx: &App) -> Option<&HashMap<i64, (String, std::time::Duration)>> {
    cx.try_global::<Notifier>().map(|notifier| &notifier.watch.finished)
}

/// "Send test": a real banner, as "needs you" would be, about the terminal in front.
pub(crate) fn send_test(cx: &mut App) {
    let name = AppState::front_window(cx)
        .and_then(|front| front.workspace.read_with(cx, |ws, cx| ws.active_session_name(cx)).ok())
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
