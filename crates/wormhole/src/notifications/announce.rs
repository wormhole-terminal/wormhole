//! Telling the user about one event as the rules say: its words, the banner, the Dock's bounce and what
//! VoiceOver reads.

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::config::Settings;
use wormhole_core::notify::{self, About, Grouped, Kind, Level, Seen, Text, Throttle};

use super::respond::dismiss;
use super::{notifier, post, time_sensitive};
use crate::platform::notifications::{Category, Notice};
use crate::window::ambient::TabFacts;

/// One event, with what it is about.
pub(super) struct Happened<'a> {
    pub(super) kind: Kind,
    pub(super) facts: &'a TabFacts,
    pub(super) took: Option<std::time::Duration>,
    pub(super) command: Option<&'a str>,
}

impl Happened<'_> {
    pub(super) fn text(&self) -> Text {
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
pub(super) fn spoken_words(kind: Kind, facts: &TabFacts) -> Option<String> {
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
pub(super) fn announce(happened: Happened, app_active: bool, settings: &Settings, cx: &mut App) {
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

pub(super) fn bell_rate() -> Throttle {
    Throttle::new(notify::BELL_RATE)
}

pub(super) fn report_rate() -> Throttle {
    Throttle::new(notify::REPORT_RATE)
}

/// One clock per terminal and kind: asking, finishing and failing in quick turn are all told, a
/// program repeating any of them is not.
pub(super) fn report_key(session: i64, kind: Kind) -> i64 {
    let slot = match kind {
        Kind::NeedsYou => 1,
        Kind::AgentFailed => 2,
        _ => 0,
    };
    session.saturating_mul(3).saturating_add(slot)
}

pub(super) fn banner(happened: &Happened, settings: &Settings, cx: &mut App) {
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

pub(super) fn category(kind: Kind) -> Category {
    match kind {
        Kind::NeedsYou => Category::NeedsYou,
        Kind::AgentDone => Category::Finished,
        Kind::CommandDone | Kind::CommandFailed | Kind::AgentFailed | Kind::Bell | Kind::Notice => {
            Category::Plain
        }
    }
}
