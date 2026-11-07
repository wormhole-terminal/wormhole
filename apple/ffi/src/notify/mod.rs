//! Who hears about a terminal, and how: core's `notify` rules behind one object. Swift reads each terminal
//! (`TabFactsRecord`), asks [`Notifier::observe`], and gets back the banners to post, the ones to take down, the
//! Dock bounce and what VoiceOver says. It never decides anything itself; delivery (the notification centre, the
//! Dock, VoiceOver) is Swift's.
//!
//! Sans-IO: the clock comes in (`now_ms` monotonic, `wall_ms` Unix) and nothing is read or posted from here.

mod records;
mod watch;

use std::collections::HashSet;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, UNIX_EPOCH};

use wormhole_core::config::Settings;
use wormhole_core::notify::{
    self, About, Announcer, Bursts, Delivery, Grouped, Kind, Level, Said, Seen, Text, Throttle,
};

pub use records::*;
use watch::{Event, Failure, Running, Tab, Watch};

/// The banner "Send test" posts.
const TEST_ID: &str = "wormhole/test";

struct Inner {
    watch: Watch,
    bursts: Bursts,
    bells: Throttle,
    /// What programs' own reports (OSC 7501) may ask for.
    reports: Throttle,
    announcer: Announcer,
    /// The ids of the banners that are up, so taking one down asks the system only when there is one.
    up: HashSet<String>,
}

/// The notification rules' memory: what was seen last, the recent burst, the throttles. One per app.
#[derive(uniffi::Object)]
pub struct Notifier {
    inner: Mutex<Inner>,
    /// Where the monotonic clock's zero is, so a millisecond count becomes an `Instant`.
    epoch: Instant,
}

#[uniffi::export]
impl Notifier {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                watch: Watch::default(),
                bursts: Bursts::default(),
                bells: Throttle::new(notify::BELL_RATE),
                reports: Throttle::new(notify::REPORT_RATE),
                announcer: Announcer::default(),
                up: HashSet::new(),
            }),
            epoch: Instant::now(),
        }
    }

    /// Read every terminal of every window and act on what changed since the last look. `settings` are the
    /// look's own, for a terminal that names none; `time_sensitive`: the build is entitled to it and the person
    /// allows it (a banner for needs input is then Time Sensitive).
    pub fn observe(
        &self,
        tabs: Vec<TabFactsRecord>,
        settings: NotifySettingsRecord,
        app_active: bool,
        time_sensitive: bool,
        now_ms: u64,
        wall_ms: u64,
    ) -> Vec<NotifyActionRecord> {
        let now = self.instant(now_ms);
        let wall = UNIX_EPOCH + Duration::from_millis(wall_ms);
        let looks: Vec<Tab> = tabs.iter().map(|tab| self.tab(tab, &settings, app_active)).collect();
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let events = inner.watch.observe(&looks, now, wall);
        let mut out = Vec::new();
        let mut heard: Vec<(i64, &TabFactsRecord, Kind, Option<String>)> = Vec::new();
        for event in events {
            match event {
                Event::Post { tab, kind, took, command } => {
                    let Some(facts) = tabs.iter().find(|facts| facts.tab == tab) else { continue };
                    heard.push((tab, facts, kind, spoken_words(kind, facts)));
                    let own = facts.settings.as_ref().unwrap_or(&settings).to_core();
                    let happened = Happened { kind, facts, took, command: command.as_deref() };
                    inner.announce(&happened, app_active, time_sensitive, &own, now, &mut out);
                }
                Event::Retract(tab) | Event::Seen(tab) => inner.take_down(&notify::notice_id(tab), &mut out),
            }
        }
        // VoiceOver hears every terminal that needs input, finished or failed, the one on screen too: unlike a
        // banner, nothing else would read it out.
        let said: Vec<Said> = heard
            .iter()
            .map(|(tab, facts, kind, words)| Said {
                tab: *tab,
                name: &facts.name,
                kind: *kind,
                question: words.as_deref(),
            })
            .collect();
        for spoken in inner.announcer.announce(now, &said) {
            out.push(NotifyActionRecord::Say {
                text: spoken.text,
                urgent: spoken.priority == notify::Priority::High,
            });
        }
        out
    }

    /// A terminal's banner goes (the person looked at it, or it closed): the retract to carry out, if one is up.
    pub fn dismiss(&self, tab: i64) -> Option<String> {
        let id = notify::notice_id(tab);
        self.lock().up.remove(&id).then_some(id)
    }

    /// What a click on a banner, or one of its buttons, asks of the app, and the banner to take down.
    pub fn respond(&self, response: ResponseRecord) -> ResponseOutcomeRecord {
        let retract = self.lock().up.remove(&response.id).then(|| response.id.clone());
        let kind = match notify::notice_tab(&response.id) {
            None if response.id == notify::SUMMARY_ID || response.id == TEST_ID => {
                ResponseKindRecord::Activate
            }
            None => ResponseKindRecord::Nothing,
            Some(tab) => match response.action.as_deref() {
                Some("reply") => match response.reply {
                    Some(text) => ResponseKindRecord::Reply { tab, text },
                    None => ResponseKindRecord::Nothing,
                },
                Some("show-changes") => ResponseKindRecord::ShowChanges { tab },
                _ => ResponseKindRecord::Open { tab },
            },
        };
        ResponseOutcomeRecord { retract, kind }
    }

    /// "Send test": a banner as "needs input" would be, about `name`.
    pub fn test_notice(
        &self,
        name: String,
        settings: NotifySettingsRecord,
        time_sensitive: bool,
    ) -> NoticeRecord {
        let level = notify::interruption_level(Kind::NeedsYou, time_sensitive);
        let sound = notify::sound(Kind::NeedsYou, level, settings.to_core().notify_sound);
        self.lock().up.insert(TEST_ID.to_string());
        NoticeRecord {
            id: TEST_ID.to_string(),
            thread: TEST_ID.to_string(),
            title: format!("{name} needs input"),
            subtitle: "Wormhole · test".to_string(),
            body: "This is how a terminal that needs input is announced.".to_string(),
            level: level.into(),
            sound,
            category: NoticeCategoryRecord::Plain,
        }
    }

    /// The last command that ended by itself in `tab` and how long it ran (a finished terminal's row says it).
    pub fn finished_command(&self, tab: i64) -> Option<FinishedRecord> {
        let inner = self.lock();
        let (command, took) = inner.watch.finished.get(&tab)?;
        Some(FinishedRecord { command: command.clone(), took_ms: took.as_millis() as u64 })
    }
}

impl Default for Notifier {
    fn default() -> Self {
        Self::new()
    }
}

/// The last command that ended by itself in a terminal.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FinishedRecord {
    pub command: String,
    pub took_ms: u64,
}

/// The Dock icon's badge: terminals that need input, with the finished and failed ones nobody has seen in the
/// mode that counts them. Working never badges; nil clears it.
#[uniffi::export]
pub fn notify_dock_badge(mode: DockBadgeModeRecord, needs_you: u32, results: u32) -> Option<String> {
    notify::dock_badge(mode.into(), needs_you as usize, results as usize)
}

/// The settings the rules start from (`Settings::default()`).
#[uniffi::export]
pub fn notify_default_settings() -> NotifySettingsRecord {
    NotifySettingsRecord::from(&Settings::default())
}

/// The banner's level for a kind (`notify::interruption_level`), for a front end that posts its own.
#[uniffi::export]
pub fn notify_level(kind: NoticeKindRecord, time_sensitive: bool) -> NoticeLevelRecord {
    notify::interruption_level(kind.into(), time_sensitive).into()
}

impl Notifier {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn instant(&self, ms: u64) -> Instant {
        self.epoch + Duration::from_millis(ms)
    }

    fn tab(&self, facts: &TabFactsRecord, look: &NotifySettingsRecord, app_active: bool) -> Tab {
        let settings = facts.settings.as_ref().unwrap_or(look);
        Tab {
            tab: facts.tab,
            state: facts.state.into(),
            since: self.instant(facts.since_ms),
            source: facts.source.into(),
            notice_serial: facts.notice.as_ref().map_or(0, |n| n.serial),
            failure: facts.failure.as_ref().map(|f| Failure {
                serial: f.serial,
                at: self.instant(f.at_ms),
                command: f.command.clone(),
                took: f.took_ms.map(Duration::from_millis),
            }),
            foreground: facts.foreground.as_ref().map(|f| Running {
                command: f.command.clone(),
                started: UNIX_EPOCH + Duration::from_millis(f.started_unix_ms),
            }),
            looking: facts.selected && facts.key && app_active,
            command_after: settings.command_after_secs,
        }
    }
}

/// One event, with what it is about.
struct Happened<'a> {
    kind: Kind,
    facts: &'a TabFactsRecord,
    took: Option<Duration>,
    command: Option<&'a str>,
}

impl Happened<'_> {
    fn text(&self) -> Text {
        let facts = self.facts;
        let command = self.command.or(facts.foreground.as_ref().map(|f| f.command.as_str()));
        // A program's own notification speaks with its words, not the agent's question.
        let notice = facts.notice.as_ref().filter(|_| self.kind == Kind::Notice);
        // A failed command: the exit code, and what the shell's blocks could say it last printed.
        let failure = facts.failure.as_ref().filter(|_| self.kind == Kind::CommandFailed);
        notify::text(
            self.kind,
            &About {
                name: &facts.name,
                who: facts.who.as_deref(),
                command: failure.and_then(|f| f.command.as_deref()).or(command),
                folder: facts.folder.as_deref(),
                branch: facts.branch.as_deref(),
                message: notice.map(|n| n.body.as_str()).or(facts.message.as_deref()),
                title: notice.map(|n| n.title.as_str()),
                took: self.took.or(failure.and_then(|f| f.took_ms).map(Duration::from_millis)),
                exit: failure.map(|f| f.code),
                last_line: failure.and_then(|f| f.last_line.as_deref()),
                changed: facts.changed.map(|n| n as usize),
            },
        )
    }
}

/// What VoiceOver reads of an event: a program's notification in its words, a failed command's exit code, else
/// the agent's question (or what a program that failed said).
fn spoken_words(kind: Kind, facts: &TabFactsRecord) -> Option<String> {
    match kind {
        Kind::Notice => {
            let notice = facts.notice.as_ref()?;
            [&notice.body, &notice.title].into_iter().find(|s| !s.is_empty()).cloned()
        }
        Kind::CommandFailed => facts.failure.as_ref().map(|failure| format!("exit {}", failure.code)),
        _ => facts.message.clone(),
    }
}

/// One clock per terminal and kind: asking, finishing and failing in quick turn are all told, a program repeating
/// any of them is not.
fn report_key(session: i64, kind: Kind) -> i64 {
    let slot = match kind {
        Kind::NeedsYou => 1,
        Kind::AgentFailed => 2,
        _ => 0,
    };
    session.saturating_mul(3).saturating_add(slot)
}

fn category(kind: Kind) -> NoticeCategoryRecord {
    match kind {
        Kind::NeedsYou => NoticeCategoryRecord::NeedsYou,
        Kind::AgentDone => NoticeCategoryRecord::Finished,
        Kind::CommandDone | Kind::CommandFailed | Kind::AgentFailed | Kind::Bell | Kind::Notice => {
            NoticeCategoryRecord::Plain
        }
    }
}

impl Inner {
    fn post(&mut self, notice: NoticeRecord, out: &mut Vec<NotifyActionRecord>) {
        self.up.insert(notice.id.clone());
        out.push(NotifyActionRecord::Post { notice });
    }

    fn take_down(&mut self, id: &str, out: &mut Vec<NotifyActionRecord>) {
        if self.up.remove(id) {
            out.push(NotifyActionRecord::Retract { id: id.to_string() });
        }
    }

    /// Tell the user about one event as the rules say: a banner (maybe as part of a burst's summary) or nothing,
    /// and the Dock's bounce.
    fn announce(
        &mut self,
        happened: &Happened,
        app_active: bool,
        time_sensitive: bool,
        settings: &Settings,
        now: Instant,
        out: &mut Vec<NotifyActionRecord>,
    ) {
        let Happened { kind, facts, .. } = *happened;
        // A program that reports its own state can flip it as fast as it can write: its questions and its
        // finishing are each told at most once in a while, banner, sound and Dock bounce alike.
        if facts.reported
            && matches!(kind, Kind::NeedsYou | Kind::AgentDone | Kind::AgentFailed)
            && !self.reports.allow(report_key(facts.session, kind), now)
        {
            return;
        }
        if notify::bounces(kind, app_active, settings.dock_bounce) {
            out.push(NotifyActionRecord::Bounce);
        }
        if kind == Kind::Bell && !self.bells.allow(facts.session, now) {
            return;
        }
        let seen = Seen { looking: facts.selected && facts.key && app_active, app_active };
        match notify::delivery(kind, seen, settings) {
            Delivery::Nothing => {}
            Delivery::Banner => self.banner(happened, time_sensitive, settings, now, out),
        }
    }

    fn banner(
        &mut self,
        happened: &Happened,
        time_sensitive: bool,
        settings: &Settings,
        now: Instant,
        out: &mut Vec<NotifyActionRecord>,
    ) {
        let Happened { kind, facts, .. } = *happened;
        let text = happened.text();
        let level = notify::interruption_level(kind, time_sensitive);
        let single = NoticeRecord {
            id: notify::notice_id(facts.tab),
            thread: notify::thread_id(facts.session),
            title: text.title,
            subtitle: text.subtitle,
            body: text.body,
            level: level.into(),
            sound: notify::sound(kind, level, settings.notify_sound),
            category: category(kind),
        };
        // What needs input is never folded into a summary: it is the one worth a banner of its own.
        if !settings.notify_group_bursts || kind == Kind::NeedsYou {
            self.post(single, out);
            return;
        }
        match self.bursts.add(now, facts.tab, &facts.name, kind) {
            Grouped::Single => self.post(single, out),
            Grouped::Summary { text, retract } => {
                for tab in retract {
                    self.take_down(&notify::notice_id(tab), out);
                }
                self.post(
                    NoticeRecord {
                        id: notify::SUMMARY_ID.to_string(),
                        thread: notify::SUMMARY_ID.to_string(),
                        title: text.title,
                        subtitle: text.subtitle,
                        body: text.body,
                        level: Level::Active.into(),
                        sound: notify::sound(kind, Level::Active, settings.notify_sound),
                        category: NoticeCategoryRecord::Plain,
                    },
                    out,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests;
